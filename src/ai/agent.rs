// src/ai/agent.rs

use crate::ai::client::LlmClient;
use crate::ai::tools::McpTools;
use sqlx::PgPool;
use uuid::Uuid;

pub struct RagAgent {
    tools: McpTools,
    client: LlmClient,
}

impl RagAgent {
    pub fn new(pool: PgPool, client: LlmClient) -> Self {
        Self {
            tools: McpTools::new(pool),
            client,
        }
    }

    /// Handles an ESG question using dynamic database lookups and verified UI context
    pub async fn answer_question(
        &self,
        entity_id: Option<Uuid>,
        company_name: &str,
        cin: &str,
        ui_context: &str,
        user_query: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut context_buffer = String::new();

        let lookup_key = if !cin.trim().is_empty() {
            cin.trim()
        } else {
            company_name.trim()
        };

        let mut resolved_entity_id = entity_id;
        if (resolved_entity_id.is_some() || !lookup_key.is_empty())
            && let Ok((eid_opt, profile)) = self
                .tools
                .get_company_profile(resolved_entity_id, lookup_key)
                .await
        {
            if resolved_entity_id.is_none() {
                resolved_entity_id = eid_opt;
            }
            context_buffer.push_str(&profile);
            context_buffer.push('\n');
        }

        let query_lower = user_query.to_lowercase();
        let topic_keywords = [
            ("scope 1", "Scope1"),
            ("scope 2", "Scope2"),
            ("scope 3", "Scope3"),
            ("emission", "Emissions"),
            ("ghg", "ghg"),
            ("air", "AirEmission"),
            ("nox", "NOx"),
            ("sox", "SOx"),
            ("water", "Water"),
            ("energy", "Energy"),
            ("fuel", "Fuel"),
            ("electricity", "Electricity"),
            ("renewable", "Renewable"),
            ("solar", "Renewable"),
            ("waste", "Waste"),
            ("plastic", "Plastic"),
            ("e-waste", "EWaste"),
            ("hazardous", "Hazardous"),
            ("recycl", "Recycl"),
            ("female", "Female"),
            ("women", "Women"),
            ("gender", "Gender"),
            ("director", "Director"),
            ("board", "BoardOfDirectors"),
            ("wage", "Wages"),
            ("remuneration", "Remuneration"),
            ("turnover", "Turnover"),
            ("injur", "Injur"),
            ("fatal", "Fatal"),
            ("posh", "POSH"),
            ("harassment", "SexualHarassment"),
            ("complaint", "Complaint"),
            ("grievance", "Grievance"),
            ("disciplinary", "Disciplinary"),
            ("wellbeing", "WellBeing"),
            ("well-being", "WellBeing"),
            ("safety", "Safety"),
            ("training", "Training"),
            ("human right", "HumanRights"),
            ("csr", "CSR"),
            ("green credit", "GreenCredit"),
            ("value chain", "ValueChain"),
            ("assurance", "Assurance"),
            ("assessment", "Assessment"),
            ("material", "Material"),
            ("biodiversity", "Biodiversity"),
            ("policy", "Policy"),
        ];

        let mut matched_stems = Vec::new();
        for (phrase, stem) in topic_keywords {
            if query_lower.contains(phrase) && !matched_stems.contains(&stem) {
                matched_stems.push(stem);
            }
        }
        matched_stems.truncate(8);

        if let Some(eid) = resolved_entity_id
            && !matched_stems.is_empty()
            && let Ok(facts) = self
                .tools
                .query_company_metrics_batch(eid, &matched_stems)
                .await
            && !facts.is_empty()
        {
            context_buffer.push_str(&facts);
            context_buffer.push('\n');
        }

        if !ui_context.trim().is_empty() {
            context_buffer.push_str("ACTIVE REPORT DISCLOSURES (CURRENT VIEW):\n");
            context_buffer.push_str(ui_context.trim());
            context_buffer.push('\n');
        }

        let system_instructions = format!(
            r#"You are an expert SEBI BRSR and ESG sustainability analyst.
You are reviewing verified statutory filings for: {} (CIN: {}).

INSTRUCTIONS:
1. Base your response STRICTLY on the verified filing data provided below.
2. If a metric or disclosure is NOT present in the data below, state clearly that it is not disclosed in the active filing rather than speculating.
3. Always cite exact numbers, standardized units (e.g., tCO2e, KL, GJ, %), and the corresponding BRSR Principle or Section when available.
4. Keep explanations concise, professional, and audit-ready.

VERIFIED REPOSITORY DATA & REPORT DISCLOSURES:
{}
"#,
            company_name, cin, context_buffer
        );

        self.client.ask(&system_instructions, user_query).await
    }
}
