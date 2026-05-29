use async_trait::async_trait;
use tracing::{error, info, warn};

use pylos_core::domain::openai::{ChatCompletionMessage, MessageRole};
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;

pub struct RagPlugin {
    qdrant_url: String,
    collection_name: String,
    pylos_base_url: String,
    pylos_api_key: Option<String>,
    embedding_model: String,
    pylos_model: String,
    client: reqwest::Client,
}

impl RagPlugin {
    pub fn new(
        qdrant_url: String,
        collection_name: String,
        pylos_base_url: String,
        pylos_api_key: Option<String>,
        embedding_model: String,
        pylos_model: String,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            qdrant_url,
            collection_name,
            pylos_base_url,
            pylos_api_key,
            embedding_model,
            pylos_model,
            client,
        }
    }
}

#[async_trait]
impl LlmPlugin for RagPlugin {
    fn name(&self) -> &str {
        "rag"
    }

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        _ctx: &mut RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let chat_req = match request {
            PylosRequest::ChatCompletion(ref req) => req,
            _ => return Ok(None),
        };

        let (collection_name, is_email) = match chat_req.model.as_str() {
            "graphon-rag" | "graphon-rag-emails" => (self.collection_name.clone(), true),
            "graphon-rag-files" | "graphon-rag-docs" | "mnemosyne-search" => {
                let col = std::env::var("QDRANT_FILES_COLLECTION")
                    .unwrap_or_else(|_| "mnemosyne_docs".to_string());
                (col, false)
            }
            _ => return Ok(None),
        };

        info!(
            "RagPlugin: Intercepted {} request (targeting collection: {})",
            chat_req.model, collection_name
        );

