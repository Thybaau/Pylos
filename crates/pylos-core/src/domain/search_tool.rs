use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchToolConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
