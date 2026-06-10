use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct AuthConfigResponse {
    pub google_auth_enabled: bool,
    pub google_client_id: Option<String>,
    pub google_redirect_uri: Option<String>,
}

pub async fn get_auth_config(State(state): State<AppState>) -> impl IntoResponse {
    let enabled = state.google_client_id.is_some() && state.google_client_secret.is_some();
    Json(AuthConfigResponse {
        google_auth_enabled: enabled,
        google_client_id: state.google_client_id.clone(),
        google_redirect_uri: state.google_redirect_uri.clone(),
    })
}

#[derive(Debug, Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PylosSessionClaims {
    pub sub: String, // email
    pub exp: usize,  // expiration timestamp (seconds)
    pub role: String,
    pub name: String,
    #[serde(default = "default_group")]
    pub group: String,
}

fn default_group() -> String {
    "default".to_string()
}

pub async fn google_callback(
    State(state): State<AppState>,
    Json(req): Json<CallbackRequest>,
) -> impl IntoResponse {
    let client_id = match &state.google_client_id {
        Some(cid) => cid,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Google OAuth is not configured on this server" })),
            )
                .into_response()
        }
    };
    let client_secret = match &state.google_client_secret {
        Some(sec) => sec,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Google OAuth client secret is not configured" })),
            )
                .into_response()
        }
    };

    // Construct or use the redirect URI. Prioritize:
    // 1. request redirect_uri (sent from frontend)
    // 2. state.google_redirect_uri (configured in server env)
    // 3. Fallback default
    let redirect_uri = req
        .redirect_uri
        .or_else(|| state.google_redirect_uri.clone())
        .unwrap_or_else(|| "https://pylos-dev.p.zacharie.org/callback".to_string());

    let client = reqwest::Client::new();

    // 1. Exchange authorization code for tokens
    let params = [
        ("code", req.code.as_str()),
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
    ];

    let token_resp =
        match client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to contact Google token endpoint: {}", e) })),
            )
                .into_response(),
        };

    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({ "error": format!("Google token exchange failed ({}): {}", status, body) }),
            ),
        )
            .into_response();
    }

    let token_data: GoogleTokenResponse = match token_resp.json().await {
        Ok(data) => data,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to parse Google token response: {}", e) })),
            )
                .into_response()
        }
    };

    // 2. Retrieve user info using access_token
    let user_info_resp = match client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to fetch user info from Google: {}", e) })),
            )
                .into_response()
        }
    };

    if !user_info_resp.status().is_success() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Google userinfo request failed" })),
        )
            .into_response();
    }

    let google_user: GoogleUserInfo = match user_info_resp.json().await {
        Ok(info) => info,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to parse Google user info: {}", e) })),
            )
                .into_response()
        }
    };

    // 3. Match Google email with InternalUser from OrganizationStore
    let users = match state.org_store.list_users().await {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database error listing users: {}", e) })),
            )
                .into_response()
        }
    };

    let matched_user = users
        .iter()
        .find(|u| u.email.to_lowercase() == google_user.email.to_lowercase());

    let (user_role, user_group) = match matched_user {
        Some(user) => {
            if !user.is_active {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": format!("User email {} is inactive in Pylos", google_user.email) })),
                )
                    .into_response();
            }
            (user.role.clone(), user.group.clone())
        }
        None => {
            let role = if users.is_empty() { "admin" } else { "member" };
            let group = if users.is_empty() {
                "default"
            } else {
                "playgroup"
            };
            // Generate a random ID using fastrand
            let random_id = (0..16)
                .map(|_| fastrand::alphanumeric())
                .collect::<String>();
            let new_user = pylos_core::domain::organization::InternalUser {
                id: format!("usr_{}", random_id),
                email: google_user.email.clone(),
                name: google_user
                    .name
                    .clone()
                    .unwrap_or_else(|| google_user.email.clone()),
                role: role.to_string(),
                group: group.to_string(),
                organization_id: None,
                team_ids: vec![],
                is_active: true,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                updated_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            };
            if let Err(e) = state.org_store.upsert_user(&new_user).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to auto-register user: {}", e) })),
                )
                    .into_response();
            }

            // Create a default virtual key with $1 budget
            let vk_id = format!("vk-{}", random_id);
            let vk_value = format!("sk-pylos-{}", fastrand::u64(..));
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let vk_cfg = pylos_core::domain::config::VirtualKeyConfig {
                id: vk_id.clone(),
                name: format!("Default Key for {}", google_user.email),
                description: Some("Automatically generated key on registration".to_string()),
                value: Some(pylos_core::domain::config::EnvVar::Literal(
                    vk_value.clone(),
                )),
                is_active: true,
                rate_limit_id: None,
                provider_configs: vec![],
                team_alias: None,
                team_id: None,
                organization_id: None,
                access_group_id: None,
                user_email: Some(google_user.email.clone()),
                user_id: Some(new_user.id.clone()),
                created_at: Some(now_secs * 1000),
                created_by: Some("auto-registration".to_string()),
                updated_at: Some(now_secs * 1000),
                last_active: None,
                expires_at: None,
            };

            if let Err(e) = state.vk_store.upsert_key(&vk_cfg).await {
                tracing::error!(error = %e, "Failed to create default virtual key for new user");
            } else {
                let vk = pylos_core::domain::virtual_key::VirtualKey::new(
                    vk_value.clone(),
                    &vk_cfg.name,
                );
                state.vk_registry.register(vk).await;
            }

            let budget_cfg = pylos_core::domain::config::BudgetConfig {
                id: format!("b-{}", random_id),
                max_limit: 1.0,
                reset_duration: pylos_core::domain::config::Duration("1M".to_string()),
                current_usage: 0.0,
                virtual_key_id: Some(vk_id.clone()),
            };
            if let Err(e) = state.budget_store.upsert_budget(&vk_id, &budget_cfg).await {
                tracing::error!(error = %e, "Failed to create default budget for new user VK");
            }

            (role.to_string(), group.to_string())
        }
    };

    // 4. Issue local session JWT
    let exp_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
        + 24 * 3600; // 24 hours validity

    let claims = PylosSessionClaims {
        sub: google_user.email.clone(),
        exp: exp_time,
        role: user_role.clone(),
        name: google_user
            .name
            .unwrap_or_else(|| google_user.email.clone()),
        group: user_group.clone(),
    };

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to sign JWT: {}", e) })),
            )
                .into_response()
        }
    };

    Json(json!({
        "token": token,
        "user": {
            "email": claims.sub,
            "name": claims.name,
            "role": claims.role,
            "group": claims.group,
        }
    }))
    .into_response()
}

pub async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Logged out successfully"
        })),
    )
}
