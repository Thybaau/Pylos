use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub organization_id: Option<String>,
    pub team_ids: Vec<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub organization_id: Option<String>,
    pub team_ids: Vec<String>,
    pub user_ids: Vec<String>,
    pub model_ids: Vec<String>,
    pub provider_ids: Vec<String>,
    pub is_active: bool,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub policy_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    pub allowed_models: Vec<String>,
    pub allowed_providers: Vec<String>,
    pub max_tokens_per_call: Option<i64>,
    pub max_calls_per_minute: Option<i64>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
