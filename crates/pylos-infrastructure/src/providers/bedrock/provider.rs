use std::sync::Arc;

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use pylos_core::domain::config::BedrockKeyConfig;
use pylos_core::domain::provider::ProviderConfig;
use pylos_core::domain::request::{PylosRequest, PylosResponse};
use pylos_core::domain::traits::{ChunkStream, Provider};
use pylos_core::error::PylosError;

use super::converters::{
    build_inference_config, build_tool_config, convert_messages, from_bedrock_response,
    generate_completion_id, make_stream_chunk, make_tool_stream_chunk, map_stop_reason,
    normalize_model_id,
};

// ─────────────────────────────────────────────────────────────────────────────
// Cache des clients Bedrock par région
// Évite de recréer le client (et recharger les credentials) à chaque requête
// Identique au pattern de bifrost avec sync.Pool / assumeRoleCredsCache
// ─────────────────────────────────────────────────────────────────────────────

type ClientCache = std::collections::HashMap<String, Arc<BedrockClient>>;

/// Provider AWS Bedrock — implémente le trait Provider via l'API Converse
/// Supporte :
///   - Credentials explicites (access_key + secret_key)
///   - Credentials implicites (IAM role, IRSA, env vars, ~/.aws/credentials)
///   - AssumeRole via STS
///   - Claude, Nova, Llama, Mistral, Titan… (tous les modèles Converse)
pub struct BedrockProvider {
    client_cache: Arc<RwLock<ClientCache>>,
}

impl BedrockProvider {
    pub fn new() -> Self {
        Self {
            client_cache: Arc::new(RwLock::new(ClientCache::new())),
        }
    }

    /// Construit ou retourne un client Bedrock mis en cache pour la région
    async fn get_client(
        &self,
        bedrock_cfg: &BedrockKeyConfig,
    ) -> Result<Arc<BedrockClient>, PylosError> {
        let region = bedrock_cfg.region.clone();

        // Lecture rapide depuis le cache
        {
            let cache = self.client_cache.read().await;
            if let Some(client) = cache.get(&region) {
                return Ok(client.clone());
            }
        }

        // Pas en cache — on construit le client
        let client = Arc::new(self.build_client(bedrock_cfg).await?);

        // Écriture dans le cache
        {
            let mut cache = self.client_cache.write().await;
            cache.insert(region.clone(), client.clone());
        }

        info!(region = %region, "Bedrock client initialized");
        Ok(client)
    }

