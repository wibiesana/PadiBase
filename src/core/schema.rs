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
    #[serde(alias = "string", alias = "varchar", alias = "char", alias = "uuid", alias = "text")]
    Text,
    #[serde(alias = "int", alias = "integer", alias = "bigint", alias = "smallint", alias = "tinyint", alias = "real", alias = "float", alias = "double", alias = "decimal", alias = "numeric")]
    Number,
    #[serde(alias = "boolean")]
    Bool,
    Email,
    Url,
    #[serde(alias = "datetime", alias = "timestamp", alias = "timestamptz", alias = "time")]
    Date,
    #[serde(alias = "enum")]
    Select,
    #[serde(alias = "jsonb")]
    Json,
    #[serde(alias = "blob", alias = "bytea", alias = "binary")]
    File,
    #[serde(alias = "fk", alias = "foreign_key")]
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

    /// Whether auto load relations is enabled for this collection by default (default: false)
    #[serde(rename = "autoLoadRelations", alias = "auto_load_relations", default)]
    pub auto_load_relations: bool,

    /// Whether blameable behavior (tracking created_by and updated_by) is enabled
    #[serde(default)]
    pub blameable: bool,

    // API Rules (PocketBase rules)
    #[serde(rename = "listRule", default)]
    pub list_rule: Option<String>,
    #[serde(rename = "allRule", default)]
    pub all_rule: Option<String>,
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
            auto_load_relations: false,
            blameable: true,
            list_rule: None,
            all_rule: None,
            view_rule: None,
            create_rule: Some("@request.auth.id != \"\"".to_string()),
            update_rule: Some("@request.auth.id != \"\"".to_string()),
            delete_rule: Some("@request.auth.id != \"\"".to_string()),
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

fn default_http_method() -> String {
    "GET".to_string()
}

fn default_endpoint_auth_rule() -> String {
    "public".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEndpoint {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    pub sql: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "authRule", alias = "auth_rule", default = "default_endpoint_auth_rule")]
    pub auth_rule: String,
    #[serde(default)]
    pub transaction: bool,
    #[serde(rename = "singleRow", alias = "single_row", default)]
    pub single_row: bool,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
}

pub fn generate_id() -> String {
    let raw = Uuid::new_v4().simple().to_string();
    raw[..15].to_string()
}
