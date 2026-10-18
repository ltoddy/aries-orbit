use std::sync::Arc;

use agent_client_protocol::schema::MaybeUndefined;
use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate, ToolCallContent};
use base64::Engine;
use gpui_kit::{Image, ImageFormat};

use crate::ui::ChatView;

fn content_image(content: &ContentBlock) -> Option<Arc<Image>> {
    let ContentBlock::Image(image) = content else {
        return None;
    };
    let format = ImageFormat::from_mime_type(&image.mime_type)?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(&image.data).ok()?;
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

fn tool_images(contents: &[ToolCallContent]) -> Vec<Arc<Image>> {
    contents
        .iter()
        .filter_map(|content| match content {
            ToolCallContent::Content(content) => content_image(&content.content),
            _ => None,
        })
        .collect()
}

fn images(update: &SessionUpdate) -> Vec<Arc<Image>> {
    match update {
        SessionUpdate::UserMessageChunk(chunk)
        | SessionUpdate::AgentMessageChunk(chunk)
        | SessionUpdate::AgentThoughtChunk(chunk) => {
            content_image(&chunk.content).into_iter().collect()
        }
        SessionUpdate::ToolCall(tool) => tool_images(&tool.content),
        SessionUpdate::ToolCallUpdate(tool) => {
            tool_images(tool.fields.content.as_deref().unwrap_or_default())
        }
        SessionUpdate::CompactionUpdate(update) => match &update.summary {
            MaybeUndefined::Value(summary) => summary.iter().filter_map(content_image).collect(),
            _ => Vec::new(),
        },
        SessionUpdate::CompactionSummaryChunk(chunk) => {
            content_image(&chunk.content).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

impl ChatView {
    pub fn cache_message_images(&mut self, index: usize) {
        let Some(notification) = self.chat.messages()[index].updates().last() else {
            return;
        };
        let update = &notification.update;
        if let SessionUpdate::CompactionUpdate(update) = update
            && update.summary.is_undefined()
        {
            return;
        }
        let images = images(update);
        if matches!(update, SessionUpdate::CompactionSummaryChunk(_)) {
            if !images.is_empty() {
                self.images.entry(index).or_default().extend(images);
            }
        } else if images.is_empty() {
            self.images.remove(&index);
        } else {
            self.images.insert(index, images);
        }
    }

    pub fn rebuild_images(&mut self) {
        self.images.clear();
        for (index, message) in self.chat.messages().iter().enumerate() {
            let images = if let Some(compaction) = message.compaction() {
                match &compaction.summary {
                    MaybeUndefined::Value(summary) => {
                        summary.iter().filter_map(content_image).collect()
                    }
                    _ => Vec::new(),
                }
            } else {
                message
                    .updates()
                    .last()
                    .map(|notification| images(&notification.update))
                    .unwrap_or_default()
            };
            if !images.is_empty() {
                self.images.insert(index, images);
            }
        }
    }
}
