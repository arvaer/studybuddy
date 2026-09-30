use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmMessage {
    pub role:    LlmRole,
    pub content: String,
}

/// The only things a client may send. Provider, model, credential and
/// destination are server configuration; any such field is rejected.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmProxyRequest {
    pub messages:   Vec<LlmMessage>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProxyResponse {
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_supplied_configuration_is_rejected() {
        for extra in ["provider", "baseUrl", "apiKey", "model"] {
            let body = format!(r#"{{"messages":[{{"role":"user","content":"hi"}}],"{extra}":"x"}}"#);
            assert!(serde_json::from_str::<LlmProxyRequest>(&body).is_err(), "{extra} accepted");
        }
    }

    #[test]
    fn messages_and_max_tokens_are_accepted() {
        let req: LlmProxyRequest =
            serde_json::from_str(r#"{"messages":[{"role":"system","content":"s"}],"maxTokens":5}"#).unwrap();
        assert_eq!(req.messages[0].role, LlmRole::System);
        assert_eq!(req.max_tokens, Some(5));
    }

    #[test]
    fn unknown_role_is_rejected() {
        assert!(serde_json::from_str::<LlmMessage>(r#"{"role":"tool","content":"x"}"#).is_err());
    }
}
