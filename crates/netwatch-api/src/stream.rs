//! WS `/api/v1/stream` — API.md §3 (streaming half of the API).
//!
//! One connection multiplexes all channels. Phase 1 is the skeleton:
//! `subscribe`/`unsubscribe`/`ping` ops, status pushed on change,
//! `live.connections`/`usage.totals` send their stub snapshot/update and go
//! quiet until Phase 2 wires real data.
//!
//! Auth is handled at the socket level via peer-credential (`SO_PEERCRED`),
//! so no token check is needed here (D1a). The upgrade itself happens in the
//! axum router (`routes::stream_upgrade`); this module runs the upgraded
//! connection.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use futures_util::StreamExt;
use netwatch_common::{Channel, ErrorCode, StatusSnapshot};
use serde_json::json;
use tokio::sync::broadcast;

use crate::state::AppState;

/// Per-connection state: which channels this client subscribed to.
struct Connection {
    subs: HashSet<Channel>,
}

/// Build a JSON object for a status frame.
fn status_obj(snap: &StatusSnapshot) -> serde_json::Value {
    json!({
        "channel": Channel::Status.as_str(),
        "kind": "update",
        "data": snap,
    })
}

/// Build a JSON object for a live snapshot frame (rows are the current
/// C1-shaped live view).
fn live_snapshot_obj(rows: &[netwatch_common::LiveConnectionRow]) -> serde_json::Value {
    json!({
        "channel": Channel::LiveConnections.as_str(),
        "kind": "snapshot",
        "data": rows,
    })
}

/// Build a JSON object for a live delta frame (API.md §3: `added`/`updated`
/// keyed by row id, `removed` ids).
fn live_delta_obj(delta: &netwatch_common::LiveDelta) -> serde_json::Value {
    json!({
        "channel": Channel::LiveConnections.as_str(),
        "kind": "delta",
        "added": delta.added,
        "updated": delta.updated,
        "removed": delta.removed,
    })
}

/// Build a JSON object for a usage totals frame.
fn usage_totals_obj() -> serde_json::Value {
    json!({
        "channel": Channel::UsageTotals.as_str(),
        "kind": "update",
        "data": { "today": { "up": 0, "down": 0 } },
    })
}

/// Build a JSON object for an error frame.
fn error_obj(code: ErrorCode, message: impl Into<String>) -> serde_json::Value {
    json!({
        "op": "error",
        "error": { "code": code, "message": message.into() },
    })
}

/// Initial frame sent on subscribe (API.md §3: snapshot on subscribe).
fn initial_frame(channel: Channel, state: &AppState) -> Message {
    let v = match channel {
        Channel::Status => status_obj(&state.status.borrow().clone()),
        Channel::LiveConnections => live_snapshot_obj(&state.live.snapshot.borrow()),
        Channel::UsageTotals => usage_totals_obj(),
    };
    Message::text(v.to_string())
}

/// Run one connection until the client disconnects or the server shuts down.
pub async fn run_connection(state: Arc<AppState>, mut stream: WebSocket) {
    state.client_delta(1);
    let mut conn = Connection {
        subs: HashSet::new(),
    };
    let mut status_rx = state.status.clone();
    // Per-connection delta receiver; subscribed to the broadcast on connect
    // so frames between subscribe and this point are not lost (the client
    // still gets the fresh snapshot on subscribe).
    let mut live_rx = state.live.deltas.subscribe();

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
                    if stream
                        .send(Message::text(status_obj(&snap).to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            // Live deltas push to subscribed clients (§3: ≤ 1 msg/s, kept
            // by the live service's 1 Hz sampler).
            delta = live_rx.recv() => {
                match delta {
                    Ok(delta) => {
                        if conn.subs.contains(&Channel::LiveConnections) {
                            if stream
                                .send(Message::text(live_delta_obj(&delta).to_string()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_missed)) => {
                        // Receiver fell behind: resync with a fresh snapshot
                        // instead of replaying stale deltas (API.md §3 rule:
                        // snapshot is the recovery mechanism).
                        if conn.subs.contains(&Channel::LiveConnections) {
                            let rows = state.live.snapshot.borrow().clone();
                            if stream
                                .send(Message::text(live_snapshot_obj(&rows).to_string()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
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
    stream: &mut WebSocket,
) -> Result<bool, Message> {
    match msg {
        Message::Text(text) => {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                return Err(Message::text(
                    error_obj(
                        ErrorCode::QueryInvalid,
                        "Frames are versioned JSON (API.md §3)",
                    )
                    .to_string(),
                ));
            };
            match v["op"].as_str() {
                Some("ping") => {
                    let sent = stream
                        .send(Message::text(json!({ "op": "pong" }).to_string()))
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

/// Build a JSON error frame.
fn error_frame(code: ErrorCode, message: impl Into<String>) -> Message {
    Message::text(error_obj(code, message).to_string())
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
