mod bridge;
mod config;
mod request_permission;
mod session;
mod session_update;

use std::path::PathBuf;

use agent_client_protocol::{
    AcpAgent, AcpAgentConfig, Error,
    schema::v1::{SessionNotification, StopReason},
};
use async_channel::{Receiver, Sender};
pub use bridge::Bridge;
pub use config::aries_config;
use tokio::sync::{oneshot, watch};

#[derive(Debug)]
pub enum Command {
    Prompt(String),
    Cancel,
}

#[derive(Debug)]
pub struct Permission {
    pub title: String,
    pub options: Vec<(String, String)>,
    pub response: oneshot::Sender<Option<String>>,
}

#[derive(Debug)]
pub enum Event {
    Ready(String),
    Update(Box<SessionNotification>),
    Permission(Permission),
    Finished(Result<StopReason, String>),
    Disconnected(String),
}

async fn connect(
    config: AcpAgentConfig,
    cwd: PathBuf,
    commands: Receiver<Command>,
    events: Sender<Event>,
) -> Result<(), Error> {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let updates = events.clone();
    let permissions = events.clone();
    agent_client_protocol::Client
        .builder()
        .name("aries-orbit")
        .on_receive_notification(
            async move |notification, _connection| {
                session_update::handle(notification, &updates).await
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request, responder, connection| {
                request_permission::handle(
                    request,
                    responder,
                    connection,
                    &permissions,
                    cancel_rx.clone(),
                )
                .await
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(AcpAgent::new(config), |connection| {
            session::run(connection, cwd, commands, events, cancel_tx)
        })
        .await
}
