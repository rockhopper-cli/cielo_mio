-- create views - Description: Analytical Views for Company ESG KPIs and Sector Medians
CREATE OR REPLACE VIEW view_company_esg_kpis AS
WITH classified_facts AS (
    SELECT
        f.client_uuid,
        f.standard_indicator_capmkt AS tag,
        f.standard_context_reference AS ctx,
        COALESCE(f.standard_numeric_value, f.numeric_value::numeric) AS val,
        -- Strict CY (3) > Axis CY (2) > PY/PPY (1) priority
        CASE
            WHEN f.standard_context_reference ~* '(^[di]_?p?py([0-9]+|_|main|$)|_p?py([0-9]+|_|$)|previous|prior)' THEN 1
            WHEN f.standard_context_reference ~* '(^[di]_?cy([0-9]+|_|main|$)|_cy([0-9]+|_|$)|current)' THEN 3
            ELSE 2
        END AS ctx_priority
    FROM brsr_facts f
    WHERE COALESCE(f.standard_numeric_value, f.numeric_value::numeric) IS NOT NULL
      AND f.standard_indicator_capmkt IN (
          'TotalScope1Emissions',
          'TotalScope2Emissions',
          'TotalScope3Emissions',
          'TotalVolumeOfWaterWithdrawal',
          'EnergyIntensityPerRupeeOfTurnover',
          'WasteIntensityPerRupeeOfTurnover',
          'TotalEnergyConsumedFromRenewableSources',
          'TotalEnergyConsumedFromRenewableAndNonRenewableSources',
          'PercentageOfFemaleBoardOfDirectors',
          'NumberOfFemaleBoardOfDirectors',
          'TotalNumberOfBoardOfDirectors',
          'PercentageOfGrossWagesPaidToFemaleToTotalWagesPaid',
          'GrossWagesPaidToFemale',
          'TotalWagesPaid',
          'TurnoverRate',
          'PercentageOfEmployeesOrWorkersIncludingDifferentlyAbled',
          'NumberOfEmployeesOrWorkersIncludingDifferentlyAbled',
          'ComplaintsOnPOSHUpHeld',
          'TotalComplaintsReportedUnderSexualHarassmentOfWomenAtWorkplace',
          'PercentageOfCostIncurredOnWellBeingMeasuresWithRespectToTotalRevenueOfTheCompany',
          'AmountOfCostIncurredOnWellBeingMeasures',
          'TotalRevenueOfTheCompany',
          'TotalRecordableWorkRelatedInjuries',
          'NumberOfFatalities',
          'NumberOfDirectorsAgainstWhomDisciplinaryActionWasTaken',
          'NumberOfKMPsAgainstWhomDisciplinaryActionWasTaken',
          'NumberOfEmployeesAgainstWhomDisciplinaryActionWasTaken',
          'NumberOfWorkersAgainstWhomDisciplinaryActionWasTaken',
          'NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear',
          'ConsumerComplaintsReceivedDuringTheYear',
          'NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear',
          'ConsumerComplaintsPendingResolutionAtEndOfYear'
      )
),
scalar_facts AS (
    SELECT
        client_uuid,
        CASE
            WHEN tag = 'TotalScope1Emissions' THEN 'scope_1_tco2e'
            WHEN tag = 'TotalScope2Emissions' THEN 'scope_2_tco2e'
            WHEN tag = 'TotalScope3Emissions' THEN 'scope_3_tco2e'
            WHEN tag = 'TotalVolumeOfWaterWithdrawal' THEN 'water_withdrawal_kl'
            WHEN tag = 'EnergyIntensityPerRupeeOfTurnover' THEN 'energy_intensity'
            WHEN tag = 'WasteIntensityPerRupeeOfTurnover' THEN 'waste_intensity'
            WHEN tag = 'TotalEnergyConsumedFromRenewableSources' THEN '_energy_renew'
            WHEN tag = 'TotalEnergyConsumedFromRenewableAndNonRenewableSources' THEN '_energy_total'
            WHEN tag = 'PercentageOfFemaleBoardOfDirectors' THEN 'female_board_directors_pct'
            WHEN tag = 'NumberOfFemaleBoardOfDirectors' THEN '_female_bod'
            WHEN tag = 'TotalNumberOfBoardOfDirectors' THEN '_total_bod'
            WHEN tag = 'PercentageOfGrossWagesPaidToFemaleToTotalWagesPaid' THEN 'gross_wages_to_female_pct'
            WHEN tag = 'GrossWagesPaidToFemale' THEN '_wages_female'
            WHEN tag = 'TotalWagesPaid' THEN '_wages_total'
            WHEN tag = 'TurnoverRate' AND ctx = 'D_Gender_PermanentEmployees_TableB_TurnOverRate_CY' THEN 'employee_turnover_pct'
            WHEN tag = 'PercentageOfEmployeesOrWorkersIncludingDifferentlyAbled' AND ctx = 'D_Female_Employees_TableA' THEN 'female_workforce_pct'
            WHEN tag = 'NumberOfEmployeesOrWorkersIncludingDifferentlyAbled' AND ctx = 'D_Female_Employees_TableA' THEN '_female_emp'
            WHEN tag = 'NumberOfEmployeesOrWorkersIncludingDifferentlyAbled' AND ctx = 'D_Gender_Employees_TableA' THEN '_total_emp'
            WHEN tag = 'ComplaintsOnPOSHUpHeld' THEN 'posh_complaints_upheld'
            WHEN tag = 'TotalComplaintsReportedUnderSexualHarassmentOfWomenAtWorkplace' THEN 'total_posh_complaints'
            WHEN tag = 'PercentageOfCostIncurredOnWellBeingMeasuresWithRespectToTotalRevenueOfTheCompany' THEN 'cost_on_wellbeing_pct'
            WHEN tag = 'AmountOfCostIncurredOnWellBeingMeasures' THEN '_wellbeing_cost'
            WHEN tag = 'TotalRevenueOfTheCompany' THEN '_wellbeing_rev'
            ELSE NULL
        END AS metric_name,
        val,
        ctx_priority
    FROM classified_facts
    WHERE ctx_priority >= 2
),
resolved_scalar AS (
    SELECT DISTINCT ON (client_uuid, metric_name)
        client_uuid,
        metric_name,
        val
    FROM scalar_facts
    WHERE metric_name IS NOT NULL
    ORDER BY client_uuid, metric_name, ctx_priority DESC, val DESC
),
pivoted_scalar AS (
    SELECT
        client_uuid,
        MAX(val) FILTER (WHERE metric_name = 'scope_1_tco2e') AS scope_1_tco2e,
        MAX(val) FILTER (WHERE metric_name = 'scope_2_tco2e') AS scope_2_tco2e,
        MAX(val) FILTER (WHERE metric_name = 'scope_3_tco2e') AS scope_3_tco2e,
        MAX(val) FILTER (WHERE metric_name = 'water_withdrawal_kl') AS water_withdrawal_kl,
        MAX(val) FILTER (WHERE metric_name = 'energy_intensity') AS energy_intensity,
        MAX(val) FILTER (WHERE metric_name = 'waste_intensity') AS waste_intensity,
        CASE
            WHEN MAX(val) FILTER (WHERE metric_name = '_energy_total') > 0 THEN
                ROUND(((MAX(val) FILTER (WHERE metric_name = '_energy_renew') * 100.0) /
                       NULLIF(MAX(val) FILTER (WHERE metric_name = '_energy_total'), 0))::numeric, 2)
            ELSE NULL
        END AS renewable_energy_pct,
        COALESCE(
            MAX(val) FILTER (WHERE metric_name = 'female_board_directors_pct'),
            CASE
                WHEN MAX(val) FILTER (WHERE metric_name = '_total_bod') > 0 THEN
                    ROUND(((MAX(val) FILTER (WHERE metric_name = '_female_bod') * 100.0) /
                           NULLIF(MAX(val) FILTER (WHERE metric_name = '_total_bod'), 0))::numeric, 2)
                ELSE NULL
            END
        ) AS female_board_directors_pct,
        COALESCE(
            MAX(val) FILTER (WHERE metric_name = 'gross_wages_to_female_pct'),
            CASE
                WHEN MAX(val) FILTER (WHERE metric_name = '_wages_total') > 0 THEN
                    ROUND(((MAX(val) FILTER (WHERE metric_name = '_wages_female') * 100.0) /
                           NULLIF(MAX(val) FILTER (WHERE metric_name = '_wages_total'), 0))::numeric, 2)
                ELSE NULL
            END
        ) AS gross_wages_to_female_pct,
        MAX(val) FILTER (WHERE metric_name = 'employee_turnover_pct') AS employee_turnover_pct,
        COALESCE(
            MAX(val) FILTER (WHERE metric_name = 'female_workforce_pct'),
            CASE
                WHEN MAX(val) FILTER (WHERE metric_name = '_total_emp') > 0 THEN
                    ROUND(((MAX(val) FILTER (WHERE metric_name = '_female_emp') * 100.0) /
                           NULLIF(MAX(val) FILTER (WHERE metric_name = '_total_emp'), 0))::numeric, 2)
                ELSE NULL
            END
        ) AS female_workforce_pct,
        MAX(val) FILTER (WHERE metric_name = 'posh_complaints_upheld') AS posh_complaints_upheld,
        MAX(val) FILTER (WHERE metric_name = 'total_posh_complaints') AS total_posh_complaints,
        COALESCE(
            MAX(val) FILTER (WHERE metric_name = 'cost_on_wellbeing_pct'),
            CASE
                WHEN MAX(val) FILTER (WHERE metric_name = '_wellbeing_rev') > 0 THEN
                    ROUND(((MAX(val) FILTER (WHERE metric_name = '_wellbeing_cost') * 100.0) /
                           NULLIF(MAX(val) FILTER (WHERE metric_name = '_wellbeing_rev'), 0))::numeric, 2)
                ELSE NULL
            END
        ) AS cost_on_wellbeing_pct
    FROM resolved_scalar
    GROUP BY client_uuid
),
summed_cy_facts AS (
    SELECT
        client_uuid,
        SUM(val) FILTER (WHERE tag = 'TotalRecordableWorkRelatedInjuries' AND ctx IN ('D_Employees', 'D_Workers')) AS work_related_injuries,
        SUM(val) FILTER (WHERE tag = 'NumberOfFatalities' AND ctx IN ('D_Employees', 'D_Workers')) AS fatalities,
        SUM(val) FILTER (
            WHERE tag IN (
                'NumberOfDirectorsAgainstWhomDisciplinaryActionWasTaken',
                'NumberOfKMPsAgainstWhomDisciplinaryActionWasTaken',
                'NumberOfEmployeesAgainstWhomDisciplinaryActionWasTaken',
                'NumberOfWorkersAgainstWhomDisciplinaryActionWasTaken'
            ) AND ctx_priority >= 2
        ) AS disciplinary_actions,
        SUM(val) FILTER (
            WHERE tag IN (
                'NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear',
                'ConsumerComplaintsReceivedDuringTheYear'
            ) AND ctx_priority >= 2
        ) AS complaints_filed,
        SUM(val) FILTER (
            WHERE tag IN (
                'NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear',
                'ConsumerComplaintsPendingResolutionAtEndOfYear'
            ) AND ctx_priority >= 2
        ) AS pending_complaints
    FROM classified_facts
    GROUP BY client_uuid
)
SELECT
    e.entity_id,
    e.company_name,
    e.corporate_id_number,
    COALESCE(e.industry_sector, 'Unclassified') AS industry_sector,
    COALESCE(e.industry_division, 'General / Unclassified') AS industry_division,
    e.financial_year_start,
    e.financial_year_end,
    ROUND((e.paid_up_shares_value / 10000000.0)::numeric, 2) AS paid_up_capital_cr,
    p.scope_1_tco2e,
    p.scope_2_tco2e,
    p.scope_3_tco2e,
    p.water_withdrawal_kl,
    p.energy_intensity,
    p.waste_intensity,
    p.renewable_energy_pct,
    p.female_board_directors_pct,
    p.gross_wages_to_female_pct,
    p.employee_turnover_pct,
    s.work_related_injuries,
    s.fatalities,
    p.female_workforce_pct,
    p.posh_complaints_upheld,
    p.total_posh_complaints,
    s.complaints_filed,
    s.pending_complaints,
    s.disciplinary_actions,
    p.cost_on_wellbeing_pct
