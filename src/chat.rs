use agent_client_protocol::schema::{
    MaybeUndefined,
    v1::{
        CompactionStatus, CompactionUpdate, ContentBlock, ContentChunk, SessionNotification,
        SessionUpdate,
    },
};

use crate::session_update::describe;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Thought,
    Event,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub role: Role,
    pub title: String,
    pub text: String,
    pub updates: Vec<SessionNotification>,
    pub compaction: Option<CompactionUpdate>,
}

#[derive(Debug, Default)]
pub struct Chat {
    pub messages: Vec<Message>,
    pub busy: bool,
    pub error: Option<String>,
    stream: Option<usize>,
}

impl Chat {
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
        });
        self.busy = true;
        self.error = None;
        Some(text.into())
    }

    pub fn receive(&mut self, notification: SessionNotification) -> usize {
        if matches!(
            &notification.update,
            SessionUpdate::CompactionUpdate(_) | SessionUpdate::CompactionSummaryChunk(_)
        ) {
            self.end_segment();
            return self.receive_compaction(notification);
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
                self.messages[index].updates.push(notification);
                return index;
            }
            self.stream = Some(self.messages.len());
            self.messages.push(Message {
                role,
                title: title.into(),
                text: content.text.clone(),
                updates: vec![notification],
                compaction: None,
            });
        } else {
            self.end_segment();
            let display = describe(&notification.update);
            self.messages.push(Message {
                role: Role::Event,
                title: display.title,
                text: display.body,
                updates: vec![notification],
                compaction: None,
            });
        }
        self.messages.len() - 1
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
                message
                    .compaction
                    .as_ref()
                    .is_some_and(|state| &state.compaction_id == id)
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
                });
                self.messages.len() - 1
            });
        let message = &mut self.messages[index];
        let state = message
            .compaction
            .as_mut()
            .expect("compaction entry has state");
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
        let display = describe(&SessionUpdate::CompactionUpdate(state.clone()));
        message.title = display.title;
        message.text = display.body;
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

    pub fn title(&self) -> String {
        self.messages
            .iter()
            .find(|message| message.role == Role::User && message.updates.is_empty())
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
