// src/app/tab_form_viewer/mod.rs

pub mod formatter;
pub mod nav_tree;
pub mod profile;
pub mod question_cards;
pub mod report_view;
pub mod section_b_matrix;

use crate::app::ai_drawer::render_ai_copilot_drawer;
use crate::app::state::{AppState, CompanyFilingSummary, FormMasterItem, FormViewerEvent};
use eframe::egui::{self, Color32, ComboBox, Layout, RichText, TextEdit, Ui, Vec2};
use sqlx::Row;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("📄 Official BRSR Report Explorer");
    ui.add_space(4.0);

    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("⚠ PostgreSQL Disconnected")
                        .size(18.0)
                        .color(Color32::from_rgb(230, 90, 90)),
                );
                ui.label("Connect your database container to inspect reports.");
                ui.add_space(8.0);
                if ui
                    .button(
                        RichText::new("⚡ Attempt Reconnect")
                            .color(Color32::from_rgb(100, 180, 240)),
                    )
                    .clicked()
                {
                    crate::app::state::trigger_db_reconnect(state);
                }
            });
            return;
        }
    };

    // Embedded AI Copilot Drawer (Context-Aware to Active Report)
    let company_label = if state.form_viewer.selected_cin.is_empty() {
        "None selected".to_string()
    } else {
        state.form_viewer.selected_cin.clone()
    };
    let copilot_hint = format!("Filing Explorer | Entity CIN: {}", company_label);
    render_ai_copilot_drawer(ui, state, &copilot_hint);

    // Drain incoming copilot responses
    if let Some(ref mut rx) = state.ai.chat_rx {
        while let Ok(msg) = rx.try_recv() {
            state.ai.chat_history.push(msg);
        }
    }
    // 1. Asynchronously load the statutory Form Template from form_master (with guard against error loops)
    if !state.form_viewer.template_loaded {
        state.form_viewer.template_loaded = true; // Latch immediately to prevent 60 task/sec spam
        let pool_cloned = pool.clone();
        let tx = state.form_tx.clone();

        tokio::spawn(async move {
            let sql = r#"
                SELECT 
                    question_key, section, principle_number, sub_section, indicator_type,
                    question_in_form, question_number, question_order, response_datatype,
                    COALESCE(field_flag, 0)::smallint as field_flag, append_to_field,
                    COALESCE(NULLIF(section_text, ''), 'SECTION ' || section) as section_text,
                    COALESCE(NULLIF(principle_text, ''), 'General') as principle_text,
                    COALESCE(NULLIF(sub_section_text, ''), 'General') as sub_section_text,
                    COALESCE(NULLIF(indicator_text, ''), 'General') as indicator_text,
                    question_text, standard_indicator_capmkt, standard_context_reference,
                    input_text
                FROM form_master
                WHERE (legacy = FALSE OR legacy IS NULL)
                ORDER BY section, principle_number, sub_section, question_number, question_order;
            "#;

            match sqlx::query(sql).fetch_all(&pool_cloned).await {
                Ok(rows) => {
                    let mut template = Vec::with_capacity(rows.len());
                    for r in rows {
                        let field_flag = r
                            .try_get::<i16, _>("field_flag")
                            .or_else(|_| r.try_get::<i32, _>("field_flag").map(|v| v as i16))
                            .unwrap_or(0);

                        let p_num = r
                            .try_get::<i16, _>("principle_number")
                            .or_else(|_| r.try_get::<i32, _>("principle_number").map(|v| v as i16))
                            .unwrap_or(0);

                        let sub_sec = r
                            .try_get::<i16, _>("sub_section")
                            .or_else(|_| r.try_get::<i32, _>("sub_section").map(|v| v as i16))
                            .unwrap_or(0);

                        let q_in_form =
                            r.try_get::<i16, _>("question_in_form").ok().or_else(|| {
                                r.try_get::<i32, _>("question_in_form")
                                    .ok()
                                    .map(|v| v as i16)
                            });

                        let q_num = r
                            .try_get::<i16, _>("question_number")
                            .or_else(|_| r.try_get::<i32, _>("question_number").map(|v| v as i16))
                            .unwrap_or(0);

                        let q_order = r
                            .try_get::<i16, _>("question_order")
                            .or_else(|_| r.try_get::<i32, _>("question_order").map(|v| v as i16))
                            .unwrap_or(0);

                        template.push(FormMasterItem {
                            question_key: r.get("question_key"),
                            section: r.get("section"),
                            principle_number: p_num,
                            sub_section: sub_sec,
                            indicator_type: r.get("indicator_type"),
                            question_in_form: q_in_form,
                            question_number: q_num,
                            question_order: q_order,
                            response_datatype: r.get("response_datatype"),
                            field_flag,
                            append_to_field: r.get("append_to_field"),
                            section_text: question_cards::clean_text(r.get("section_text")),
                            principle_text: question_cards::clean_text(r.get("principle_text")),
                            sub_section_text: question_cards::clean_text(r.get("sub_section_text")),
                            indicator_text: question_cards::clean_text(r.get("indicator_text")),
                            question_text: question_cards::clean_text(r.get("question_text")),
                            standard_indicator_capmkt: r.get("standard_indicator_capmkt"),
                            standard_context_reference: r.get("standard_context_reference"),
                            input_text: r
                                .get::<Option<String>, _>("input_text")
                                .map(|s| question_cards::clean_text(&s)),
                        });
                    }
                    let _ = tx.send(FormViewerEvent::TemplateLoaded(template)).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(FormViewerEvent::Error(format!(
                            "Failed to load template: {}",
                            e
                        )))
                        .await;
                }
            }
        });
    }

    // 2. Load Ingested Filings dropdown list
    if !state.form_viewer.filings_loaded {
        state.form_viewer.filings_loaded = true;
        let pool_cloned = pool.clone();
        let tx = state.form_tx.clone();

        tokio::spawn(async move {
            let sql = r#"
                SELECT 
                    entity_id, corporate_id_number, company_name, 
                    address_registered_office, address_corporate_office,
                    company_email, company_telephone, website, paid_up_shares_value,
                    financial_year_start, financial_year_end
                FROM entity_list
                ORDER BY company_name ASC, financial_year_start DESC;
            "#;
            if let Ok(rows) = sqlx::query(sql).fetch_all(&pool_cloned).await {
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
                let _ = tx.send(FormViewerEvent::FilingsLoaded(items)).await;
            }
        });
    }

    // 3. Fetch Facts when Entity Selection changes
    if let Some(selected_id) = state.form_viewer.selected_entity_id {
        if state.form_viewer.cached_entity_id != Some(selected_id)
            && !state.form_viewer.is_loading_facts
        {
            state.form_viewer.is_loading_facts = true;
            let pool_cloned = pool.clone();
            let tx = state.form_tx.clone();

            tokio::spawn(async move {
                let sql = r#"
                    SELECT 
                        section, principle_number, sub_section, indicator_type,
                        standard_indicator_capmkt, standard_context_reference,
                        raw_value, standard_numeric_value, standard_unit_reference
                    FROM brsr_facts
                    WHERE client_uuid = $1;
                "#;
                match sqlx::query(sql)
                    .bind(selected_id)
                    .fetch_all(&pool_cloned)
                    .await
                {
                    Ok(rows) => {
                        let mut facts = Vec::with_capacity(rows.len());
                        for r in rows {
                            let p_num = r
                                .try_get::<i16, _>("principle_number")
                                .or_else(|_| {
                                    r.try_get::<i32, _>("principle_number").map(|v| v as i16)
                                })
                                .unwrap_or(0);

                            let sub_sec = r
                                .try_get::<i16, _>("sub_section")
                                .or_else(|_| r.try_get::<i32, _>("sub_section").map(|v| v as i16))
                                .unwrap_or(0);

                            facts.push(crate::app::state::FormFactRow {
                                section: r.get("section"),
                                principle_number: p_num,
                                sub_section: sub_sec,
                                indicator_type: r.get("indicator_type"),
                                standard_indicator: r.get("standard_indicator_capmkt"),
                                context_ref: r.get("standard_context_reference"),
                                raw_value: r.get("raw_value"),
                                standard_numeric_value: r.get("standard_numeric_value"),
                                standard_unit: r.get("standard_unit_reference"),
                            });
                        }
                        let _ = tx
                            .send(FormViewerEvent::FactsLoaded {
                                entity_id: selected_id,
                                facts,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(FormViewerEvent::Error(format!(
                                "Failed to load facts: {}",
                                e
                            )))
                            .await;
                    }
                }
            });
        }
    }

    render_header_toolbar(ui, state);

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Responsive split layout
    let left_col_width = 290.0;
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(left_col_width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(left_col_width);
                nav_tree::render(ui, state, ui.available_height());
            },
        );

        ui.separator();

        let right_width = (ui.available_width() - 4.0).max(320.0);
        ui.allocate_ui_with_layout(
            Vec2::new(right_width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(right_width);
                report_view::render(ui, state, ui.available_height());
            },
        );
    });
}

