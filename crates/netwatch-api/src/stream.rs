//! WS `/api/v1/stream` — API.md §3 (streaming half of the API).
//!
//! One connection multiplexes all channels. Phase 1 is the skeleton:
//! `subscribe`/`unsubscribe`/`ping` ops, status pushed on change,
//! `live.connections`/`usage.totals` send their stub snapshot/update and go
//! quiet until Phase 2 wires real data.
//!
//! Auth is handled at the socket level via peer-credential (`SO_PEERCRED`),
//! so no token check is needed here (D1a).

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use futures_util::SinkExt;
use futures_util::StreamExt;
use netwatch_common::{Channel, ErrorCode, StatusSnapshot};
use serde_json::{Value, json};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use crate::state::AppState;

/// Per-connection state: which channels this client subscribed to.
struct Connection {
    subs: HashSet<Channel>,
}

/// Build a JSON object for a status frame.
fn status_obj(snap: &StatusSnapshot) -> Value {
    serde_json::json!({
        "channel": Channel::Status.as_str(),
        "kind": "update",
        "data": snap,
    })
}

/// Build a JSON object for a live snapshot frame.
fn live_snapshot_obj() -> Value {
    serde_json::json!({
        "channel": Channel::LiveConnections.as_str(),
        "kind": "snapshot",
        "data": [],
    })
}

/// Build a JSON object for a usage totals frame.
fn usage_totals_obj() -> Value {
    serde_json::json!({
        "channel": Channel::UsageTotals.as_str(),
        "kind": "update",
        "data": { "today": { "up": 0, "down": 0 } },
    })
}

/// Build a JSON object for an error frame.
fn error_obj(code: ErrorCode, message: impl Into<String>) -> Value {
    serde_json::json!({
        "op": "error",
        "error": { "code": code, "message": message.into() },
    })
}

/// Initial frame sent on subscribe (API.md §3: snapshot on subscribe).
fn initial_frame(channel: Channel, state: &AppState) -> Message {
    match channel {
        Channel::Status => status_frame(&state.status.borrow().clone()),
        Channel::LiveConnections => live_snapshot_frame(),
        Channel::UsageTotals => usage_totals_frame(),
    }
}

/// Build a JSON status frame from a snapshot.
fn status_frame(snap: &StatusSnapshot) -> Message {
    Message::text(serde_json::json!(status_obj(snap)).to_string())
}

/// Build a JSON live snapshot frame.
fn live_snapshot_frame() -> Message {
    Message::text(serde_json::json!(live_snapshot_obj()).to_string())
}

/// Build a JSON usage totals frame.
fn usage_totals_frame() -> Message {
    Message::text(serde_json::json!(usage_totals_obj()).to_string())
}

/// Build a JSON error frame.
fn error_frame(code: ErrorCode, message: impl Into<String>) -> Message {
    Message::text(
        serde_json::json!({
            "op": "error",
            "error": { "code": code, "message": message.into() },
        })
        .to_string(),
    )
}

/// Run one connection until the client disconnects or the server shuts down.
pub async fn run_connection(
    state: Arc<AppState>,
    mut stream: WebSocketStream<tokio::net::UnixStream>,
) {
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
                    if stream.send(status_frame(&snap)).await.is_err() {
                        break;
                    }
                }
            }
            incoming = stream.next() => {
                let Some(Ok(msg)) = incoming else {
                    // Client went away (or the socket errored): done either way.
                    break;
                };
                let keep = match handle_client_frame(&state, &mut conn, msg, &mut stream).await {
                    Ok(keep) => keep,
                    Err(frame) => stream.send(frame).await.is_ok(),
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
/// = send the returned error frame (API.md §3 error frame).
async fn handle_client_frame(
    state: &Arc<AppState>,
    conn: &mut Connection,
    msg: Message,
    stream: &mut WebSocketStream<tokio::net::UnixStream>,
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
                    let sent = stream
                        .send(Message::text(
                            serde_json::json!({ "op": "pong" }).to_string(),
                        ))
                        .await;
                    Ok(sent.is_ok())
                }
                Some("subscribe") => match parse_channel(&v) {
                    Ok(ch) => {
                        conn.subs.insert(ch);
                        // §3: subscribe → immediate snapshot/update.
                        Ok(stream.send(initial_frame(ch, state)).await.is_ok())
                    }
                    Err(msg) => Err(error_frame(ErrorCode::QueryInvalid, msg)),
                },
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