        // 1. Extract the latest user query
        let user_query = chat_req
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, MessageRole::User))
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        if user_query.is_empty() {
            return Ok(None);
        }

        // 2. Fetch query embedding via Pylos' own /v1/embeddings endpoint
        let embed_url = format!(
            "{}/v1/embeddings",
            self.pylos_base_url.trim_end_matches('/')
        );
        let embed_body = serde_json::json!({
            "model": self.embedding_model,
            "input": user_query
        });

        let mut embed_req = self.client.post(&embed_url).json(&embed_body);
        if let Some(ref key) = self.pylos_api_key {
            embed_req = embed_req.header("Authorization", format!("Bearer {}", key));
        }

        let embed_resp = embed_req.send().await.map_err(|e| {
            error!(
                "RagPlugin: Failed to connect to Pylos for embedding: {:?}",
                e
            );
            PylosError::Internal(format!("Failed to connect to Pylos for embedding: {}", e))
        })?;

        if !embed_resp.status().is_success() {
            let err = embed_resp.text().await.unwrap_or_default();
            error!("RagPlugin: Pylos embedding API returned error: {}", err);
            return Err(PylosError::Internal(format!(
                "Pylos embedding error: {}",
                err
            )));
        }

        #[derive(serde::Deserialize)]
        struct PylosEmbeddingData {
            embedding: Vec<f32>,
        }
        #[derive(serde::Deserialize)]
        struct PylosEmbeddingResponse {
            data: Vec<PylosEmbeddingData>,
        }

        let embed_data: PylosEmbeddingResponse = embed_resp.json().await.map_err(|e| {
            error!("RagPlugin: Failed to parse embedding response: {:?}", e);
            PylosError::Internal(format!("Failed to parse embedding response: {}", e))
        })?;

        let query_vector = match embed_data.data.first() {
            Some(d) => &d.embedding,
            None => {
                error!("RagPlugin: Empty embedding returned from Pylos");
                return Err(PylosError::Internal("Empty embedding returned".into()));
            }
        };

        // 3. Query Qdrant directly
        let search_url = format!(
            "{}/collections/{}/points/search",
            self.qdrant_url.trim_end_matches('/'),
            collection_name
        );
        let search_body = serde_json::json!({
            "vector": query_vector,
            "limit": 5,
            "with_payload": true
        });

        let search_resp = self
            .client
            .post(&search_url)
            .json(&search_body)
            .send()
            .await
            .map_err(|e| {
                error!("RagPlugin: Failed to connect to Qdrant: {:?}", e);
                PylosError::Internal(format!("Failed to connect to Qdrant: {}", e))
            })?;

        let mut context_text = String::new();
        if search_resp.status().is_success() {
            #[derive(serde::Deserialize)]
            struct QdrantResponse {
                result: Vec<QdrantPoint>,
            }
            #[derive(serde::Deserialize)]
            struct QdrantPoint {
                payload: Option<serde_json::Value>,
            }

            if let Ok(res) = search_resp.json::<QdrantResponse>().await {
                if !res.result.is_empty() {
                    if is_email {
                        context_text.push_str("Voici les emails pertinents trouvés dans les archives publiques pour vous aider à répondre:\n\n");
                        for (i, point) in res.result.iter().enumerate() {
                            if let Some(ref payload) = point.payload {
                                let sender = payload
                                    .get("sender")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Inconnu");
                                let subject = payload
                                    .get("subject")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Sans objet");
                                let content = payload
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                context_text.push_str(&format!(
                                    "--- EMAIL #{i} ---\nDe: {sender}\nObjet: {subject}\nContenu:\n{content}\n\n",
                                    i = i + 1,
                                    sender = sender,
                                    subject = subject,
                                    content = content
                                ));
                            }
                        }
                    } else {
                        context_text.push_str("Voici les documents pertinents trouvés dans les archives de fichiers pour vous aider à répondre:\n\n");
                        for (i, point) in res.result.iter().enumerate() {
                            if let Some(ref payload) = point.payload {
                                let file_name = payload
                                    .get("file_name")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| {
                                        payload
                                            .get("metadata")
                                            .and_then(|m| m.get("file_name"))
                                            .and_then(|v| v.as_str())
                                    })
                                    .unwrap_or("Inconnu");
                                let source_path = payload
                                    .get("source_path")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| {
                                        payload
                                            .get("metadata")
                                            .and_then(|m| m.get("source_path"))
                                            .and_then(|v| v.as_str())
                                    })
                                    .unwrap_or("Inconnu");
                                let content = payload
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                context_text.push_str(&format!(
                                    "--- DOCUMENT #{i} ---\nFichier: {file_name}\nChemin: {source_path}\nContenu:\n{content}\n\n",
                                    i = i + 1,
                                    file_name = file_name,
                                    source_path = source_path,
                                    content = content
                                ));
                            }
                        }
                    }
                }
            }
        } else {
            warn!(
                "RagPlugin: Qdrant search returned status: {}",
                search_resp.status()
            );
        }

        // 4. Augment message list and delegate back to Pylos /v1/chat/completions
        let mut outbound_messages = Vec::new();
        if !context_text.is_empty() {
            let system_prompt = if is_email {
                format!(
                    "Tu es un assistant d'archivage d'emails. Utilise les emails pertinents suivants pour répondre à l'utilisateur de manière précise, concise et en français:\n\n{}",
                    context_text
                )
            } else {
                format!(
                    "Tu es un assistant de recherche de documents. Utilise les documents pertinents suivants pour répondre à l'utilisateur de manière précise, concise et en français:\n\n{}",
                    context_text
                )
            };
            outbound_messages.push(ChatCompletionMessage {
                role: MessageRole::System,
                content: Some(system_prompt),
                ..Default::default()
            });
        }

        for msg in &chat_req.messages {
            outbound_messages.push(msg.clone());
        }

        let chat_url = format!(
            "{}/v1/chat/completions",
            self.pylos_base_url.trim_end_matches('/')
        );
        let chat_body = serde_json::json!({
            "model": self.pylos_model,
            "messages": outbound_messages,
            "stream": false
        });

        let mut chat_req_builder = self.client.post(&chat_url).json(&chat_body);
        if let Some(ref key) = self.pylos_api_key {
            chat_req_builder = chat_req_builder.header("Authorization", format!("Bearer {}", key));
        }

        let chat_resp = chat_req_builder.send().await.map_err(|e| {
            error!(
                "RagPlugin: Failed to connect to Pylos for chat completion: {:?}",
                e
            );
            PylosError::Internal(format!(
                "Failed to connect to Pylos for chat completion: {}",
                e
            ))
        })?;

        if !chat_resp.status().is_success() {
            let err = chat_resp.text().await.unwrap_or_default();
            error!(
                "RagPlugin: Pylos chat completion API returned error: {}",
                err
            );
            return Err(PylosError::Internal(format!(
                "Pylos chat completion error: {}",
                err
            )));
        }

        let completion_data: pylos_core::domain::openai::ChatCompletionResponse =
            chat_resp.json().await.map_err(|e| {
                error!(
                    "RagPlugin: Failed to parse chat completion response: {:?}",
                    e
                );
                PylosError::Internal(format!("Failed to parse chat completion response: {}", e))
            })?;

        Ok(Some(PylosResponse::ChatCompletion(completion_data)))
    }
}
