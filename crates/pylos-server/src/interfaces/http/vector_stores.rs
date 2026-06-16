use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::error;

use crate::state::AppState;
use pylos_core::domain::embedding::{EmbeddingInput, EmbeddingRequest};
use pylos_core::domain::request::RequestContext;

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub vector_size: u64,
    pub distance: String,
}

#[derive(Debug, Deserialize)]
pub struct AddDocumentRequest {
    pub text: String,
    pub embedding_model: String,
    pub payload: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct SearchCollectionRequest {
    pub query: String,
    pub embedding_model: String,
    pub limit: Option<usize>,
}

fn get_qdrant_url() -> String {
    std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".to_string())
}

fn get_qdrant_client(timeout_secs: u64) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(key) = std::env::var("QDRANT_API_KEY") {
        if !key.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&key) {
                headers.insert("api-key", val);
            }
        }
    }
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .default_headers(headers)
        .build()
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/vector-stores/collections
// ─────────────────────────────────────────────────────────────────────────────
pub async fn list_collections(State(_state): State<AppState>) -> impl IntoResponse {
    let client = get_qdrant_client(5);
    let url = format!("{}/collections", get_qdrant_url().trim_end_matches('/'));

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to fetch collections from Qdrant: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Failed to connect to Qdrant: {}", e) })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!("Qdrant collections API returned error: {}", err_body);
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Qdrant returned error: {}", err_body) })),
        )
            .into_response();
    }

    let body: Value = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to parse Qdrant response: {}", e) })),
            )
                .into_response();
        }
    };

    let collections_arr = match body
        .get("result")
        .and_then(|r| r.get("collections"))
        .and_then(|c| c.as_array())
    {
        Some(arr) => arr,
        None => {
            return Json(json!({ "collections": [] })).into_response();
        }
    };

    let mut collections = Vec::new();
    for item in collections_arr {
        if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
            let detail_url = format!(
                "{}/collections/{}",
                get_qdrant_url().trim_end_matches('/'),
                name
            );
            let mut point_count = 0;
            let mut vector_size = 0;
            let mut distance = "Cosine".to_string();
            let mut status = "unknown".to_string();

            if let Ok(d_resp) = client.get(&detail_url).send().await {
                if d_resp.status().is_success() {
                    if let Ok(d_body) = d_resp.json::<Value>().await {
                        if let Some(res) = d_body.get("result") {
                            status = res
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("green")
                                .to_string();
                            point_count = res
                                .get("points_count")
                                .and_then(|p| p.as_u64())
                                .unwrap_or(0);

                            if let Some(config) = res
                                .get("config")
                                .and_then(|c| c.get("params"))
                                .and_then(|p| p.get("vectors"))
                            {
                                vector_size =
                                    config.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                distance = config
                                    .get("distance")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("Cosine")
                                    .to_string();
                            }
                        }
                    }
                }
            }

            collections.push(json!({
                "name": name,
                "status": status,
                "points_count": point_count,
                "vector_size": vector_size,
                "distance": distance,
            }));
        }
    }

    Json(json!({ "collections": collections })).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/vector-stores/collections
// ─────────────────────────────────────────────────────────────────────────────
pub async fn create_collection(
    State(_state): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> impl IntoResponse {
    if req.name.is_empty()
        || !req
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid collection name. Only alphanumeric, '_' and '-' are allowed." })),
        )
            .into_response();
    }

    let client = get_qdrant_client(5);
    let url = format!(
        "{}/collections/{}",
        get_qdrant_url().trim_end_matches('/'),
        req.name
    );

    let body = json!({
        "vectors": {
            "size": req.vector_size,
            "distance": req.distance
        }
    });

    let resp = match client.put(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to create collection: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Failed to connect to Qdrant: {}", e) })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Qdrant returned error: {}", err_body) })),
        )
            .into_response();
    }

    Json(json!({ "success": true, "message": format!("Collection '{}' created successfully", req.name) }))
        .into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// DELETE /api/vector-stores/collections/:name
