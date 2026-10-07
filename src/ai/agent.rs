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

        // 1. Fetch Verified Company Profile from PostgreSQL
        let lookup_key = if !cin.trim().is_empty() {
            cin.trim()
        } else {
            company_name.trim()
        };

        if !lookup_key.is_empty() {
            if let Ok(profile) = self.tools.get_company_profile(lookup_key).await {
                context_buffer.push_str(&profile);
                context_buffer.push('\n');
            }
        }

        // 2. Dynamic Topic Keyword Extraction from User Query
        let query_lower = user_query.to_lowercase();
        let topic_keywords = [
            ("emission", "emissions"),
            ("ghg", "GHG emissions"),
            ("scope 1", "Scope 1"),
            ("scope 2", "Scope 2"),
            ("scope 3", "Scope 3"),
            ("water", "water withdrawal"),
            ("energy", "energy consumption"),
            ("renewable", "renewable energy"),
            ("waste", "waste generated"),
            ("female", "female gender diversity"),
            ("women", "women workforce"),
            ("director", "board directors"),
            ("wage", "wages and remuneration"),
            ("turnover", "employee turnover"),
            ("injur", "work injuries"),
            ("fatal", "fatalities"),
            ("posh", "POSH sexual harassment"),
            ("complaint", "statutory complaints"),
            ("disciplinary", "disciplinary actions"),
            ("wellbeing", "well-being spending"),
            ("safety", "health and safety"),
            ("training", "training and skill development"),
            ("human right", "human rights"),
            ("csr", "corporate social responsibility"),
        ];

        let mut matched_topics = Vec::new();
        for (stem, label) in topic_keywords {
            if query_lower.contains(stem) {
                matched_topics.push((stem, label));
            }
        }

        // Query database for up to 3 most relevant topics to avoid token bloat
        for (stem, _) in matched_topics.iter().take(3) {
            if !lookup_key.is_empty() {
                if let Ok(facts) = self.tools.query_company_metrics(lookup_key, stem).await {
                    if !facts.is_empty() {
                        context_buffer.push_str(&facts);
                        context_buffer.push('\n');
                    }
                }
            }
        }

        // 3. Inject In-Memory Disclosures from the active UI Viewer/Dashboard
        if !ui_context.trim().is_empty() {
            context_buffer.push_str("ACTIVE REPORT DISCLOSURES (CURRENT VIEW):\n");
            context_buffer.push_str(ui_context.trim());
            context_buffer.push('\n');
        }

        // 4. Assemble Grounded System Instructions
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

        // 5. Query LLM with clean separation of system instructions and user prompt
        self.client.ask(&system_instructions, user_query).await
    }
}
