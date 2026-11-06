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

    // API Resource & Projection Configuration (Unified Resource and per-endpoint overrides)
    #[serde(rename = "resourceFields", alias = "resource_fields", default)]
    pub resource_fields: Option<Vec<String>>,
    #[serde(rename = "resourceRelations", alias = "resource_relations", default)]
    pub resource_relations: Option<Vec<String>>,
    #[serde(rename = "resourceRelationFields", alias = "resource_relation_fields", default)]
    pub resource_relation_fields: Option<std::collections::HashMap<String, Vec<String>>>,

    #[serde(rename = "listFields", alias = "list_fields", default)]
    pub list_fields: Option<Vec<String>>,
    #[serde(rename = "allFields", alias = "all_fields", default)]
    pub all_fields: Option<Vec<String>>,
    #[serde(rename = "viewFields", alias = "view_fields", default)]
    pub view_fields: Option<Vec<String>>,

    #[serde(rename = "listRelations", alias = "list_relations", default)]
    pub list_relations: Option<Vec<String>>,
    #[serde(rename = "allRelations", alias = "all_relations", default)]
    pub all_relations: Option<Vec<String>>,
    #[serde(rename = "viewRelations", alias = "view_relations", default)]
    pub view_relations: Option<Vec<String>>,

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
            resource_fields: None,
            resource_relations: None,
            resource_relation_fields: None,
            list_fields: None,
            all_fields: None,
            view_fields: None,
            list_relations: None,
            all_relations: None,
            view_relations: None,
            created: now.clone(),
            updated: now,
        }
    }

    pub fn effective_relation_fields(&self, relation_name: &str) -> Option<&[String]> {
        self.resource_relation_fields.as_ref().and_then(|m| {
            m.get(relation_name)
                .or_else(|| {
                    let stripped = relation_name.strip_suffix("_id").unwrap_or(relation_name);
                    m.get(stripped)
                })
                .map(|v| v.as_slice())
        })
    }

    pub fn effective_resource_fields(&self, action: Option<&str>) -> Option<&[String]> {
        if let Some(ref rf) = self.resource_fields {
            return Some(rf.as_slice());
        }
        match action {
            Some("list") => self.list_fields.as_deref(),
            Some("all") => self.all_fields.as_deref().or(self.list_fields.as_deref()),
            Some("view") => self.view_fields.as_deref().or(self.list_fields.as_deref()),
            _ => self.list_fields.as_deref(),
        }
    }

    pub fn effective_resource_relations(&self, action: Option<&str>) -> Option<&[String]> {
        if let Some(ref rr) = self.resource_relations {
            return Some(rr.as_slice());
        }
        match action {
            Some("list") => self.list_relations.as_deref(),
            Some("all") => self.all_relations.as_deref().or(self.list_relations.as_deref()),
            Some("view") => self.view_relations.as_deref().or(self.list_relations.as_deref()),
            _ => self.list_relations.as_deref(),
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
