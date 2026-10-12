// src/ai/client.rs

use reqwest::Client;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    Ollama { model: String, endpoint: String },
    Gemini { model: String, api_key: String },
}

pub struct LlmClient {
    http: Client,
    pub provider: LlmProvider,
}

impl LlmClient {
    pub fn new(provider: LlmProvider) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        Self { http, provider }
    }

    /// Sends prompt with verified statutory context and receives grounded response
    pub async fn ask(
        &self,
        system_context: &str,
        user_query: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match &self.provider {
            LlmProvider::Ollama { model, endpoint } => {
                let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));
                let payload = json!({
                    "model": model,
                    "stream": false,
                    "messages": [
                        { "role": "system", "content": system_context },
                        { "role": "user", "content": user_query }
                    ]
                });

                let resp = self.http.post(&url).json(&payload).send().await?;
                if !resp.status().is_success() {
                    let status = resp.status(); // ✅ Capture status BEFORE .text() consumes resp
                    let err_text = resp.text().await.unwrap_or_default();
                    return Err(format!("Ollama HTTP {}: {}", status, err_text).into());
                }

                let body: serde_json::Value = resp.json().await?;

                if let Some(err) = body.get("error").and_then(|e| e.as_str()) {
                    return Err(format!("Ollama error: {}", err).into());
                }

                let text = body["message"]["content"]
                    .as_str()
                    .unwrap_or("No response generated.")
                    .to_string();

                Ok(text)
            }

            LlmProvider::Gemini { model, api_key } => {
                let clean_model = model.trim_start_matches("models/");
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    clean_model, api_key
                );

                // Use Gemini v1beta native system_instruction structure
                let payload = json!({
                    "system_instruction": {
                        "parts": [{ "text": system_context }]
                    },
                    "contents": [{
                        "role": "user",
                        "parts": [{ "text": user_query }]
                    }]
                });

                let resp = self.http.post(&url).json(&payload).send().await?;
                if !resp.status().is_success() {
                    let status = resp.status(); // ✅ Capture status BEFORE .text() consumes resp
                    let err_text = resp.text().await.unwrap_or_default();
                    return Err(format!("Gemini API error (HTTP {}): {}", status, err_text).into());
                }

                let body: serde_json::Value = resp.json().await?;

                // 1. Check for API-level error block
                if let Some(err_msg) = body
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    return Err(format!("Gemini API error: {}", err_msg).into());
                }

                // 2. Check for Prompt-level safety block
                if let Some(feedback) = body.get("promptFeedback") {
                    if let Some(block_reason) = feedback.get("blockReason").and_then(|r| r.as_str())
                    {
                        return Err(format!(
                            "Prompt blocked by Gemini safety filters: {}",
                            block_reason
                        )
                        .into());
                    }
                }

                // 3. Inspect Candidates array
                let candidates = match body.get("candidates").and_then(|c| c.as_array()) {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => return Err("Gemini produced no candidates for this prompt.".into()),
                };

                let first_candidate = &candidates[0];
                let finish_reason = first_candidate
                    .get("finishReason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("UNKNOWN");

                if finish_reason != "STOP"
                    && finish_reason != "MAX_TOKENS"
                    && finish_reason != "UNKNOWN"
                    && finish_reason != "FINISH_REASON_UNSPECIFIED"
                {
                    return Err(format!(
                        "Gemini generation halted prematurely (finishReason: {}).",
                        finish_reason
                    )
                    .into());
                }

                // 4. Extract generated text
                if let Some(text) = first_candidate["content"]["parts"][0]["text"].as_str() {
                    Ok(text.to_string())
                } else {
                    Err(format!(
                        "Gemini candidate contains no text parts (finishReason: {}).",
                        finish_reason
                    )
                    .into())
                }
            }
        }
    }
}
