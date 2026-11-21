//! Keeps the client in step with the node: opens the live stream, reconnects when it drops and
//! catches up on what was missed. One task owns every change that comes from outside Dioxus
//! (the socket, the page going to the background, the network coming back).
//!
//! Why a channel: the browser's callbacks run outside Dioxus, where writing a signal is not
//! allowed. They only send a message; this task, which Dioxus runs, does the writing.

use crate::actions;
use crate::net::{self, PageEvent, SocketEvent};
use crate::route::Route;
use crate::store::{self, CLOCK, CONN, Conn, ROUTE};
use chxchx_protocol::JobEvent;
use dioxus::prelude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use futures_util::future::{Either, select};
use std::cell::RefCell;

/// While connected, the node is asked how it is every so often: a phone can lose the socket
/// without being told.
const POLL_MS: i32 = 25_000;
const FIRST_RETRY_MS: u64 = 1_000;
const LAST_RETRY_MS: u64 = 15_000;

enum Msg {
    Page(PageEvent),
    Socket(u64, SocketEvent),
    /// The person linked or unlinked this device.
    Session,
}

thread_local! {
    static ENGINE: RefCell<Option<UnboundedSender<Msg>>> = const { RefCell::new(None) };
}

/// Tells the engine this device was linked or unlinked.
pub fn session_changed() {
    ENGINE.with(|engine| {
        if let Some(tx) = engine.borrow().as_ref() {
            let _ = tx.unbounded_send(Msg::Session);
        }
    });
}

async fn next_or_timeout(rx: &mut UnboundedReceiver<Msg>, ms: i32) -> Option<Option<Msg>> {
    match select(rx.next(), Box::pin(net::sleep(ms))).await {
        Either::Left((message, _)) => Some(message),
        Either::Right(_) => None,
    }
}

pub async fn run() {
    let (tx, mut rx) = unbounded::<Msg>();
    ENGINE.with(|engine| *engine.borrow_mut() = Some(tx.clone()));
    let page = tx.clone();
    net::listen_page(move |event| {
        let _ = page.unbounded_send(Msg::Page(event));
    });
    *ROUTE.write() = Route::from_hash(&net::hash());

    let mut socket: Option<(u64, net::Socket)> = None;
    let mut generation = 0u64;
    let mut backoff = FIRST_RETRY_MS;
    let mut retry_at = 0u64;

    loop {
        let signed_in = store::token().is_some();
        if !signed_in {
            socket = None;
        }
        if signed_in && socket.is_none() && net::now_ms() >= retry_at {
            *CONN.write() = Conn::Connecting;
            match connect(&tx, &mut generation).await {
                Ok(open) => {
                    socket = Some((generation, open));
                    backoff = FIRST_RETRY_MS;
                }
                Err(message) => {
                    if store::token().is_some() {
                        *CONN.write() = Conn::Offline(message);
                    }
                    retry_at = net::now_ms() + backoff;
                    backoff = (backoff * 2).min(LAST_RETRY_MS);
                }
            }
        }
        let wait = if signed_in && socket.is_none() {
            retry_at
                .saturating_sub(net::now_ms())
                .clamp(200, LAST_RETRY_MS) as i32
        } else {
            POLL_MS
        };
        match next_or_timeout(&mut rx, wait).await {
            None => {
                // Quiet for a while: a check that also keeps the clock and the counts fresh.
                if signed_in && socket.is_some() && !net::is_hidden() {
                    if let Err(error) = actions::refresh().await {
                        if error.is_unauthorized() {
                            actions::sign_out();
                        } else {
                            socket = None;
                            *CONN.write() = Conn::Offline(error.message);
                            retry_at = net::now_ms() + FIRST_RETRY_MS;
                        }
                    }
                } else {
                    *CLOCK.write() = net::now_ms();
                }
            }
            Some(None) => return,
            Some(Some(Msg::Session)) => {
                socket = None;
                retry_at = 0;
                backoff = FIRST_RETRY_MS;
            }
            Some(Some(Msg::Page(event))) => match event {
                PageEvent::Hash => {
                    *ROUTE.write() = Route::from_hash(&net::hash());
                    if let Route::Job(id) = ROUTE.read().clone() {
                        let _ = actions::events_of(&id).await;
                    }
                }
                PageEvent::Visible | PageEvent::Online => {
                    // Back in the foreground: what happened meanwhile is fetched at once.
                    if signed_in {
                        if socket.is_none() {
                            retry_at = 0;
                        } else if actions::refresh().await.is_err() {
                            socket = None;
                            retry_at = 0;
                        }
                    }
                }
                PageEvent::Offline => {
                    if signed_in {
                        socket = None;
                        *CONN.write() = Conn::Offline("Sin red".to_owned());
                    }
                }
                PageEvent::Hidden => {}
            },
            Some(Some(Msg::Socket(from, event))) => {
                if from != generation {
                    continue; // a socket that was already replaced
                }
                match event {
                    SocketEvent::Open => {
                        *CONN.write() = Conn::Online;
                        // Anything said between the first read and the socket opening.
                        if actions::refresh().await.is_err() {
                            socket = None;
                        }
                    }
                    SocketEvent::Text(text) => {
                        if let Ok(event) = serde_json::from_str::<JobEvent>(&text) {
                            actions::ingest(event).await;
                        }
                    }
                    SocketEvent::Closed => {
                        socket = None;
                        if signed_in {
                            *CONN.write() = Conn::Offline("Conexión interrumpida".to_owned());
                            retry_at = net::now_ms() + backoff;
                            backoff = (backoff * 2).min(LAST_RETRY_MS);
                        }
                    }
                }
            }
        }
    }
}

/// Reads the node once, then opens the live stream.
async fn connect(tx: &UnboundedSender<Msg>, generation: &mut u64) -> Result<net::Socket, String> {
    let token = store::token().ok_or("No hay sesión")?;
    if let Err(error) = actions::refresh().await {
        if error.is_unauthorized() {
            actions::sign_out();
        }
        return Err(error.message);
    }
    *generation += 1;
    let current = *generation;
    let tx = tx.clone();
    net::open_events(&token, move |event| {
        let _ = tx.unbounded_send(Msg::Socket(current, event));
    })
}