    /// Construit un client Bedrock avec les credentials appropriés
    async fn build_client(
        &self,
        bedrock_cfg: &BedrockKeyConfig,
    ) -> Result<BedrockClient, PylosError> {
        let region = aws_config::meta::region::RegionProviderChain::first_try(
            aws_sdk_bedrockruntime::config::Region::new(bedrock_cfg.region.clone()),
        );

        let sdk_config = if let (Some(access_key_env), Some(secret_key_env)) =
            (&bedrock_cfg.access_key_id, &bedrock_cfg.secret_access_key)
        {
            // Mode credentials explicites
            let access_key = access_key_env.resolve().ok_or_else(|| {
                PylosError::InvalidRequest(
                    "Bedrock access_key_id is empty or env var not found".into(),
                )
            })?;
            let secret_key = secret_key_env.resolve().ok_or_else(|| {
                PylosError::InvalidRequest(
                    "Bedrock secret_access_key is empty or env var not found".into(),
                )
            })?;

            let session_token = bedrock_cfg.session_token.as_ref().and_then(|t| t.resolve());

            debug!(region = %bedrock_cfg.region, "Using explicit AWS credentials");

            let creds = Credentials::new(
                &access_key,
                &secret_key,
                session_token,
                None,
                "pylos-config",
            );

            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(SharedCredentialsProvider::new(creds))
                .load()
                .await
        } else {
            // Mode IAM / chaîne de credentials par défaut
            // (env AWS_*, profils ~/.aws, IMDS EC2, IRSA EKS, etc.)
            debug!(region = %bedrock_cfg.region, "Using default AWS credential chain");

            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await
        };

        // AssumeRole si role_arn configuré
        let sdk_config = if let Some(role_arn_env) = &bedrock_cfg.role_arn {
            if let Some(role_arn) = role_arn_env.resolve() {
                debug!(role_arn = %role_arn, "Assuming IAM role via STS");

                let sts_client = aws_sdk_sts::Client::new(&sdk_config);
                let mut assume = sts_client
                    .assume_role()
                    .role_arn(&role_arn)
                    .role_session_name(&bedrock_cfg.role_session_name);

                if let Some(ext_id_env) = &bedrock_cfg.external_id {
                    if let Some(ext_id) = ext_id_env.resolve() {
                        assume = assume.external_id(ext_id);
                    }
                }

                let assumed = assume.send().await.map_err(|e| PylosError::ProviderError {
                    provider: "bedrock".into(),
                    message: format!("STS AssumeRole failed: {}", e),
                })?;

                let creds_output =
                    assumed
                        .credentials()
                        .ok_or_else(|| PylosError::ProviderError {
                            provider: "bedrock".into(),
                            message: "STS AssumeRole returned no credentials".into(),
                        })?;

                let assumed_creds = Credentials::new(
                    creds_output.access_key_id(),
                    creds_output.secret_access_key(),
                    Some(creds_output.session_token().to_string()),
                    None,
                    "pylos-assume-role",
                );

                aws_config::defaults(BehaviorVersion::latest())
                    .region(aws_config::meta::region::RegionProviderChain::first_try(
                        aws_sdk_bedrockruntime::config::Region::new(bedrock_cfg.region.clone()),
                    ))
                    .credentials_provider(SharedCredentialsProvider::new(assumed_creds))
                    .load()
                    .await
            } else {
                sdk_config
            }
        } else {
            sdk_config
        };

        Ok(BedrockClient::new(&sdk_config))
    }

    /// Extrait la BedrockKeyConfig depuis le ProviderConfig runtime
    fn get_bedrock_cfg<'a>(
        &self,
        config: &'a ProviderConfig,
    ) -> Result<&'a BedrockKeyConfig, PylosError> {
        config.bedrock.as_ref().ok_or_else(|| {
            PylosError::InvalidRequest(
                "Bedrock provider requires bedrock_key_config in pylos.json".into(),
            )
        })
    }
}

