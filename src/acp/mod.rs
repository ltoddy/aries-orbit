mod bridge;
mod config;
mod connection;
mod delete_sessions;
mod initialize;
mod list_sessions;
mod load_session;
mod new_session;
mod permission;
mod prompt;
mod request_permission;
mod session_update;

use std::path::PathBuf;

use agent_client_protocol::Error;
use agent_client_protocol::schema::v1::{
    LoadSessionRequest, SessionId, SessionInfo, SessionNotification, StopReason,
};

pub use self::bridge::Bridge;
pub use self::config::aries_config;
pub use self::delete_sessions::DeletionReport;
pub use self::permission::Permission;

#[derive(Debug)]
pub enum Command {
    NewSession(PathBuf),
    ListSessions,
    DeleteSessions(Vec<SessionId>),
    LoadSession(Box<LoadSessionRequest>),
    Prompt { target: PromptTarget, text: String },
    Cancel,
}

#[derive(Debug)]
pub enum PromptTarget {
    New(PathBuf),
    Existing(SessionId),
}

#[derive(Debug)]
pub enum Event {
    Connected { list_sessions: bool, load_session: bool },
    Sessions(Result<Vec<SessionInfo>, Error>),
    SessionsDeleted(DeletionReport),
    SessionLoaded { session_id: SessionId, result: Result<(), Error> },
    Ready(SessionId),
    Update(Box<SessionNotification>),
    Permission(Permission),
    Finished(Result<StopReason, Error>),
    Disconnected(String),
}
