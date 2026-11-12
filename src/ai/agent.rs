// src/ai/agent.rs

use crate::ai::client::LlmClient;
use crate::ai::tools::McpTools;
use sqlx::PgPool;

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

        if !lookup_key.is_empty()
            && let Ok(profile) = self.tools.get_company_profile(lookup_key).await
        {
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
            ("water", "Water"),
            ("energy", "Energy"),
            ("renewable", "Renewable"),
            ("waste", "Waste"),
            ("female", "Female"),
            ("women", "Women"),
            ("director", "Director"),
            ("wage", "Wages"),
            ("turnover", "Turnover"),
            ("injur", "Injur"),
            ("fatal", "Fatal"),
            ("posh", "POSH"),
            ("complaint", "Complaint"),
            ("disciplinary", "Disciplinary"),
            ("wellbeing", "WellBeing"),
            ("well-being", "WellBeing"),
            ("safety", "Safety"),
            ("training", "Training"),
            ("human right", "HumanRights"),
            ("csr", "CSR"),
            ("green credit", "GreenCredit"),
        ];

        let mut matched_stems = Vec::new();
        for (_phrase, stem) in topic_keywords {
            if query_lower.contains(_phrase) && !matched_stems.contains(&stem) {
                matched_stems.push(stem);
            }
        }

        if !lookup_key.is_empty() {
            for stem in matched_stems.into_iter().take(6) {
                if let Ok(facts) = self.tools.query_company_metrics(lookup_key, stem).await
                    && !facts.is_empty()
                {
                    context_buffer.push_str(&facts);
                    context_buffer.push('\n');
                }
            }
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