impl Default for BedrockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn name(&self) -> &str {
        "bedrock"
    }

    async fn complete(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<PylosResponse, PylosError> {
        let bedrock_cfg = self.get_bedrock_cfg(config)?;
        let client = self.get_client(bedrock_cfg).await?;

        match request {
            PylosRequest::ChatCompletion(req) => {
                let model_id = normalize_model_id(&req.model);
                let (messages, system_blocks) = convert_messages(&req.messages)?;
                let inference_config = build_inference_config(req);

                debug!(
                    model = model_id,
                    region = %bedrock_cfg.region,
                    messages = messages.len(),
                    "Sending Bedrock Converse request"
                );

                let mut call = client
                    .converse()
                    .model_id(model_id)
                    .inference_config(inference_config);

                for msg in messages {
                    call = call.messages(msg);
                }
                for sys in system_blocks {
                    call = call.system(sys);
                }

                // Ajouter les tools si présents
                if let Some(tools) = &req.tools {
                    if !tools.is_empty() {
                        match build_tool_config(tools) {
                            Ok(tc) => call = call.tool_config(tc),
                            Err(e) => {
                                warn!(model = model_id, error = %e, "Failed to build tool config")
                            }
                        }
                    }
                }

                let response = call.send().await.map_err(|e| {
                    let msg = e.to_string();
                    warn!(model = model_id, error = %msg, "Bedrock Converse error");
                    map_bedrock_sdk_error(&msg)
                })?;

                let output_msg = response
                    .output()
                    .and_then(|o| o.as_message().ok())
                    .ok_or_else(|| {
                        PylosError::Internal("Bedrock returned no output message".into())
                    })?;

                let stop_reason = response.stop_reason().as_str();

                let usage = response.usage();

                let id = generate_completion_id();

                info!(
                    model = model_id,
                    stop_reason = stop_reason,
                    "Bedrock Converse successful"
                );

                Ok(from_bedrock_response(
                    output_msg,
                    stop_reason,
                    usage,
                    &req.model,
                    id,
                ))
            }
            PylosRequest::TextCompletion(_) | PylosRequest::Embedding(_) => {
                Err(PylosError::Unsupported(
                    "Use Bedrock Titan Embeddings endpoint directly for embeddings".into(),
                ))
            }
        }
    }

    async fn stream(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError> {
        let bedrock_cfg = self.get_bedrock_cfg(config)?;
        let client = self.get_client(bedrock_cfg).await?;

        match request {
            PylosRequest::ChatCompletion(req) => {
                let model_id = normalize_model_id(&req.model).to_string();
                let (messages, system_blocks) = convert_messages(&req.messages)?;
                let inference_config = build_inference_config(req);

                debug!(
                    model = %model_id,
                    region = %bedrock_cfg.region,
                    "Sending Bedrock ConverseStream request"
                );

                let mut call = client
                    .converse_stream()
                    .model_id(&model_id)
                    .inference_config(inference_config);

                for msg in messages {
                    call = call.messages(msg);
                }
                for sys in system_blocks {
                    call = call.system(sys);
                }

                // Ajouter les tools si présents
                if let Some(tools) = &req.tools {
                    if !tools.is_empty() {
                        match build_tool_config(tools) {
                            Ok(tc) => call = call.tool_config(tc),
                            Err(e) => {
                                warn!(model = %model_id, error = %e, "Failed to build stream tool config")
                            }
                        }
                    }
                }

                let response = call.send().await.map_err(|e| {
                    let msg = e.to_string();
                    warn!(model = %model_id, error = %msg, "Bedrock ConverseStream setup error");
                    map_bedrock_sdk_error(&msg)
                })?;

                let id = generate_completion_id();
                let model_clone = req.model.clone();

                // Consomme l'EventStream SDK et l'adapte en Stream<StreamChunk>
                let event_stream = response.stream;

                let stream = async_stream::stream! {
                    // Premier chunk : role assistant
                    yield Ok(make_stream_chunk(
                        &id,
                        &model_clone,
                        None,
                        Some("assistant".into()),
                        None,
                    ));

                    let mut event_stream = event_stream;
                    // Accumulateur pour les tool calls en cours de streaming
                    let mut current_tool_id = String::new();
                    let mut current_tool_name = String::new();
                    let mut current_tool_index: i32 = 0;

                    loop {
                        match event_stream.recv().await {
                            Ok(Some(event)) => {
                                use aws_sdk_bedrockruntime::types::ConverseStreamOutput;
                                match event {
                                    ConverseStreamOutput::ContentBlockStart(start_event) => {
                                        // Début d'un bloc — on retient les métadonnées tool_use
                                        if let Some(start) = start_event.start() {
                                            use aws_sdk_bedrockruntime::types::ContentBlockStart;
                                            if let ContentBlockStart::ToolUse(tu) = start {
                                                current_tool_id = tu.tool_use_id().to_string();
                                                current_tool_name = tu.name().to_string();
                                                current_tool_index = start_event.content_block_index();
                                                // Émet le chunk d'annonce de l'outil
                                                yield Ok(make_tool_stream_chunk(
                                                    &id,
                                                    &model_clone,
                                                    pylos_core::domain::request::StreamToolCallChunk {
                                                        index: current_tool_index,
                                                        id: Some(current_tool_id.clone()),
                                                        call_type: Some("function".into()),
                                                        function: Some(pylos_core::domain::request::StreamToolCallFunction {
                                                            name: Some(current_tool_name.clone()),
                                                            arguments: None,
                                                        }),
                                                    },
                                                ));
                                            }
                                        }
                                    }
                                    ConverseStreamOutput::ContentBlockDelta(delta_event) => {
                                        if let Some(delta) = delta_event.delta() {
                                            use aws_sdk_bedrockruntime::types::ContentBlockDelta;
                                            match delta {
                                                ContentBlockDelta::Text(text) => {
                                                    yield Ok(make_stream_chunk(
                                                        &id,
                                                        &model_clone,
                                                        Some(text.clone()),
                                                        None,
                                                        None,
                                                    ));
                                                }
                                                ContentBlockDelta::ToolUse(tu_delta) => {
                                                    // Fragment JSON des arguments de l'outil
                                                    yield Ok(make_tool_stream_chunk(
                                                        &id,
                                                        &model_clone,
                                                        pylos_core::domain::request::StreamToolCallChunk {
                                                            index: current_tool_index,
                                                            id: None,
                                                            call_type: None,
                                                            function: Some(pylos_core::domain::request::StreamToolCallFunction {
                                                                name: None,
                                                                arguments: Some(tu_delta.input().to_string()),
                                                            }),
                                                        },
                                                    ));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    ConverseStreamOutput::MessageStop(stop_event) => {
                                        let reason = map_stop_reason(
                                            stop_event.stop_reason().as_str()
                                        ).to_string();
                                        yield Ok(make_stream_chunk(
                                            &id,
                                            &model_clone,
                                            None,
                                            None,
                                            Some(reason),
                                        ));
                                        break;
                                    }
                                    ConverseStreamOutput::MessageStart(_) => {
                                        // Déjà émis role=assistant au début
                                    }
                                    ConverseStreamOutput::ContentBlockStop(_) => {
                                        // Fin d'un bloc — rien à émettre
                                    }
                                    ConverseStreamOutput::Metadata(_) => {
                                        // Usage tokens — disponible en fin de stream
                                    }
                                    _ => {
                                        // Événement inconnu — ignoré
                                    }
                                }
                            }
                            Ok(None) => {
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "Bedrock stream error");
                                yield Err(PylosError::ProviderError {
                                    provider: "bedrock".into(),
                                    message: e.to_string(),
                                });
                                break;
                            }
                        }
                    }
                };

                Ok(Box::pin(stream))
            }
            PylosRequest::TextCompletion(_) | PylosRequest::Embedding(_) => {
                Err(PylosError::Unsupported(
                    "Use Bedrock Titan Embeddings endpoint directly for embeddings".into(),
                ))
            }
        }
    }

    async fn health_check(&self, config: &ProviderConfig) -> Result<(), PylosError> {
        let bedrock_cfg = self.get_bedrock_cfg(config)?;
        // Tente d'initialiser le client — valide les credentials
        self.get_client(bedrock_cfg).await?;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mapping des erreurs SDK AWS → PylosError
// ─────────────────────────────────────────────────────────────────────────────

fn map_bedrock_sdk_error(msg: &str) -> PylosError {
    let lower = msg.to_lowercase();

    if lower.contains("throttling") || lower.contains("too many requests") {
        PylosError::RateLimitExceeded(msg.to_string())
    } else if lower.contains("timeout") || lower.contains("timed out") {
        PylosError::Timeout(msg.to_string())
    } else if lower.contains("unauthorized")
        || lower.contains("invalid signature")
        || lower.contains("access denied")
        || lower.contains("forbidden")
    {
        PylosError::Unauthorized(msg.to_string())
    } else if lower.contains("model not found") || lower.contains("no such model") {
        PylosError::NotFound(msg.to_string())
    } else {
        PylosError::ProviderError {
            provider: "bedrock".into(),
            message: msg.to_string(),
        }
    }
}
