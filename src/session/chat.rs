use std::sync::{Arc, OnceLock};

use agent_client_protocol::schema::MaybeUndefined;
use agent_client_protocol::schema::v1::{
    AvailableCommand, CompactionStatus, CompactionUpdate, ContentBlock, ContentChunk,
    SessionNotification, SessionUpdate,
};

use crate::session::display::{UpdateDisplay, code_block};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Thought,
    Event,
}

#[derive(Clone, Debug)]
pub struct Message {
    role: Role,
    title: String,
    text: String,
    updates: Vec<SessionNotification>,
    compaction: Option<CompactionUpdate>,
    details: OnceLock<UpdateDetails>,
}

#[derive(Clone, Debug)]
pub struct UpdateDetails {
    raw: Arc<str>,
    markdown: Arc<str>,
}

impl UpdateDetails {
    pub fn raw(&self) -> Arc<str> {
        self.raw.clone()
    }

    pub fn markdown(&self) -> Arc<str> {
        self.markdown.clone()
    }
}

impl Message {
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn updates(&self) -> &[SessionNotification] {
        &self.updates
    }
    pub fn compaction(&self) -> Option<&CompactionUpdate> {
        self.compaction.as_ref()
    }

    pub fn details(&self) -> &UpdateDetails {
        self.details.get_or_init(|| {
            let raw = serde_json::to_string_pretty(&self.updates).unwrap_or_else(|error| {
                format!("无法序列化会话更新：{error}\n\n{:#?}", self.updates)
            });
            UpdateDetails { markdown: code_block("json", &raw).into(), raw: raw.into() }
        })
    }
}

#[derive(Debug, Default)]
pub struct Chat {
    messages: Vec<Message>,
    busy: bool,
    error: Option<String>,
    session_title: Option<String>,
    available_commands: Vec<AvailableCommand>,
    stream: Option<usize>,
}

impl Chat {
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn is_busy(&self) -> bool {
        self.busy
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn available_commands(&self) -> &[AvailableCommand] {
        &self.available_commands
    }

    pub fn begin_turn(&mut self, text: &str) -> Option<String> {
        let text = text.trim();
        if self.busy || text.is_empty() {
            return None;
        }
        self.end_segment();
        self.messages.push(Message {
            role: Role::User,
            title: "你".into(),
            text: text.into(),
            updates: Vec::new(),
            compaction: None,
            details: OnceLock::new(),
        });
        self.busy = true;
        self.error = None;
        Some(text.into())
    }

    pub fn receive(&mut self, notification: SessionNotification) -> Option<usize> {
        if let SessionUpdate::AvailableCommandsUpdate(update) = notification.update {
            self.available_commands = update.available_commands;
            return None;
        }
        if let SessionUpdate::SessionInfoUpdate(update) = &notification.update {
            update.title.clone().update_to(&mut self.session_title);
        }
        if matches!(
            &notification.update,
            SessionUpdate::CompactionUpdate(_) | SessionUpdate::CompactionSummaryChunk(_)
        ) {
            self.end_segment();
            return Some(self.receive_compaction(notification));
        }
        let chunk = streamed_content(&notification.update);
        if let Some((role, title, chunk)) = chunk
            && let ContentBlock::Text(content) = &chunk.content
        {
            if let Some(index) = self.stream
                && index + 1 == self.messages.len()
                && self.messages[index].role == role
                && self.messages[index].updates.last().is_some_and(|previous| {
                    previous.session_id == notification.session_id
                        && streamed_content(&previous.update)
                            .is_some_and(|(_, _, previous)| previous.message_id == chunk.message_id)
                })
            {
                self.messages[index].text.push_str(&content.text);
                self.messages[index].details.take();
                self.messages[index].updates.push(notification);
                return Some(index);
            }
            self.stream = Some(self.messages.len());
            self.messages.push(Message {
                role,
                title: title.into(),
                text: content.text.clone(),
                updates: vec![notification],
                compaction: None,
                details: OnceLock::new(),
            });
        } else {
            self.end_segment();
            let (title, text) = UpdateDisplay::from(&notification.update).into_parts();
            self.messages.push(Message {
                role: Role::Event,
                title,
                text,
                updates: vec![notification],
                compaction: None,
                details: OnceLock::new(),
            });
        }
        Some(self.messages.len() - 1)
    }

    fn receive_compaction(&mut self, notification: SessionNotification) -> usize {
        let id = match &notification.update {
            SessionUpdate::CompactionUpdate(update) => &update.compaction_id,
            SessionUpdate::CompactionSummaryChunk(chunk) => &chunk.compaction_id,
            _ => unreachable!("only compaction notifications reach this handler"),
        };
        let index = self
            .messages
            .iter()
            .position(|message| {
                message.compaction.as_ref().is_some_and(|state| &state.compaction_id == id)
                    && message
                        .updates
                        .first()
                        .is_some_and(|first| first.session_id == notification.session_id)
            })
            .unwrap_or_else(|| {
                self.messages.push(Message {
                    role: Role::Event,
                    title: String::new(),
                    text: String::new(),
                    updates: Vec::new(),
                    compaction: Some(CompactionUpdate::new(
                        id.clone(),
                        CompactionStatus::InProgress,
                    )),
                    details: OnceLock::new(),
                });
                self.messages.len() - 1
            });
        let message = &mut self.messages[index];
        message.details.take();
        let state = message.compaction.as_mut().expect("compaction entry has state");
        match &notification.update {
            SessionUpdate::CompactionUpdate(patch) => {
                state.status = patch.status.clone();
                if !patch.summary.is_undefined() {
                    state.summary = patch.summary.clone();
                }
                if !patch.error.is_undefined() {
                    state.error = patch.error.clone();
                }
                if !patch.meta.is_undefined() {
                    state.meta = patch.meta.clone();
                }
            }
            SessionUpdate::CompactionSummaryChunk(chunk) => {
                if let MaybeUndefined::Value(summary) = &mut state.summary {
                    summary.push(chunk.content.clone());
                } else {
                    state.summary = MaybeUndefined::Value(vec![chunk.content.clone()]);
                }
            }
            _ => unreachable!(),
        }
        (message.title, message.text) =
            UpdateDisplay::from(&SessionUpdate::CompactionUpdate(state.clone())).into_parts();
        message.updates.push(notification);
        index
    }

    pub fn end_segment(&mut self) {
        self.stream = None;
    }

    pub fn finish(&mut self, error: Option<String>) {
        self.end_segment();
        self.busy = false;
        self.error = error;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.session_title = title;
    }

    pub fn title(&self) -> String {
        if let Some(title) = self.session_title.as_deref().filter(|title| !title.trim().is_empty())
        {
            return title.to_owned();
        }
        self.messages
            .iter()
            .find(|message| message.role == Role::User)
            .map(|message| {
                let line = message.text.lines().next().unwrap_or_default();
                let mut title: String = line.chars().take(24).collect();
                if line.chars().count() > 24 {
                    title.push('…');
                }
                title
            })
            .unwrap_or_else(|| "新对话".into())
    }
}

fn streamed_content(update: &SessionUpdate) -> Option<(Role, &'static str, &ContentChunk)> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => Some((Role::Assistant, "Aries", chunk)),
        SessionUpdate::AgentThoughtChunk(chunk) => Some((Role::Thought, "思考", chunk)),
        SessionUpdate::UserMessageChunk(chunk) => Some((Role::User, "用户消息", chunk)),
        _ => None,
    }
}
