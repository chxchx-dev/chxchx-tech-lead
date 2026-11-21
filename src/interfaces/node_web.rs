//! The web client the node serves (`apps/remote`, embedded at build time).
//!
//! These files are public: the page has to load before a device is linked, so there is nothing
//! secret in them and no token is asked. What protects the node is the `Host` check every request
//! passes, the content security policy below and the token every API route needs. The files are
//! kept gzip-compressed and served as they are; every browser accepts gzip.

use super::api::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::Response;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/remote_assets.rs"));

pub(crate) struct Asset {
    body: &'static [u8],
    content_type: &'static str,
    etag: String,
}

#[derive(Default)]
pub(crate) struct Assets {
    items: BTreeMap<String, Asset>,
}

impl Assets {
    /// The client built into this binary (empty when it was built without one).
    pub fn embedded() -> Self {
        Self::from(REMOTE_APP)
    }

    pub fn from(table: &[(&str, &'static [u8])]) -> Self {
        let items = table
            .iter()
            .map(|(path, body)| {
                let digest = Sha256::digest(body);
                let etag: String = digest
                    .iter()
                    .take(8)
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                (
                    (*path).to_owned(),
                    Asset {
                        body,
                        content_type: content_type(path),
                        etag: format!("\"{etag}\""),
                    },
                )
            })
            .collect();
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "webmanifest" => "application/manifest+json",
        "json" => "application/json",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// What the page may load and talk to. The host comes from the request, which the guard already
/// checked, so a page served on the Tailscale name may only open a socket back to that name.
pub(crate) fn content_security_policy(host: &str) -> String {
    format!(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; \
         img-src 'self' data:; font-src 'self'; manifest-src 'self'; worker-src 'self'; \
         connect-src 'self' ws://{host} wss://{host}; base-uri 'none'; form-action 'none'; \
         frame-ancestors 'none'"
    )
}

const NOT_BUILT: &str = "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\">\
<title>ChxChx</title><body style=\"font:16px system-ui;margin:2rem;max-width:34rem\">\
<h1>El nodo está en marcha</h1>\
<p>Esta compilación no incluye la app web. Constrúyela con <code>scripts/build-remote.sh</code> \
y vuelve a compilar el nodo (<code>cargo build --features node</code>).</p>\
<p>El API sigue disponible en <code>/api/v1</code>.</p>";

/// `GET` and `HEAD` for everything that is not the API.
pub(super) async fn serve(
    State(state): State<AppState>,
    method: Method,
    uri: axum::http::Uri,
    headers: HeaderMap,
) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return plain(StatusCode::METHOD_NOT_ALLOWED, "Método no permitido");
    }
    let path = match uri.path() {
        "/" => "/index.html",
        other => other,
    };
    if state.assets.is_empty() {
        let mut response = if path == "/index.html" {
            html(StatusCode::SERVICE_UNAVAILABLE, NOT_BUILT)
        } else {
            plain(StatusCode::NOT_FOUND, "No encontrado")
        };
        security(&mut response, &headers);
        return response;
    }
    let Some(asset) = state.assets.items.get(path) else {
        return plain(StatusCode::NOT_FOUND, "No encontrado");
    };
    if !accepts_gzip(&headers) {
        return plain(
            StatusCode::NOT_ACCEPTABLE,
            "Este navegador no acepta contenido comprimido (gzip)",
        );
    }
    let unchanged = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|given| given.split(',').any(|etag| etag.trim() == asset.etag));
    let mut response = Response::new(if unchanged || method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(asset.body)
    });
    *response.status_mut() = if unchanged {
        StatusCode::NOT_MODIFIED
    } else {
        StatusCode::OK
    };
    let out = response.headers_mut();
    set(out, header::CONTENT_TYPE, asset.content_type);
    set(out, header::CONTENT_ENCODING, "gzip");
    set(out, header::ETAG, &asset.etag);
    set(out, header::VARY, "Accept-Encoding");
    // Always revalidate: the ETag makes it a 304, and a new build is picked up at once.
    set(out, header::CACHE_CONTROL, "no-cache");
    if path == "/sw.js" {
        set(
            out,
            header::HeaderName::from_static("service-worker-allowed"),
            "/",
        );
    }
    security(&mut response, &headers);
    response
}

/// Headers every response carries; the policy needs the `Host` the page was reached on.
fn security(response: &mut Response, request: &HeaderMap) {
    let host = request
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("127.0.0.1");
    let out = response.headers_mut();
    set(
        out,
        header::CONTENT_SECURITY_POLICY,
        &content_security_policy(host),
    );
}

fn set(headers: &mut HeaderMap, name: header::HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

/// Whether the browser accepts gzip (`gzip` or `*`, and not refused with `q=0`).
fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|list| {
            list.split(',').any(|item| {
                let item = item.trim().to_ascii_lowercase();
                let mut parts = item.split(';');
                let coding = parts.next().unwrap_or_default().trim();
                let refused = parts.any(|part| {
                    part.trim()
                        .strip_prefix("q=")
                        .and_then(|quality| quality.trim().parse::<f32>().ok())
                        .is_some_and(|quality| quality == 0.0)
                });
                (coding == "gzip" || coding == "*") && !refused
            })
        })
}

fn plain(status: StatusCode, text: &'static str) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    set(
        response.headers_mut(),
        header::CONTENT_TYPE,
        "text/plain; charset=utf-8",
    );
    response
}

fn html(status: StatusCode, text: &'static str) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    set(
        response.headers_mut(),
        header::CONTENT_TYPE,
        "text/html; charset=utf-8",
    );
    response
}

/// Added to every response, API or page.
pub(super) async fn harden(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    let out = response.headers_mut();
    for (name, value) in [
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (
            header::HeaderName::from_static("cross-origin-resource-policy"),
            "same-origin",
        ),
    ] {
        out.entry(name).or_insert(HeaderValue::from_static(value));
    }
    if api {
        // Answers about jobs and devices are never kept by a cache.
        out.entry(header::CACHE_CONTROL)
            .or_insert(HeaderValue::from_static("no-store"));
    }
    response
}
