// src/storage/repository/entities.rs

use crate::domain::entity::EntityProfile;
use crate::domain::taxonomy::{IndustryRecord, IndustryRegistry};
use crate::ui::state::{CompanyFilingSummary, GeoEntityMarker};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row, Transaction};
use uuid::Uuid;

/// Fetches company filings summary for the Form Viewer selector
pub async fn fetch_company_filings(
    pool: &PgPool,
) -> Result<Vec<CompanyFilingSummary>, sqlx::Error> {
    let sql = r#"
        SELECT 
            entity_id, corporate_id_number, company_name, 
            address_registered_office, address_corporate_office,
            company_email, company_telephone, website, paid_up_shares_value,
            financial_year_start, financial_year_end
        FROM entity_list
        ORDER BY company_name ASC, financial_year_start DESC NULLS LAST;
    "#;

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    let mut items = Vec::with_capacity(rows.len());

    for r in rows {
        items.push(CompanyFilingSummary {
            entity_id: r.get("entity_id"),
            corporate_id_number: r.get("corporate_id_number"),
            company_name: r.get("company_name"),
            address_registered_office: r.get("address_registered_office"),
            address_corporate_office: r.get("address_corporate_office"),
            company_email: r.get("company_email"),
            company_telephone: r.get("company_telephone"),
            website: r.get("website"),
            paid_up_shares_value: r.get("paid_up_shares_value"),
            fy_start: r.get("financial_year_start"),
            fy_end: r.get("financial_year_end"),
        });
    }

    Ok(items)
}

/// Fetches geolocated corporate entities for spatial map plotting
pub async fn fetch_geo_entities(pool: &PgPool) -> Result<Vec<GeoEntityMarker>, sqlx::Error> {
    let sql = r#"
        SELECT 
            entity_id, company_name, corporate_id_number,
            COALESCE(industry_sector, 'Unclassified') as industry_sector,
            latitude, longitude,
            COALESCE(resolved_state, 'Unknown State') as state_name,
            COALESCE(resolved_city, 'Unknown City') as city_name
        FROM entity_list
        WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
    "#;

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    let mut markers = Vec::with_capacity(rows.len());

    for r in rows {
        markers.push(GeoEntityMarker {
            entity_id: r.get("entity_id"),
            company_name: r.get("company_name"),
            corporate_id_number: r.get("corporate_id_number"),
            industry_sector: r.get("industry_sector"),
            latitude: r.get("latitude"),
            longitude: r.get("longitude"),
            state_name: r.get("state_name"),
            city_name: r.get("city_name"),
            scope_1_tco2e: None,
            water_withdrawal_kl: None,
        });
    }

    Ok(markers)
}

/// Inserts corporate profile, preserving ISIN/NSE Symbol/BSE Scrip Code and safely handling
/// overlapping FY records across exchanges/amendments.
pub async fn insert_entity_profile(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    file_id: Uuid,
    stock_market: &str,
    profile: &EntityProfile,
    fallback_industry: Option<&IndustryRecord>,
) -> Result<Uuid, sqlx::Error> {
    let paid_up_decimal = profile
        .paid_up_shares_value
        .filter(|v| v.is_finite() && v.abs() < 1e16)
        .and_then(|v| Decimal::try_from(v).ok())
        .map(|d| d.round_dp(4));

    let (fy_start, fy_end) = match (profile.financial_year_start, profile.financial_year_end) {
        (Some(s), Some(e)) if s > e => (Some(e), Some(s)),
        pair => pair,
    };

    let boundary = profile
        .reporting_boundary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Standalone");

    // Clear any superseded overlapping filing for the same CIN, boundary, and FY window
    if let (Some(s), Some(e)) = (fy_start, fy_end) {
        sqlx::query(
            r#"
            DELETE FROM entity_list
            WHERE corporate_id_number = $1
              AND COALESCE(reporting_boundary, 'Standalone') = $2
              AND financial_year_start IS NOT NULL
              AND financial_year_end IS NOT NULL
              AND financial_year_start <= financial_year_end
              AND daterange(financial_year_start, financial_year_end, '[]') && daterange($3, $4, '[]');
            "#,
        )
        .bind(&profile.corporate_id_number)
        .bind(boundary)
        .bind(s)
        .bind(e)
        .execute(&mut **tx)
        .await?;
    }

    let row = sqlx::query(
        r#"
        INSERT INTO entity_list (
            section, principle_number, sub_section, friendly_group, stock_market,
            file_uuid, corporate_id_number, isin, nse_symbol, bse_scrip_code,
            company_name, address_corporate_office, address_registered_office,
            contact_person_name, contact_person_phone, contact_person_email,
            company_email, company_telephone, website,
            reporting_boundary, financial_year_start, financial_year_end,
            incorporation_date, paid_up_shares_value
        )
        VALUES (
            'A', 0, 1, 'entity_list', $1,
            $2, $3, $4, $5, $6,
            $7, $8, $9,
            $10, $11, $12,
            $13, $14, $15,
            $16, $17, $18,
            $19, $20
        )
        RETURNING entity_id, industry_sector;
        "#,
    )
    .bind(stock_market)
    .bind(file_id)
    .bind(&profile.corporate_id_number)
    .bind(&profile.isin)
    .bind(&profile.nse_symbol)
    .bind(&profile.bse_scrip_code)
    .bind(&profile.company_name)
    .bind(&profile.address_corporate_office)
    .bind(&profile.address_registered_office)
    .bind(&profile.contact_person_name)
    .bind(&profile.contact_person_phone)
    .bind(&profile.contact_person_email)
    .bind(&profile.company_email)
    .bind(&profile.company_telephone)
    .bind(&profile.website)
    .bind(boundary)
    .bind(fy_start)
    .bind(fy_end)
    .bind(profile.incorporation_date)
    .bind(paid_up_decimal)
    .fetch_one(&mut **tx)
    .await?;

    let entity_id: Uuid = row.get("entity_id");
    let db_sector: Option<String> = row.get("industry_sector");

    // Safety fallback if the database still uses the older trigger without NIC lookup
    if matches!(db_sector.as_deref(), None | Some("Unclassified"))
        && let Some(fb) = fallback_industry
        && fb.sector != "Unclassified"
    {
        let _ = sqlx::query(
            r#"
            UPDATE entity_list
            SET industry_sector = $2,
                industry_division = $3,
                industry_source = 'RUST_NIC_FALLBACK'
            WHERE entity_id = $1;
            "#,
        )
        .bind(entity_id)
        .bind(&fb.sector)
        .bind(&fb.division)
        .execute(&mut **tx)
        .await;
    }

    Ok(entity_id)
}

