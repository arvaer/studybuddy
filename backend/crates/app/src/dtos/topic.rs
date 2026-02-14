use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::Topic;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTopicRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTopicRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicResponse {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub color:       String,
    pub created_at:  String,
}

impl From<Topic> for TopicResponse {
    fn from(t: Topic) -> Self {
        Self {
            id:          t.id.to_string(),
            name:        t.name,
            description: t.description,
            color:       t.color,
            created_at:  t.created_at.to_rfc3339(),
        }
    }
}
