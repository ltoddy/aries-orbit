mod bridge;
mod config;
mod connection;
mod create_terminal;
mod delete_sessions;
mod initialize;
mod kill_terminal;
mod list_sessions;
mod load_session;
mod new_session;
mod permission;
mod prompt;
mod release_terminal;
mod request_permission;
mod session_update;
mod terminal;
mod terminal_output;
mod wait_for_terminal_exit;

use std::path::PathBuf;

use agent_client_protocol::Error;
use agent_client_protocol::schema::v1::{
    LoadSessionRequest, SessionId, SessionInfo, SessionNotification, StopReason, TerminalId,
    TerminalOutputResponse,
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
    TerminalOutput {
        session_id: SessionId,
        terminal_id: TerminalId,
        output: TerminalOutputResponse,
    },
    Permission(Permission),
    Finished(Result<StopReason, Error>),
    Disconnected(String),
}