fn render_header_toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            // Read directly from memoized state (zero per-frame allocations)
            let unique_companies = &state.form_viewer.unique_companies;

            ui.label(RichText::new("Company:").strong());
            let current_company_label = if let Some(selected) = unique_companies
                .iter()
                .find(|(cin, _)| cin == &state.form_viewer.selected_cin)
            {
                format!("{} ({})", selected.1, selected.0)
            } else {
                "Select a Company...".to_string()
            };

            ComboBox::from_id_salt("company_selector")
                .width(280.0)
                // Keeps dropdown open when clicking the search box or scrolling
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .selected_text(RichText::new(current_company_label).strong())
                .show_ui(ui, |ui| {
                    // In-Menu Search Field (Type-to-Jump)
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        ui.add(
                            TextEdit::singleline(&mut state.form_viewer.company_search)
                                .hint_text("Type to jump / search 2,000+ companies...")
                                .desired_width(240.0),
                        );
                        if !state.form_viewer.company_search.is_empty()
                            && ui.small_button("✖").clicked()
                        {
                            state.form_viewer.company_search.clear();
                        }
                    });
                    ui.separator();

                    let query_lower = state.form_viewer.company_search.trim().to_lowercase();
                    let filtered_companies: Vec<&(String, String)> = unique_companies
                        .iter()
                        .filter(|(cin, name)| {
                            query_lower.is_empty()
                                || name.to_lowercase().contains(&query_lower)
                                || cin.to_lowercase().contains(&query_lower)
                        })
                        .take(80) // Prevents lag while displaying instant results
                        .collect();

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (cin, name) in filtered_companies {
                                let label = format!("{} ({})", name, cin);
                                if ui
                                    .selectable_value(
                                        &mut state.form_viewer.selected_cin,
                                        cin.clone(),
                                        label,
                                    )
                                    .clicked()
                                {
                                    if let Some(first_filing) = state
                                        .form_viewer
                                        .available_filings
                                        .iter()
                                        .find(|f| &f.corporate_id_number == cin)
                                    {
                                        state.form_viewer.selected_entity_id =
                                            Some(first_filing.entity_id);
                                    }
                                    state.form_viewer.company_search.clear();
                                    ui.close(); // Close the menu once a selection is made
                                }
                            }
                        });
                });
            ui.add_space(8.0);

            ui.label(RichText::new("Financial Year:").strong());
            let available_years: Vec<&CompanyFilingSummary> = state
                .form_viewer
                .available_filings
                .iter()
                .filter(|f| f.corporate_id_number == state.form_viewer.selected_cin)
                .collect();

            let current_fy_label = if let Some(ent_id) = state.form_viewer.selected_entity_id {
                available_years
                    .iter()
                    .find(|f| f.entity_id == ent_id)
                    .map(|f| {
                        format!(
                            "{} → {}",
                            f.fy_start.map(|d| d.to_string()).unwrap_or_default(),
                            f.fy_end.map(|d| d.to_string()).unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|| "Select FY...".to_string())
            } else {
                "Select FY...".to_string()
            };

            ComboBox::from_id_salt("fy_selector")
                .width(180.0)
                .selected_text(current_fy_label)
                .show_ui(ui, |ui| {
                    for filing in available_years {
                        let label = format!(
                            "{} → {}",
                            filing.fy_start.map(|d| d.to_string()).unwrap_or_default(),
                            filing.fy_end.map(|d| d.to_string()).unwrap_or_default()
                        );
                        ui.selectable_value(
                            &mut state.form_viewer.selected_entity_id,
                            Some(filing.entity_id),
                            label,
                        );
                    }
                });

            ui.add_space(8.0);

            ui.label(RichText::new("🔍").strong());
            ui.add(
                TextEdit::singleline(&mut state.form_viewer.search_query)
                    .hint_text("Search question or response...")
                    .desired_width(180.0),
            );

            if !state.form_viewer.search_query.is_empty() && ui.small_button("✖").clicked() {
                state.form_viewer.search_query.clear();
            }
        });
    });
}
