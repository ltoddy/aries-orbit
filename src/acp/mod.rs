mod bridge;
mod config;
mod delete_sessions;
mod list_sessions;
mod load_session;
mod permission;
mod request_permission;
mod session;
mod session_update;

use std::path::PathBuf;

use agent_client_protocol::{
    AcpAgent, AcpAgentConfig, Error,
    schema::v1::{LoadSessionRequest, SessionId, SessionInfo, SessionNotification, StopReason},
};
use async_channel::{Receiver, Sender};
use tokio::sync::watch;

pub use self::bridge::Bridge;
pub use self::config::aries_config;
pub use self::delete_sessions::DeletionReport;
pub use self::permission::Permission;

#[derive(Debug)]
pub enum Command {
    ListSessions,
    DeleteSessions(Vec<SessionId>),
    LoadSession(Box<LoadSessionRequest>),
    Prompt(String),
    Cancel,
}

#[derive(Debug)]
pub enum Event {
    Connected {
        list_sessions: bool,
        load_session: bool,
    },
    Sessions(Result<Vec<SessionInfo>, Error>),
    SessionsDeleted(DeletionReport),
    SessionLoaded {
        session_id: SessionId,
        result: Result<(), Error>,
    },
    Ready(SessionId),
    Update(Box<SessionNotification>),
    Permission(Permission),
    Finished(Result<StopReason, Error>),
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
