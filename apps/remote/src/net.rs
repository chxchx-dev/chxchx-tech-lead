//! The browser's side of the client: HTTP, the event socket, storage, timers and notifications.
//! Everything that needs `window` lives here, so the rest is plain Rust.

use chxchx_protocol::JobError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// How long the node gets to answer a request.
const REQUEST_TIMEOUT_MS: i32 = 20_000;

fn window() -> web_sys::Window {
    web_sys::window().expect("the client runs in a browser window")
}

#[derive(Clone, Debug)]
pub struct HttpError {
    /// 0 when the node could not be reached at all.
    pub status: u16,
    pub message: String,
}

impl HttpError {
    pub fn unreachable() -> Self {
        Self {
            status: 0,
            message: "Sin conexión con el nodo".into(),
        }
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == 401
    }
}

pub async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = JsFuture::from(promise).await;
}

pub fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

/// One request to the node. The body is already JSON; the answer is the text of the response.
pub async fn request(
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<String>,
    key: Option<&str>,
) -> Result<String, HttpError> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_cache(web_sys::RequestCache::NoStore);
    let headers = web_sys::Headers::new().map_err(|_| HttpError::unreachable())?;
    let put = |name: &str, value: &str| {
        let _ = headers.set(name, value);
    };
    if let Some(token) = token {
        put("Authorization", &format!("Bearer {token}"));
    }
    if body.is_some() {
        put("Content-Type", "application/json");
    }
    if let Some(key) = key {
        put("Idempotency-Key", key);
    }
    init.set_headers(&headers);
    if let Some(body) = &body {
        init.set_body(&JsValue::from_str(body));
    }
    // A node that stops answering must not leave the screen waiting forever.
    let controller = web_sys::AbortController::new().map_err(|_| HttpError::unreachable())?;
    init.set_signal(Some(&controller.signal()));
    let abort = Closure::once(move || controller.abort());
    let timer = window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            abort.as_ref().unchecked_ref(),
            REQUEST_TIMEOUT_MS,
        )
        .ok();
    let outcome = JsFuture::from(window().fetch_with_str_and_init(path, &init)).await;
    if let Some(timer) = timer {
        window().clear_timeout_with_handle(timer);
    }
    drop(abort);

    let response: web_sys::Response = outcome
        .map_err(|_| HttpError::unreachable())?
        .dyn_into()
        .map_err(|_| HttpError::unreachable())?;
    let status = response.status();
    let text = match response.text() {
        Ok(promise) => JsFuture::from(promise)
            .await
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    if response.ok() {
        return Ok(text);
    }
    Err(match serde_json::from_str::<JobError>(&text) {
        Ok(error) => HttpError {
            status,
            message: error.message,
        },
        Err(_) => HttpError {
            status,
            message: format!("El nodo respondió {status}"),
        },
    })
}

fn bad_answer(error: serde_json::Error) -> HttpError {
    HttpError {
        status: 502,
        message: format!("Respuesta del nodo ilegible: {error}"),
    }
}

pub async fn get<T: DeserializeOwned>(path: &str, token: &str) -> Result<T, HttpError> {
    let text = request("GET", path, Some(token), None, None).await?;
    serde_json::from_str(&text).map_err(bad_answer)
}

pub async fn post<T: DeserializeOwned>(
    path: &str,
    token: Option<&str>,
    body: &impl Serialize,
    key: Option<&str>,
) -> Result<T, HttpError> {
    let body = serde_json::to_string(body).map_err(bad_answer)?;
    let text = request("POST", path, token, Some(body), key).await?;
    serde_json::from_str(&text).map_err(bad_answer)
}

pub async fn delete<T: DeserializeOwned>(path: &str, token: &str) -> Result<T, HttpError> {
    let text = request("DELETE", path, Some(token), None, None).await?;
    serde_json::from_str(&text).map_err(bad_answer)
}

/// A random text for an idempotency key: the same key retried is the same request.
pub fn random_key() -> String {
    window()
        .crypto()
        .ok()
        .map(|crypto| crypto.random_uuid())
        .unwrap_or_else(|| format!("{}-{}", now_ms(), (js_sys::Math::random() * 1e9) as u64))
}

// ---- storage -----------------------------------------------------------------------------

fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

pub fn load(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

/// Storing can fail (private mode, full): the client then works without remembering.
pub fn save(key: &str, value: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(key, value);
    }
}

pub fn forget(key: &str) {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(key);
    }
}

// ---- the address -------------------------------------------------------------------------

pub fn hash() -> String {
    window().location().hash().unwrap_or_default()
}

/// Gives the keyboard to the element with this id, right now. Moving between the boxes of a code
/// must not wait for the next turn of the event loop: the next key may already be on its way.
pub fn focus(id: &str) {
    if let Some(element) = window()
        .document()
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

/// Changes the address without adding a history entry (used to drop a pairing code).
pub fn replace_hash(hash: &str) {
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(hash));
    }
}

// ---- what the page tells us --------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageEvent {
    Hash,
    Visible,
    Hidden,
    Online,
    Offline,
}

