//! Linking devices, listing and revoking them, projects and the audit log.

use super::api::{ApiError, AppState, Principal, error, fill_random, require, require_local};
use crate::features::jobs::store::now_ms;
use crate::features::node_audit;
use axum::Extension;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use chxchx_protocol::{
    Device, DeviceId, ErrorCode, NewPairing, PairRequest, PairResponse, PairingOffer, Role,
};
use serde::Deserialize;
use serde_json::json;

fn parse<T: serde::de::DeserializeOwned>(body: &Bytes, what: &str) -> Result<T, ApiError> {
    serde_json::from_slice(body).map_err(|problem| {
        error(
            ErrorCode::InvalidRequest,
            format!("{what} no válida: {problem}"),
        )
    })
}

/// On the PC, with the local token: a one-use code for a new device.
pub(super) async fn offer(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    body: Bytes,
) -> Result<Json<PairingOffer>, ApiError> {
    require_local(&who)?;
    let request: NewPairing = if body.is_empty() {
        NewPairing {
            role: Role::Operator,
        }
    } else {
        parse(&body, "Petición")?
    };
    let offer = state
        .devices()
        .offer(request.role, now_ms(), &mut fill_random)?;
    state.audit(&who, "pairing_offered", json!({ "role": offer.role }));
    state.log(&format!(
        "código de vinculación creado para el rol {}",
        offer.role.as_str()
    ));
    Ok(Json(offer))
}

/// The new device, with the code it was shown. No token yet: the code is what proves it.
pub(super) async fn pair(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<PairResponse>, ApiError> {
    let request: PairRequest = parse(&body, "Petición")?;
    let result = state
        .devices()
        .pair(&request.code, &request.name, now_ms(), &mut fill_random);
    match result {
        Ok((device, token)) => {
            let who = Principal::Device(device.clone());
            state.audit(
                &who,
                "device_paired",
                json!({ "device": device.id, "name": device.name, "role": device.role }),
            );
            state.log(&format!("dispositivo vinculado: {}", device.id));
            Ok(Json(PairResponse { device, token }))
        }
        Err(problem) => {
            // The code that was tried is never written down.
            state.log(&format!("vinculación rechazada ({:?})", problem.code));
            if problem.code == ErrorCode::Unauthorized {
                node_audit::record(&state.dir, "desconocido", "pairing_failed", json!({}));
            }
            Err(problem.into())
        }
    }
}

pub(super) async fn list(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Result<Json<Vec<Device>>, ApiError> {
    require(&who, Role::Admin)?;
    Ok(Json(state.devices().list()))
}

pub(super) async fn revoke(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<Device>, ApiError> {
    require(&who, Role::Admin)?;
    let id = DeviceId::parse(id)
        .map_err(|problem| error(ErrorCode::InvalidRequest, problem.to_string()))?;
    let device = state.devices().revoke(&id)?;
    state.audit(
        &who,
        "device_revoked",
        json!({ "device": device.id, "name": device.name }),
    );
    state.log(&format!("dispositivo revocado: {}", device.id));
    Ok(Json(device))
}

#[derive(Deserialize)]
pub(super) struct AuditQuery {
    limit: Option<usize>,
}

pub(super) async fn audit(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<node_audit::Entry>>, ApiError> {
    require(&who, Role::Admin)?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    Ok(Json(node_audit::read(&state.dir, limit)))
}
