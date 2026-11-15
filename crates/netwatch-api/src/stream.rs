//! WS `/api/v1/stream` — API.md §3 (streaming half of the API).
//!
//! One connection multiplexes all channels. Phase 1 is the skeleton:
//! `subscribe`/`unsubscribe`/`ping` ops, status pushed on change,
//! `live.connections`/`usage.totals` send their stub snapshot/update and go
//! quiet until Phase 2 wires real data.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use netwatch_common::{Channel, ErrorCode, StatusSnapshot};
use serde_json::json;

use crate::auth::token_matches;
use crate::state::AppState;

/// Close code for WS auth failures (API.md §3 rules).
const CLOSE_AUTH: u16 = 4401;

/// `GET /api/v1/stream` — upgrade, then enforce auth with close 4401.
///
/// The contract says auth failure "closes with code 4401" (not a 401 HTTP
/// response), so the upgrade is accepted and the socket is closed with the
/// code — the UI's client keys its "auth" reconnect state off of it.
pub async fn stream(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    headers: axum::http::HeaderMap,
) -> Response {
    let authorized = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|tok| token_matches(tok, state.token.as_str()));

    ws.on_upgrade(move |mut socket| async move {
        if authorized {
            run_connection(state, socket).await;
        } else {
            // §3: auth failure closes with 4401.
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: CLOSE_AUTH,
                    reason: "AUTH_INVALID".into(),
                })))
                .await;
        }
    })
}

/// Per-connection state: which channels this client subscribed to.
struct Connection {
    subs: HashSet<Channel>,
}

/// Frame builders — shapes per API.md §3 server→client table.
fn status_frame(snap: &StatusSnapshot) -> Message {
    Message::text(
        json!({
            "channel": Channel::Status.as_str(),
            "kind": "update",
            "data": snap,
        })
        .to_string(),
    )
}

fn live_snapshot_frame() -> Message {
    // Phase 1 stub: no traffic data yet (Phase 2 wires the collector).
    Message::text(
        json!({
            "channel": Channel::LiveConnections.as_str(),
            "kind": "snapshot",
            "data": [],
        })
        .to_string(),
    )
}

fn usage_totals_frame() -> Message {
    // Phase 1 stub: zeroed totals until accounting lands (Phase 2).
    Message::text(
        json!({
            "channel": Channel::UsageTotals.as_str(),
            "kind": "update",
            "data": { "today": { "up": 0, "down": 0 } },
        })
        .to_string(),
    )
}

fn error_frame(code: ErrorCode, message: impl Into<String>) -> Message {
    Message::text(
        json!({
            "op": "error",
            "error": { "code": code, "message": message.into() },
        })
        .to_string(),
    )
}

/// Initial frame sent on subscribe (API.md §3: snapshot on subscribe).
fn initial_frame(channel: Channel, state: &AppState) -> Message {
    match channel {
        Channel::Status => status_frame(&state.status.borrow().clone()),
        Channel::LiveConnections => live_snapshot_frame(),
        Channel::UsageTotals => usage_totals_frame(),
    }
}

/// Run one connection until the client disconnects or the server shuts down.
async fn run_connection(state: Arc<AppState>, mut socket: WebSocket) {
    state.client_delta(1);
    let mut conn = Connection {
        subs: HashSet::new(),
    };
    let mut status_rx = state.status.clone();

    loop {
        tokio::select! {
            // Status changes push immediately to subscribed clients (§3:
            // "update on change").
            changed = status_rx.changed() => {
                if changed.is_err() {
                    // Publisher dropped: server shutting down.
                    break;
                }
                if conn.subs.contains(&Channel::Status) {
                    let snap = status_rx.borrow().clone();
                    if socket.send(status_frame(&snap)).await.is_err() {
                        break;
                    }
                }
            }
            frame = socket.recv() => {
                let Some(msg) = frame else {
                    // Client went away (or the socket errored): done either way.
                    break;
                };
                let Ok(msg) = msg else { break };
                let keep = match handle_client_frame(&state, &mut conn, msg, &mut socket).await {
                    Ok(keep) => keep,
                    Err(frame) => socket.send(frame).await.is_ok(), // close frame or send failure
                };
                if !keep {
                    break;
                }
            }
        }
    }

    state.client_delta(-1);
}

/// Handle one client frame. `Ok(false)` = close the connection; `Err(msg)`
/// = send the returned error frame (API.md §3 op:"error").
async fn handle_client_frame(
    state: &Arc<AppState>,
    conn: &mut Connection,
    msg: Message,
    socket: &mut WebSocket,
) -> Result<bool, Message> {
    match msg {
        Message::Text(text) => {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                return Err(error_frame(
                    ErrorCode::QueryInvalid,
                    "Frames are versioned JSON (API.md §3)",
                ));
            };
            match v["op"].as_str() {
                Some("ping") => {
                    let sent = socket
                        .send(Message::text(json!({ "op": "pong" }).to_string()))
                        .await
                        .is_ok();
                    Ok(sent)
                }
                Some("subscribe") => {
                    match parse_channel(&v) {
                        Ok(ch) => {
                            conn.subs.insert(ch);
                            // §3: subscribe → immediate snapshot/update.
                            Ok(socket.send(initial_frame(ch, state)).await.is_ok())
                        }
                        Err(msg) => Err(error_frame(ErrorCode::QueryInvalid, msg)),
                    }
                }
                Some("unsubscribe") => match parse_channel(&v) {
                    Ok(ch) => {
                        conn.subs.remove(&ch);
                        Ok(true)
                    }
                    Err(msg) => Err(error_frame(ErrorCode::QueryInvalid, msg)),
                },
                _ => Err(error_frame(
                    ErrorCode::QueryInvalid,
                    "Unknown op; expected subscribe|unsubscribe|ping",
                )),
            }
        }
        Message::Close(_) => Ok(false),
        // Binary and other control frames are not part of the contract.
        _ => Err(error_frame(
            ErrorCode::QueryInvalid,
            "Unsupported frame type",
        )),
    }
}

/// Parse `{"op":"…","channel":"…"}` into a known channel; errors carry the
/// contract's message shape (API.md §3 error frame example).
fn parse_channel(v: &serde_json::Value) -> Result<Channel, String> {
    let name = v["channel"].as_str().ok_or("Missing channel name")?;
    match name {
        "live.connections" => Ok(Channel::LiveConnections),
        "status" => Ok(Channel::Status),
        "usage.totals" => Ok(Channel::UsageTotals),
        other => Err(format!("Unknown channel '{other}'")),
    }
}
