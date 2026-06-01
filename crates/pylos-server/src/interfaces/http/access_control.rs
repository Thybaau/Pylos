use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use pylos_core::domain::organization::{
    AccessGroup, InternalUser, Organization, Policy, Team, ToolPolicy,
};

use crate::state::AppState;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ── Organizations ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

pub async fn list_organizations(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_organizations().await {
        Ok(orgs) => Json(json!({ "organizations": orgs, "total": orgs.len() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_organization(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.get_organization(&id).await {
        Ok(Some(org)) => Json(json!(org)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Organization '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_organization(
    State(state): State<AppState>,
    Json(req): Json<CreateOrganizationRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let org = Organization {
        id: req
            .id
            .unwrap_or_else(|| format!("org-{}", fastrand::u32(..))),
        name: req.name,
        description: req.description,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_organization(&org).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(org))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn update_organization(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateOrganizationRequest>,
) -> impl IntoResponse {
    let mut org = match state.org_store.get_organization(&id).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Organization '{}' not found", id) })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    if let Some(name) = req.name {
        org.name = name;
    }
    if let Some(desc) = req.description {
        org.description = Some(desc);
    }
    if let Some(active) = req.is_active {
        org.is_active = active;
    }
    org.updated_at = now_ms();
    match state.org_store.upsert_organization(&org).await {
        Ok(()) => Json(json!(org)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_organization(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_organization(&id).await {
        Ok(true) => {
            Json(json!({ "message": format!("Organization '{}' deleted", id) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Organization '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Teams ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub id: Option<String>,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub organization_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn list_teams(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_teams(None).await {
        Ok(teams) => Json(json!({ "teams": teams, "total": teams.len() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_team(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.org_store.get_team(&id).await {
        Ok(Some(team)) => Json(json!(team)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Team '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_team(
    State(state): State<AppState>,
    Json(req): Json<CreateTeamRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let team = Team {
        id: req
            .id
            .unwrap_or_else(|| format!("team-{}", fastrand::u32(..))),
        organization_id: req.organization_id,
        name: req.name,
        description: req.description,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_team(&team).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(team))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_team(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTeamRequest>,
) -> impl IntoResponse {
    let mut team = match state.org_store.get_team(&id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Team '{}' not found", id) })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    if let Some(oid) = req.organization_id {
        team.organization_id = oid;
    }
    if let Some(name) = req.name {
        team.name = name;
    }
    if let Some(desc) = req.description {
        team.description = Some(desc);
    }
    if let Some(active) = req.is_active {
        team.is_active = active;
    }
    team.updated_at = now_ms();
    match state.org_store.upsert_team(&team).await {
        Ok(()) => Json(json!(team)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_team(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_team(&id).await {
        Ok(true) => Json(json!({ "message": format!("Team '{}' deleted", id) })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Team '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Internal Users ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub id: Option<String>,
    pub email: String,
    pub name: String,
    #[serde(default = "default_member_role")]
    pub role: String,
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_member_role() -> String {
    "member".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub organization_id: Option<String>,
    pub team_ids: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

pub async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_users().await {
        Ok(users) => Json(json!({ "users": users, "total": users.len() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_user(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.org_store.get_user(&id).await {
        Ok(Some(user)) => Json(json!(user)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("User '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let user = InternalUser {
        id: req
            .id
            .unwrap_or_else(|| format!("user-{}", fastrand::u32(..))),
        email: req.email,
        name: req.name,
        role: req.role,
        organization_id: req.organization_id,
        team_ids: req.team_ids,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_user(&user).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(user))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let mut user = match state.org_store.get_user(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("User '{}' not found", id) })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    if let Some(email) = req.email {
        user.email = email;
    }
    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(role) = req.role {
        user.role = role;
    }
    if let Some(oid) = req.organization_id {
        user.organization_id = Some(oid);
    }
    if let Some(tids) = req.team_ids {
        user.team_ids = tids;
    }
    if let Some(active) = req.is_active {
        user.is_active = active;
    }
    user.updated_at = now_ms();
    match state.org_store.upsert_user(&user).await {
        Ok(()) => Json(json!(user)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_user(&id).await {
        Ok(true) => Json(json!({ "message": format!("User '{}' deleted", id) })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("User '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Access Groups ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAccessGroupRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_ids: Vec<String>,
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub model_ids: Vec<String>,
    #[serde(default)]
    pub provider_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccessGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub organization_id: Option<String>,
    pub team_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub model_ids: Option<Vec<String>>,
    pub provider_ids: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

pub async fn list_access_groups(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_access_groups().await {
        Ok(groups) => {
            Json(json!({ "access_groups": groups, "total": groups.len() })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_access_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.get_access_group(&id).await {
        Ok(Some(ag)) => Json(json!(ag)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Access group '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_access_group(
    State(state): State<AppState>,
    Json(req): Json<CreateAccessGroupRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let ag = AccessGroup {
        id: req
            .id
            .unwrap_or_else(|| format!("ag-{}", fastrand::u32(..))),
        name: req.name,
        description: req.description,
        organization_id: req.organization_id,
        team_ids: req.team_ids,
        user_ids: req.user_ids,
        model_ids: req.model_ids,
        provider_ids: req.provider_ids,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_access_group(&ag).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(ag))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_access_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAccessGroupRequest>,
) -> impl IntoResponse {
    let mut ag = match state.org_store.get_access_group(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Access group '{}' not found", id) })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    if let Some(name) = req.name {
        ag.name = name;
    }
    if let Some(desc) = req.description {
        ag.description = Some(desc);
    }
    if let Some(oid) = req.organization_id {
        ag.organization_id = Some(oid);
    }
    if let Some(tids) = req.team_ids {
        ag.team_ids = tids;
    }
    if let Some(uids) = req.user_ids {
        ag.user_ids = uids;
    }
    if let Some(mids) = req.model_ids {
        ag.model_ids = mids;
    }
    if let Some(pids) = req.provider_ids {
        ag.provider_ids = pids;
    }
    if let Some(active) = req.is_active {
        ag.is_active = active;
    }
    ag.updated_at = now_ms();
    match state.org_store.upsert_access_group(&ag).await {
        Ok(()) => Json(json!(ag)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_access_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_access_group(&id).await {
        Ok(true) => {
            Json(json!({ "message": format!("Access group '{}' deleted", id) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Access group '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Policies ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub policy_type: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub policy_type: Option<String>,
    pub config: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

pub async fn list_policies(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_policies().await {
        Ok(policies) => {
            Json(json!({ "policies": policies, "total": policies.len() })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_policy(
    State(state): State<AppState>,
    Json(req): Json<CreatePolicyRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let policy = Policy {
        id: req
            .id
            .unwrap_or_else(|| format!("policy-{}", fastrand::u32(..))),
        name: req.name,
        description: req.description,
        policy_type: req.policy_type,
        config: req.config,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_policy(&policy).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(policy))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePolicyRequest>,
) -> impl IntoResponse {
    let existing = match state.org_store.list_policies().await {
        Ok(policies) => policies.into_iter().find(|p| p.id == id),
        Err(_) => None,
    };
    let mut policy = match existing {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Policy '{}' not found", id) })),
            )
                .into_response()
        }
    };
    if let Some(name) = req.name {
        policy.name = name;
    }
    if let Some(desc) = req.description {
        policy.description = Some(desc);
    }
    if let Some(pt) = req.policy_type {
        policy.policy_type = pt;
    }
    if let Some(cfg) = req.config {
        policy.config = cfg;
    }
    if let Some(active) = req.is_active {
        policy.is_active = active;
    }
    policy.updated_at = now_ms();
    match state.org_store.upsert_policy(&policy).await {
        Ok(()) => Json(json!(policy)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_policy(&id).await {
        Ok(true) => Json(json!({ "message": format!("Policy '{}' deleted", id) })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Policy '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Tool Policies ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateToolPolicyRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    #[serde(default)]
    pub allowed_models: Vec<String>,
    #[serde(default)]
    pub allowed_providers: Vec<String>,
    pub max_tokens_per_call: Option<i64>,
    pub max_calls_per_minute: Option<i64>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateToolPolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tool_type: Option<String>,
    pub allowed_models: Option<Vec<String>>,
    pub allowed_providers: Option<Vec<String>>,
    pub max_tokens_per_call: Option<Option<i64>>,
    pub max_calls_per_minute: Option<Option<i64>>,
    pub is_active: Option<bool>,
}

pub async fn list_tool_policies(State(state): State<AppState>) -> impl IntoResponse {
    match state.org_store.list_tool_policies().await {
        Ok(policies) => {
            Json(json!({ "tool_policies": policies, "total": policies.len() })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_tool_policy(
    State(state): State<AppState>,
    Json(req): Json<CreateToolPolicyRequest>,
) -> impl IntoResponse {
    let now = now_ms();
    let tp = ToolPolicy {
        id: req
            .id
            .unwrap_or_else(|| format!("tp-{}", fastrand::u32(..))),
        name: req.name,
        description: req.description,
        tool_type: req.tool_type,
        allowed_models: req.allowed_models,
        allowed_providers: req.allowed_providers,
        max_tokens_per_call: req.max_tokens_per_call,
        max_calls_per_minute: req.max_calls_per_minute,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    match state.org_store.upsert_tool_policy(&tp).await {
        Ok(()) => (StatusCode::CREATED, Json(json!(tp))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_tool_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateToolPolicyRequest>,
) -> impl IntoResponse {
    let existing = match state.org_store.list_tool_policies().await {
        Ok(policies) => policies.into_iter().find(|p| p.id == id),
        Err(_) => None,
    };
    let mut tp = match existing {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Tool policy '{}' not found", id) })),
            )
                .into_response()
        }
    };
    if let Some(name) = req.name {
        tp.name = name;
    }
    if let Some(desc) = req.description {
        tp.description = Some(desc);
    }
    if let Some(tt) = req.tool_type {
        tp.tool_type = tt;
    }
    if let Some(models) = req.allowed_models {
        tp.allowed_models = models;
    }
    if let Some(providers) = req.allowed_providers {
        tp.allowed_providers = providers;
    }
    if let Some(tokens) = req.max_tokens_per_call {
        tp.max_tokens_per_call = tokens;
    }
    if let Some(cpm) = req.max_calls_per_minute {
        tp.max_calls_per_minute = cpm;
    }
    if let Some(active) = req.is_active {
        tp.is_active = active;
    }
    tp.updated_at = now_ms();
    match state.org_store.upsert_tool_policy(&tp).await {
        Ok(()) => Json(json!(tp)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_tool_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.org_store.delete_tool_policy(&id).await {
        Ok(true) => {
            Json(json!({ "message": format!("Tool policy '{}' deleted", id) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Tool policy '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
