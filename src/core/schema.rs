use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollectionType {
    Base,
    Auth,
    View,
}

impl Default for CollectionType {
    fn default() -> Self {
        CollectionType::Base
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Bool,
    Email,
    Url,
    Date,
    Select,
    Json,
    File,
    Relation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub options: serde_json::Value,
}

fn default_realtime() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(rename = "type", default)]
    pub collection_type: CollectionType,
    #[serde(default)]
    pub schema: Vec<SchemaField>,

    /// Whether realtime SSE broadcasting is enabled for this collection
    #[serde(default = "default_realtime")]
    pub realtime: bool,

    // API Rules (PocketBase rules)
    #[serde(rename = "listRule", default)]
    pub list_rule: Option<String>,
    #[serde(rename = "viewRule", default)]
    pub view_rule: Option<String>,
    #[serde(rename = "createRule", default)]
    pub create_rule: Option<String>,
    #[serde(rename = "updateRule", default)]
    pub update_rule: Option<String>,
    #[serde(rename = "deleteRule", default)]
    pub delete_rule: Option<String>,

    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
}

impl Collection {
    pub fn new(name: String, collection_type: CollectionType) -> Self {
        let now = Utc::now().to_rfc3339();
        let id = generate_id();
        Self {
            id,
            name,
            collection_type,
            schema: Vec::new(),
            realtime: true,
            list_rule: None,
            view_rule: None,
            create_rule: None,
            update_rule: None,
            delete_rule: None,
            created: now.clone(),
            updated: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admin {
    pub id: String,
    pub email: String,
    #[serde(rename = "passwordHash")]
    pub password_hash: String,
    #[serde(rename = "tokenKey")]
    pub token_key: String,
    pub created: String,
    pub updated: String,
}

pub fn generate_id() -> String {
    let raw = Uuid::new_v4().simple().to_string();
    raw[..15].to_string()
}
