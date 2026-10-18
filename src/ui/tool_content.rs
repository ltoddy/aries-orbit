use agent_client_protocol::schema::v1::{SessionUpdate, ToolCallContent};
use gpui_kit::component::diff::{Diff, DiffFile, DiffState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::tool_call::ToolCallState;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

fn diff_height(rows: usize) -> Pixels {
    px(rows.max(4) as f32 * 26.)
}

fn diff_rows(patch: &str) -> usize {
    patch
        .lines()
        .filter(|line| {
            line.starts_with("+++")
                || line.starts_with("---")
                || line.starts_with("@@")
                || line.starts_with('+')
                || line.starts_with('-')
                || line.starts_with(' ')
                || line.starts_with("\\ ")
        })
        .count()
}

impl ChatView {
    pub fn cache_tool_diffs(&mut self, index: usize, cx: &mut Context<Self>) {
        let message = &self.chat.messages()[index];
        let Some(_) = message.tool_call() else {
            return;
        };
        if matches!(message.updates().last().map(|update| &update.update), Some(SessionUpdate::ToolCallUpdate(update)) if update.fields.content.is_none())
        {
            return;
        }
        self.build_tool_diffs(index, cx);
    }

    pub fn rebuild_tool_diffs(&mut self, cx: &mut Context<Self>) {
        self.tool_diffs.clear();
        self.tool_diff_rows.clear();
        for index in 0..self.chat.messages().len() {
            self.build_tool_diffs(index, cx);
        }
    }

    fn build_tool_diffs(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tool) = self.chat.messages()[index].tool_call() else {
            return;
        };
        self.tool_diffs.retain(|(message, _), _| *message != index);
        self.tool_diff_rows.retain(|(message, _), _| *message != index);
        for (content_index, content) in tool.call().content.iter().enumerate() {
            if let ToolCallContent::Diff(diff) = content {
                let old = diff.old_text.as_deref().unwrap_or_default();
                let path = diff.path.to_string_lossy().replace(['\n', '\r', '\t'], " ");
                let patch = similar::TextDiff::configure()
                    .timeout(std::time::Duration::from_millis(100))
                    .diff_lines(old, &diff.new_text)
                    .unified_diff()
                    .context_radius(3)
                    .header(if diff.old_text.is_some() { &path } else { "/dev/null" }, &path)
                    .to_string();
                if let Ok(files) = DiffFile::parse(&patch)
                    && !files.is_empty()
                {
                    self.tool_diff_rows.insert((index, content_index), diff_rows(&patch));
                    self.tool_diffs
                        .insert((index, content_index), cx.new(|cx| DiffState::new(files, cx)));
                }
            }
        }
    }

    pub fn render_tool_content(
        &self,
        index: usize,
        tool: &ToolCallState,
        p: Palette,
    ) -> AnyElement {
        let call = tool.call();
        div()
            .id(("tool-details", index))
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .text_size(px(12.))
            .text_color(p.muted())
            .children(call.content.iter().enumerate().filter_map(|(content_index, content)| {
                let ToolCallContent::Diff(diff) = content else {
                    return None;
                };
                let id = SharedString::from(format!("tool-content:{index}:{content_index}"));
                Some(match self.tool_diffs.get(&(index, content_index)) {
                    Some(state) => div()
                        .id(id)
                        .w_full()
                        .h(diff_height(
                            self.tool_diff_rows.get(&(index, content_index)).copied().unwrap_or(10),
                        ))
                        .child(Diff::new(state).soft_wrap(true))
                        .into_any_element(),
                    None => div()
                        .w_full()
                        .child(diff.path.to_string_lossy().into_owned())
                        .into_any_element(),
                })
            }))
            .into_any_element()
    }
}
