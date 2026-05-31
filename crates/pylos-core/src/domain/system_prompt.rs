use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
}