/// Instant offline re-resolution of unmapped entities using the local geo_pincode table
pub async fn resolve_unmapped_pincodes_bulk(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        r#"
        UPDATE entity_list
        SET address_registered_office = address_registered_office
        WHERE (latitude IS NULL OR longitude IS NULL)
          AND (address_registered_office IS NOT NULL OR address_corporate_office IS NOT NULL);
        "#,
    )
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

/// Reclassifies any existing Unclassified entities in entity_list using
/// exchange_industry_master, industry_master (5-digit / 2-digit NIC), and statutory keywords.
pub async fn reclassify_unclassified_entities(pool: &PgPool) -> Result<u64, sqlx::Error> {
    // Backfill NSE symbol / BSE scrip from file_list if missing
    let _ = sqlx::query(
        r#"
        UPDATE entity_list e
        SET nse_symbol = COALESCE(e.nse_symbol, fl.symbol),
            bse_scrip_code = COALESCE(e.bse_scrip_code, fl.scrip_code)
        FROM file_list fl
        WHERE e.file_uuid = fl.file_id
          AND ((e.nse_symbol IS NULL AND fl.symbol IS NOT NULL)
            OR (e.bse_scrip_code IS NULL AND fl.scrip_code IS NOT NULL));
        "#,
    )
    .execute(pool)
    .await;

    let mut updated: u64 = 0;

    if let Ok(res) = sqlx::query(
        r#"
        UPDATE entity_list e
        SET industry_sector = ex.sector,
            industry_division = ex.industry,
            industry_source = 'EXCHANGE_BACKFILL'
        FROM exchange_industry_master ex
        WHERE (e.industry_source IS NULL OR e.industry_source = 'UNCLASSIFIED' OR e.industry_sector = 'Unclassified')
          AND (
              (e.isin IS NOT NULL AND ex.isin = UPPER(TRIM(e.isin)))
              OR (e.nse_symbol IS NOT NULL AND UPPER(ex.symbol) = UPPER(TRIM(e.nse_symbol)))
              OR (e.bse_scrip_code IS NOT NULL AND ex.scrip_code = TRIM(e.bse_scrip_code))
              OR (ex.cin = UPPER(TRIM(e.corporate_id_number)))
          );
        "#,
    )
    .execute(pool)
    .await
    {
        updated += res.rows_affected();
    }

    if let Ok(res) = sqlx::query(
        r#"
        UPDATE entity_list e
        SET industry_sector = COALESCE(im5.esg_sector, im2.esg_sector),
            industry_division = COALESCE(im5.division_name, im2.division_name),
            industry_source = CASE WHEN im5.nic_code IS NOT NULL THEN 'NIC_5DIGIT' ELSE 'NIC_2DIGIT' END
        FROM (
            SELECT entity_id,
                   substring(UPPER(TRIM(corporate_id_number)) FROM '^[LU]([0-9]{5})[A-Z]{2}[0-9]{4}[A-Z]{3}[0-9]{6}$') AS nic5
            FROM entity_list
            WHERE industry_source IS NULL
               OR industry_source = 'UNCLASSIFIED'
               OR industry_sector = 'Unclassified'
        ) src
        LEFT JOIN industry_master im5 ON im5.nic_code = src.nic5
        LEFT JOIN industry_master im2 ON im2.division_code = LEFT(src.nic5, 2) AND LENGTH(im2.nic_code) = 2
        WHERE e.entity_id = src.entity_id
          AND COALESCE(im5.esg_sector, im2.esg_sector) IS NOT NULL;
        "#,
    )
    .execute(pool)
    .await
    {
        updated += res.rows_affected();
    }

    let remaining = sqlx::query(
        r#"
        SELECT entity_id, corporate_id_number, company_name
        FROM entity_list
        WHERE industry_source IS NULL
           OR industry_source = 'UNCLASSIFIED'
           OR industry_sector = 'Unclassified';
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !remaining.is_empty()
        && let Ok(registry) = IndustryRegistry::load_from_db(pool).await
    {
        for r in remaining {
            let eid: Uuid = r.get("entity_id");
            let cin: String = r.get("corporate_id_number");
            let name: String = r.get("company_name");
            let rec = registry.classify_cin_or_name(&cin, &name);
            if rec.sector != "Unclassified" {
                let res = sqlx::query(
                    r#"
                    UPDATE entity_list
                    SET industry_sector = $2,
                        industry_division = $3,
                        industry_source = 'STATUTORY_KEYWORD'
                    WHERE entity_id = $1;
                    "#,
                )
                .bind(eid)
                .bind(&rec.sector)
                .bind(&rec.division)
                .execute(pool)
                .await;
                if let Ok(done) = res {
                    updated += done.rows_affected();
                }
            }
        }
    }

    Ok(updated)
}
