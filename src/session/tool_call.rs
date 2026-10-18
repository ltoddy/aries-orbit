use agent_client_protocol::schema::v1::{
    ContentBlock, ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdateFields,
};

#[derive(Clone, Debug)]
pub struct ToolCallState {
    call: ToolCall,
    interrupted: bool,
    failure_revision: u64,
}

impl ToolCallState {
    pub fn new(call: ToolCall) -> Self {
        let failure_revision = u64::from(call.status == ToolCallStatus::Failed);
        Self { call, interrupted: false, failure_revision }
    }

    pub fn call(&self) -> &ToolCall {
        &self.call
    }

    pub fn update(&mut self, update: ToolCallUpdateFields) {
        if let Some(status) = update.status {
            self.update_status(status);
        }
        self.call.update(update);
    }

    pub fn replace(&mut self, call: ToolCall) {
        self.update_status(call.status);
        self.call = call;
    }

    fn update_status(&mut self, status: ToolCallStatus) {
        if status == ToolCallStatus::Failed && self.call.status != ToolCallStatus::Failed {
            self.failure_revision += 1;
        }
        self.interrupted = false;
    }

    pub fn failure_revision(&self) -> u64 {
        self.failure_revision
    }

    pub fn title(&self) -> &str {
        if !self.call.title.trim().is_empty() {
            &self.call.title
        } else {
            self.call.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or("工具调用")
        }
    }

    pub fn is_interrupted(&self) -> bool {
        self.interrupted
    }

    pub fn finish(&mut self) {
        self.interrupted =
            matches!(self.call.status, ToolCallStatus::Pending | ToolCallStatus::InProgress);
    }

    pub fn attachments(&self) -> impl Iterator<Item = (usize, &ContentBlock)> {
        self.call.content.iter().enumerate().filter_map(|(index, content)| match content {
            ToolCallContent::Content(content)
                if !matches!(content.content, ContentBlock::Text(_)) =>
            {
                Some((index, &content.content))
            }
            _ => None,
        })
    }
}
