use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub prompt: String,
    #[serde(default = "default_dalle3")]
    pub model: String,
    pub n: Option<u32>,
    pub quality: Option<String>,         // "standard" | "hd"
    pub response_format: Option<String>, // "url" | "b64_json"
    pub size: Option<String>,            // "1024x1024", etc.
    pub style: Option<String>,           // "vivid" | "natural"
    pub user: Option<String>,
}

fn default_dalle3() -> String {
    "dall-e-3".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}
