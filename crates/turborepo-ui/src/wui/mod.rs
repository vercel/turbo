//! Web UI for Turborepo. Creates a WebSocket server that can be subscribed to
//! by a web client to display the status of tasks.

use std::{
    cell::RefCell,
    collections::HashSet,
    io::Write,
    sync::{atomic::AtomicU32, Arc},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::Method,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{select, sync::Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing::log::warn;

use crate::{
    sender::{TaskSender, UISender},
    tui::event::{CacheResult, OutputLogs, TaskResult},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to start websocket server")]
    Server(#[from] std::io::Error),
    #[error("failed to start websocket server: {0}")]
    WebSocket(#[source] axum::Error),
    #[error("failed to serialize message: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("failed to send message")]
    Send(#[from] axum::Error),
    #[error("failed to send message through channel")]
    Broadcast(#[from] tokio::sync::broadcast::error::SendError<WebUIEvent>),
}

#[derive(Debug, Clone)]
pub struct WebUISender {
    pub tx: tokio::sync::broadcast::Sender<WebUIEvent>,
}

impl WebUISender {
    pub fn start_task(&self, task: String, output_logs: OutputLogs) {
        self.tx
            .send(WebUIEvent::StartTask { task, output_logs })
            .ok();
    }

    pub fn end_task(&self, task: String, result: TaskResult) {
        self.tx.send(WebUIEvent::EndTask { task, result }).ok();
    }

    pub fn status(&self, task: String, status: String, result: CacheResult) {
        self.tx
            .send(WebUIEvent::Status {
                task,
                status,
                result,
            })
            .ok();
    }

    pub fn set_stdin(&self, _: String, _: Box<dyn Write + Send>) {
        warn!("stdin is not supported (yet) in web ui");
    }

    pub fn task(&self, task: String) -> TaskSender {
        TaskSender {
            name: task,
            handle: UISender::Wui(self.clone()),
            logs: Default::default(),
        }
    }

    pub fn stop(&self) {
        self.tx.send(WebUIEvent::Stop).ok();
    }

    pub fn update_tasks(&self, tasks: Vec<String>) -> Result<(), crate::Error> {
        self.tx
            .send(WebUIEvent::UpdateTasks { tasks })
            .map_err(Error::Broadcast)?;

        Ok(())
    }

    pub fn output(&self, task: String, output: Vec<u8>) -> Result<(), crate::Error> {
        self.tx
            .send(WebUIEvent::TaskOutput { task, output })
            .map_err(Error::Broadcast)?;

        Ok(())
    }
}

// Specific events that the websocket server can send to the client,
// not all the `Event` types from the TUI
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum WebUIEvent {
    StartTask {
        task: String,
        output_logs: OutputLogs,
    },
    TaskOutput {
        task: String,
        output: Vec<u8>,
    },
    EndTask {
        task: String,
        result: TaskResult,
    },
    Status {
        task: String,
        status: String,
        result: CacheResult,
    },
    UpdateTasks {
        tasks: Vec<String>,
    },
    Stop,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerMessage<'a> {
    pub id: u32,
    #[serde(flatten)]
    pub payload: &'a WebUIEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Acknowledges the receipt of a message.
    /// If we don't receive an ack, we will resend the message
    Ack { id: u32 },
    /// Asks for all messages from the given id onwards
    CatchUp { start_id: u32 },
}

struct AppState {
    rx: tokio::sync::broadcast::Receiver<WebUIEvent>,
    // We use a tokio::sync::Mutex here because we want this future to be Send.
    #[allow(clippy::type_complexity)]
    messages: Arc<Mutex<RefCell<Vec<(WebUIEvent, u32)>>>>,
    current_id: Arc<AtomicU32>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.resubscribe(),
            messages: self.messages.clone(),
            current_id: self.current_id.clone(),
        }
    }
}

async fn handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    if let Err(e) = handle_socket_inner(socket, state).await {
        warn!("error handling socket: {e}");
    }
}

async fn handle_socket_inner(mut socket: WebSocket, state: AppState) -> Result<(), Error> {
    let mut state = state.clone();
    let mut acks = HashSet::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    'socket_loop: loop {
        select! {
            biased;
            Ok(event) = state.rx.recv() => {
                let id = state.current_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                let message_payload = serde_json::to_string(&ServerMessage {
                    id,
                    payload: &event
                })?;
                state.messages.lock().await.borrow_mut().push((event, id));

                socket.send(Message::Text(message_payload)).await?;
            }
            // Every 100ms, check if we need to resend any messages
            _ = interval.tick() => {
                let messages = state.messages.lock().await;
                let mut messages_to_send = Vec::new();
                for (event, id) in messages.borrow().iter() {
                    if !acks.contains(id) {
                        let message_payload = serde_json::to_string(event).unwrap();
                        messages_to_send.push(Message::Text(message_payload));
                    }
                };

                for message in messages_to_send {
                    socket.send(message).await?;
                }
            }
            message = socket.recv() => {
                if let Some(Ok(message)) = message {
                    let message_payload = message.into_text()?;
                    if message_payload.is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<ClientMessage>(&message_payload) {
                        match event {
                            ClientMessage::Ack { id } => {
                                acks.insert(id);
                            }
                            ClientMessage::CatchUp { start_id } => {
                                let mut messages_to_send = Vec::new();
                                for (event, id) in state.messages.lock().await.borrow().iter() {
                                    if id >= &start_id {
                                        continue;
                                    }
                                    let message_payload = serde_json::to_string(event).unwrap();
                                    messages_to_send.push(Message::Text(message_payload));
                                }

                                for message in messages_to_send {
                                    socket.send(message).await?;
                                }
                            }
                        }
                    } else {
                        warn!("failed to deserialize message from client: {message_payload}");
                    }
                } else {
                    break 'socket_loop;
                }
            },
        }
    }

    Ok(())
}

pub async fn start_ws_server(
    rx: tokio::sync::broadcast::Receiver<WebUIEvent>,
) -> Result<(), crate::Error> {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    let app = Router::new()
        .route("/ws", get(handler))
        .layer(cors)
        .with_state(AppState {
            rx,
            messages: Default::default(),
            current_id: Arc::new(AtomicU32::new(0)),
        });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:1337")
        .await
        .map_err(Error::Server)?;
    println!("Web UI listening on port 1337...");
    axum::serve(listener, app).await.map_err(Error::Server)?;

    Ok(())
}
