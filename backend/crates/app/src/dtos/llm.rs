use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProxyRequest {
    pub provider:   String,
    pub base_url:   String,
    pub api_key:    String,
    pub model:      String,
    pub messages:   Vec<serde_json::Value>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProxyResponse {
    pub content: String,
    pub raw:     serde_json::Value,
}
