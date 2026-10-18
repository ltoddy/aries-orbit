use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate, TerminalId, ToolCallContent};
use gpui_kit::component::diff::{Diff, DiffFile, DiffState};
use gpui_kit::component::text::TextView;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::display::code_block;
use crate::session::tool_call::ToolCallState;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

fn output(id: impl Into<ElementId>, text: &str, p: Palette) -> AnyElement {
    let id = id.into();
    div()
        .id(id.clone())
        .w_full()
        .min_w_0()
        .max_h(px(280.))
        .overflow_y_scroll()
        .child(
            TextView::markdown(id, code_block("", text))
                .selectable(true)
                .scrollable(false)
                .w_full()
                .text_size(px(12.))
                .line_height(px(20.))
                .text_color(p.muted()),
        )
        .into_any_element()
}

fn raw_text(value: &serde_json::Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_else(|| format!("{value:#}"))
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
        for index in 0..self.chat.messages().len() {
            self.build_tool_diffs(index, cx);
        }
    }

    fn build_tool_diffs(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tool) = self.chat.messages()[index].tool_call() else {
            return;
        };
        self.tool_diffs.retain(|(message, _), _| *message != index);
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
                    self.tool_diffs
                        .insert((index, content_index), cx.new(|cx| DiffState::new(files, cx)));
                }
            }
        }
    }

    fn render_tool_terminal(
        &self,
        index: usize,
        id: SharedString,
        terminal_id: &TerminalId,
        p: Palette,
    ) -> AnyElement {
        let session_id = &self.chat.messages()[index].updates()[0].session_id;
        let Some(terminal) = self.terminal_outputs.get(&(session_id.clone(), terminal_id.clone()))
        else {
            return div()
                .child(if self.chat.is_busy() {
                    "等待终端输出…"
                } else {
                    "该终端的历史输出不可用"
                })
                .into_any_element();
        };
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .when(terminal.truncated, |view| view.child("仅显示最近的终端输出"))
            .child(output(id, &terminal.output, p))
            .when_some(terminal.exit_status.as_ref(), |view, status| {
                let failed =
                    status.exit_code.is_some_and(|code| code != 0) || status.signal.is_some();
                view.child(div().when(failed, |line| line.text_color(p.danger())).child(
                    match (&status.signal, status.exit_code) {
                        (Some(signal), _) => format!("已终止 · 信号 {signal}"),
                        (_, Some(code)) => format!("退出码 {code}"),
                        _ => "已结束".into(),
                    },
                ))
            })
            .into_any_element()
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
            .gap_2()
            .text_size(px(12.))
            .text_color(p.muted())
            .when_some(call.raw_input.as_ref(), |details, input| {
                details.child(div().w_full().child("输入").child(output(
                    ("tool-input", index),
                    &raw_text(input),
                    p,
                )))
            })
            .children(call.locations.iter().map(|location| {
                let path = location
                    .path
                    .strip_prefix(&self.cwd)
                    .unwrap_or(&location.path)
                    .to_string_lossy();
                div().whitespace_normal().child(match location.line {
                    Some(line) => format!("{path}:{line}"),
                    None => path.into_owned(),
                })
            }))
            .children(call.content.iter().enumerate().map(|(content_index, content)| {
                let id = SharedString::from(format!("tool-content:{index}:{content_index}"));
                match content {
                    ToolCallContent::Content(content) => match &content.content {
                        ContentBlock::Text(text) => output(id, &text.text, p),
                        attachment => self.render_attachment(index, content_index, attachment, p),
                    },
                    ToolCallContent::Diff(diff) => {
                        match self.tool_diffs.get(&(index, content_index)) {
                            Some(state) => div()
                                .id(id)
                                .w_full()
                                .h(px(280.))
                                .child(Diff::new(state).soft_wrap(true))
                                .into_any_element(),
                            None => div()
                                .w_full()
                                .child(diff.path.to_string_lossy().into_owned())
                                .child(output(
                                    id,
                                    &format!(
                                        "修改前\n{}\n\n修改后\n{}",
                                        diff.old_text.as_deref().unwrap_or("（新建文件）"),
                                        diff.new_text
                                    ),
                                    p,
                                ))
                                .into_any_element(),
                        }
                    }
                    ToolCallContent::Terminal(terminal) => {
                        self.render_tool_terminal(index, id, &terminal.terminal_id, p)
                    }
                    _ => output(id, &format!("{content:?}"), p),
                }
            }))
            .when(
                call.content.is_empty()
                    || call.status == agent_client_protocol::schema::v1::ToolCallStatus::Failed,
                |details| {
                    details.when_some(call.raw_output.as_ref(), |details, value| {
                        details.child(output(("tool-output", index), &raw_text(value), p))
                    })
                },
            )
            .when(call.content.is_empty() && call.raw_output.is_none(), |details| {
                details.child(
                    if call.status == agent_client_protocol::schema::v1::ToolCallStatus::Failed {
                        "工具执行失败，未提供错误详情。"
                    } else {
                        "暂无输出"
                    },
                )
            })
            .into_any_element()
    }
}
