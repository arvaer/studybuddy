use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::Resource;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceRequest {
    #[validate(length(min = 1, max = 300))]
    pub title:        String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub url:          Option<String>,
    pub topic_id:     String,
    pub concept_ids:  Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceResponse {
    pub id:            String,
    pub title:         String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub url:           Option<String>,
    pub file_path:     Option<String>,
    pub content_text:  Option<String>,
    pub page_count:    i32,
    pub topic_id:      String,
    pub concept_ids:   Vec<String>,
    pub added_at:      String,
}

impl From<Resource> for ResourceResponse {
    fn from(r: Resource) -> Self {
        Self {
            id:            r.id.to_string(),
            title:         r.title,
            resource_type: r.resource_type.to_string(),
            url:           r.url,
            file_path:     r.file_path,
            content_text:  r.content_text,
            page_count:    r.page_count,
            topic_id:      r.topic_id.to_string(),
            concept_ids:   r.concept_ids.iter().map(|id| id.to_string()).collect(),
            added_at:      r.added_at.to_rfc3339(),
        }
    }
}