/// Reports the page's own events (address, visibility, connectivity) through `send`. Installed
/// once for the life of the page.
pub fn listen_page(send: impl Fn(PageEvent) + Clone + 'static) {
    let add = |target: &web_sys::EventTarget, name: &str, event: Box<dyn Fn()>| {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| event());
        let _ = target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        closure.forget();
    };
    let win: web_sys::EventTarget = window().into();
    let doc = window().document().expect("a document");
    let doc_target: web_sys::EventTarget = doc.clone().into();
    let tell = send.clone();
    add(&win, "hashchange", Box::new(move || tell(PageEvent::Hash)));
    let tell = send.clone();
    add(&win, "online", Box::new(move || tell(PageEvent::Online)));
    let tell = send.clone();
    add(&win, "offline", Box::new(move || tell(PageEvent::Offline)));
    add(
        &doc_target,
        "visibilitychange",
        Box::new(move || {
            send(if is_hidden() {
                PageEvent::Hidden
            } else {
                PageEvent::Visible
            });
        }),
    );
}

pub fn is_hidden() -> bool {
    window()
        .document()
        .is_some_and(|doc| doc.visibility_state() == web_sys::VisibilityState::Hidden)
}

// ---- the event socket --------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketEvent {
    Open,
    Text(String),
    Closed,
}

/// The open event stream. Dropping it closes the connection.
pub struct Socket {
    ws: web_sys::WebSocket,
    _open: Closure<dyn FnMut(web_sys::Event)>,
    _message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _close: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onmessage(None);
        self.ws.set_onclose(None);
        self.ws.set_onerror(None);
        let _ = self.ws.close();
    }
}

/// Opens `/api/v1/events`. A browser cannot set `Authorization` on a WebSocket, so the token
/// travels as the second subprotocol (`bearer.<token>`), which the node checks and never logs.
pub fn open_events(
    token: &str,
    tell: impl Fn(SocketEvent) + Clone + 'static,
) -> Result<Socket, String> {
    let location = window().location();
    let scheme = if location.protocol().unwrap_or_default() == "https:" {
        "wss:"
    } else {
        "ws:"
    };
    let host = location
        .host()
        .map_err(|_| "No se pudo leer la dirección".to_owned())?;
    let url = format!("{scheme}//{host}/api/v1/events");
    let protocols = js_sys::Array::of2(
        &JsValue::from_str("chxchx.v1"),
        &JsValue::from_str(&format!("bearer.{token}")),
    );
    let ws = web_sys::WebSocket::new_with_str_sequence(&url, &protocols)
        .map_err(|_| "No se pudo abrir la conexión en vivo".to_owned())?;

    let on_open = tell.clone();
    let open = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| on_open(SocketEvent::Open));
    ws.set_onopen(Some(open.as_ref().unchecked_ref()));
    let on_message = tell.clone();
    let message =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if let Some(text) = event.data().as_string() {
                on_message(SocketEvent::Text(text));
            }
        });
    ws.set_onmessage(Some(message.as_ref().unchecked_ref()));
    // `error` is always followed by `close`; one signal is enough.
    let on_close = tell;
    let close = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| on_close(SocketEvent::Closed));
    ws.set_onclose(Some(close.as_ref().unchecked_ref()));
    ws.set_onerror(Some(close.as_ref().unchecked_ref()));
    Ok(Socket {
        ws,
        _open: open,
        _message: message,
        _close: close,
    })
}

// ---- notifications -----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Unsupported,
    Default,
    Granted,
    Denied,
}

pub fn notification_permission() -> Permission {
    if js_sys::Reflect::has(&window(), &JsValue::from_str("Notification")).unwrap_or(false) {
        match web_sys::Notification::permission() {
            web_sys::NotificationPermission::Granted => Permission::Granted,
            web_sys::NotificationPermission::Denied => Permission::Denied,
            _ => Permission::Default,
        }
    } else {
        Permission::Unsupported
    }
}

/// Asks the person (this must come from a tap).
pub async fn request_notifications() -> Permission {
    if notification_permission() == Permission::Unsupported {
        return Permission::Unsupported;
    }
    if let Ok(promise) = web_sys::Notification::request_permission() {
        let _ = JsFuture::from(promise).await;
    }
    notification_permission()
}

/// Shows a notification when the page is in the background and the person allowed them. This is
/// only while the page is alive: waking a closed page needs push, which is not part of this.
pub fn notify(title: &str, body: &str) {
    if notification_permission() != Permission::Granted || !is_hidden() {
        return;
    }
    let options = web_sys::NotificationOptions::new();
    options.set_body(body);
    options.set_tag("chxchx");
    let _ = web_sys::Notification::new_with_options(title, &options);
}

/// How far the page can still be scrolled down, in pixels.
fn distance_to_end() -> f64 {
    let window = window();
    let Some(root) = window.document().and_then(|doc| doc.document_element()) else {
        return 0.0;
    };
    let height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    f64::from(root.scroll_height()) - (window.scroll_y().unwrap_or(0.0) + height)
}

fn scroll_down() {
    // The browser stops at the end of the page, so any large number reaches it.
    window().scroll_to_with_x_and_y(0.0, 1.0e9);
}

/// Keeps the newest message in view while the conversation grows, unless the person scrolled up
/// to read something older. It is instant (a smooth scroll is cut short by the next event) and
/// repeated after the layout settles.
pub async fn follow_end() {
    if distance_to_end() > 900.0 {
        return;
    }
    scroll_down();
    sleep(80).await;
    scroll_down();
}
