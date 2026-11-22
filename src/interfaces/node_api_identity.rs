//! What each person made their own (ADR 0017): the name in block letters, the line under it, the
//! picture and the theme, served without a token. The pairing screen of a phone needs them before
//! the phone is linked, and they hold nothing private: the node only answers on the loopback or the
//! person's own tailnet, like the web client itself. They are only read here; they are changed
//! with `chxchx-tech identity` or in Studio.

use super::api::AppState;
use crate::features::identity;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chxchx_protocol::IdentityView;

/// A change must show on the next load: nothing here is kept by a cache without asking the node.
fn fresh(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

pub(super) async fn view(State(state): State<AppState>) -> Response {
    fresh(Json::<IdentityView>(identity::view_in(&state.home)).into_response())
}

pub(super) async fn avatar(State(state): State<AppState>) -> Response {
    match identity::avatar_in(&state.home) {
        Some(png) => fresh(([(header::CONTENT_TYPE, "image/png")], png).into_response()),
        None => (StatusCode::NOT_FOUND, "Sin imagen propia").into_response(),
    }
}

pub(super) async fn theme(State(state): State<AppState>) -> Response {
    fresh(
        (
            [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
            identity::css_in(&state.home),
        )
            .into_response(),
    )
}