// ─────────────────────────────────────────────────────────────────────────────
pub async fn delete_collection(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let client = get_qdrant_client(5);
    let url = format!(
        "{}/collections/{}",
        get_qdrant_url().trim_end_matches('/'),
        name
    );

    let resp = match client.delete(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to delete collection: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Failed to connect to Qdrant: {}", e) })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Qdrant returned error: {}", err_body) })),
        )
            .into_response();
    }

    Json(json!({ "success": true, "message": format!("Collection '{}' deleted successfully", name) }))
        .into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/vector-stores/collections/:name/points
// ─────────────────────────────────────────────────────────────────────────────
pub async fn add_document(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<AddDocumentRequest>,
) -> impl IntoResponse {
    if req.text.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Document text cannot be empty" })),
        )
            .into_response();
    }

    let embed_req = EmbeddingRequest {
        model: req.embedding_model.clone(),
        input: EmbeddingInput::Single(req.text.clone()),
        encoding_format: None,
        dimensions: None,
        user: None,
    };

    let ctx = RequestContext::default();
    let embed_resp = match state.orchestrator.embed(embed_req, ctx).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to generate embedding: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to generate embedding: {}", e) })),
            )
                .into_response();
        }
    };

    let embedding_data = match embed_resp.data.first() {
        Some(data) => &data.embedding,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No embedding generated by provider" })),
            )
                .into_response();
        }
    };

    let mut payload = req.payload.unwrap_or_else(|| json!({}));
    if !payload.is_object() {
        payload = json!({});
    }
    let payload_obj = payload.as_object_mut().unwrap();
    payload_obj.insert("content".to_string(), Value::String(req.text.clone()));

    let client = get_qdrant_client(10);
    let url = format!(
        "{}/collections/{}/points",
        get_qdrant_url().trim_end_matches('/'),
        name
    );

    let point_id = format!("doc-{}", fastrand::u64(..));

    let body = json!({
        "points": [
            {
                "id": point_id,
                "vector": embedding_data,
                "payload": payload_obj
            }
        ]
    });

    let resp = match client.put(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to upload point to Qdrant: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Failed to connect to Qdrant: {}", e) })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Qdrant returned error: {}", err_body) })),
        )
            .into_response();
    }

    Json(json!({
        "success": true,
        "point_id": point_id,
        "message": "Document indexed successfully"
    }))
    .into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/vector-stores/collections/:name/search
// ─────────────────────────────────────────────────────────────────────────────
pub async fn search_collection(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SearchCollectionRequest>,
) -> impl IntoResponse {
    if req.query.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Query cannot be empty" })),
        )
            .into_response();
    }

    let embed_req = EmbeddingRequest {
        model: req.embedding_model.clone(),
        input: EmbeddingInput::Single(req.query.clone()),
        encoding_format: None,
        dimensions: None,
        user: None,
    };

    let ctx = RequestContext::default();
    let embed_resp = match state.orchestrator.embed(embed_req, ctx).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to generate embedding: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to generate embedding: {}", e) })),
            )
                .into_response();
        }
    };

    let embedding_data = match embed_resp.data.first() {
        Some(data) => &data.embedding,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No embedding generated by provider" })),
            )
                .into_response();
        }
    };

    let client = get_qdrant_client(10);
    let url = format!(
        "{}/collections/{}/points/search",
        get_qdrant_url().trim_end_matches('/'),
        name
    );

    let body = json!({
        "vector": embedding_data,
        "limit": req.limit.unwrap_or(5),
        "with_payload": true
    });

    let resp = match client.post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to search Qdrant: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Failed to connect to Qdrant: {}", e) })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Qdrant returned error: {}", err_body) })),
        )
            .into_response();
    }

    let body_val: Value = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to parse Qdrant response: {}", e) })),
            )
                .into_response();
        }
    };

    let results = body_val.get("result").unwrap_or(&Value::Null).clone();
    Json(results).into_response()
}
