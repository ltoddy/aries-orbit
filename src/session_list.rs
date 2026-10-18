use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use agent_client_protocol::{
    Error,
    schema::v1::{SessionId, SessionInfo, SessionInfoUpdate},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ListState {
    #[default]
    Disconnected,
    Connecting,
    Unsupported,
    Ready,
    Loading,
}

#[derive(Debug, Default)]
pub struct SessionList {
    entries: Vec<SessionInfo>,
    expanded_cwds: HashSet<PathBuf>,
    state: ListState,
    refresh_pending: bool,
    error: Option<String>,
}

impl SessionList {
    pub fn entries(&self) -> &[SessionInfo] {
        &self.entries
    }

    pub fn session(&self, id: &SessionId) -> Option<&SessionInfo> {
        self.entries
            .iter()
            .find(|session| &session.session_id == id)
    }

    pub fn is_supported(&self) -> bool {
        matches!(self.state, ListState::Ready | ListState::Loading)
    }

    pub fn is_unsupported(&self) -> bool {
        self.state == ListState::Unsupported
    }

    pub fn is_connecting(&self) -> bool {
        self.state == ListState::Connecting
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, ListState::Connecting | ListState::Loading)
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn connect(&mut self) {
        self.state = ListState::Connecting;
        self.refresh_pending = false;
        self.error = None;
    }

    pub fn connected(&mut self, supported: bool) {
        if self.state != ListState::Connecting {
            return;
        }
        self.state = if supported {
            ListState::Ready
        } else {
            ListState::Unsupported
        };
    }

    pub fn disconnect(&mut self, error: String) {
        self.state = ListState::Disconnected;
        self.refresh_pending = false;
        self.error = Some(error);
    }

    pub fn defer_refresh(&mut self) {
        self.refresh_pending = true;
    }

    pub fn take_refresh_request(&mut self) -> bool {
        std::mem::take(&mut self.refresh_pending)
    }

    pub fn begin_refresh(&mut self) -> bool {
        if self.state != ListState::Ready {
            return false;
        }
        self.state = ListState::Loading;
        self.error = None;
        true
    }

    pub fn fail_refresh(&mut self, error: String) {
        if self.state != ListState::Loading {
            return;
        }
        self.state = ListState::Ready;
        self.error = Some(error);
    }

    pub fn receive(&mut self, result: Result<Vec<SessionInfo>, Error>) {
        if self.state != ListState::Loading {
            return;
        }
        self.state = ListState::Ready;
        match result {
            Ok(sessions) => {
                let mut ids = HashSet::new();
                self.entries = sessions
                    .into_iter()
                    .filter(|session| ids.insert(session.session_id.clone()))
                    .collect();
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn update(&mut self, id: &SessionId, update: &SessionInfoUpdate) {
        if let Some(session) = self
            .entries
            .iter_mut()
            .find(|session| &session.session_id == id)
        {
            update.title.clone().update_to(&mut session.title);
            update.updated_at.clone().update_to(&mut session.updated_at);
        }
    }

    pub fn remove_sessions(&mut self, ids: &HashSet<SessionId>) {
        self.entries
            .retain(|session| !ids.contains(&session.session_id));
    }

    pub fn is_project_expanded(&self, cwd: &Path) -> bool {
        self.expanded_cwds.contains(cwd)
    }

    pub fn expand_project(&mut self, cwd: &Path) {
        self.expanded_cwds.insert(cwd.to_path_buf());
    }

    pub fn expand_all(&mut self) {
        self.expanded_cwds
            .extend(self.entries.iter().map(|session| session.cwd.clone()));
    }

    pub fn collapse_all(&mut self) {
        self.expanded_cwds.clear();
    }

    pub fn toggle_project(&mut self, cwd: &Path) {
        if !self.expanded_cwds.remove(cwd) {
            self.expanded_cwds.insert(cwd.to_path_buf());
        }
    }
}
