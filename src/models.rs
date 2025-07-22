use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(rename = "type")]
    pub type_field: String,
    pub actor: String,
    pub attributed_to: String,
    pub attachment: Vec<Attachment>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub content: String,
    pub tag: Vec<Tag>,
    pub published: String,
    pub id: String,
    pub context: String,
    pub conversation: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(rename = "type")]
    pub type_field: String,
    pub media_type: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: String,
}