FROM entity_list e
LEFT JOIN pivoted_scalar p ON e.entity_id = p.client_uuid
LEFT JOIN summed_cy_facts s ON e.entity_id = s.client_uuid;

CREATE OR REPLACE VIEW view_sector_benchmarks AS
SELECT
    industry_sector,
    COUNT(entity_id) AS entities_reporting,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY scope_1_tco2e)::numeric, 2) AS median_scope_1,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY scope_2_tco2e)::numeric, 2) AS median_scope_2,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY scope_3_tco2e)::numeric, 2) AS median_scope_3,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY water_withdrawal_kl)::numeric, 2) AS median_water,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY energy_intensity)::numeric, 4) AS median_energy_intensity,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY waste_intensity)::numeric, 4) AS median_waste_intensity,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY renewable_energy_pct)::numeric, 2) AS median_renewable_energy_pct,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY female_board_directors_pct)::numeric, 2) AS median_female_board_pct,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY gross_wages_to_female_pct)::numeric, 2) AS median_gross_wages_pct,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY employee_turnover_pct)::numeric, 2) AS median_turnover_pct,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY work_related_injuries)::numeric, 2) AS median_injuries,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY fatalities)::numeric, 2) AS median_fatalities,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY female_workforce_pct)::numeric, 2) AS median_female_workforce_pct,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY posh_complaints_upheld)::numeric, 2) AS median_posh_upheld,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY total_posh_complaints)::numeric, 2) AS median_total_posh,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY complaints_filed)::numeric, 2) AS median_complaints_filed,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY pending_complaints)::numeric, 2) AS median_pending_complaints,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY disciplinary_actions)::numeric, 2) AS median_disciplinary_actions,
    ROUND(percentile_cont(0.5) WITHIN GROUP (ORDER BY cost_on_wellbeing_pct)::numeric, 2) AS median_cost_wellbeing_pct
FROM view_company_esg_kpis
GROUP BY industry_sector;