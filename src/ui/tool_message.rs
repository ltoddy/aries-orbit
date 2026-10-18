use agent_client_protocol::schema::v1::{ToolCallStatus, ToolKind};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::tool_call::ToolCallState;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

#[derive(Clone, Copy, Debug)]
pub struct ToolExpansion {
    pub expanded: bool,
    pub failure_revision: u64,
}

fn tool_icon(kind: ToolKind) -> IconName {
    match kind {
        ToolKind::Read => IconName::Eye,
        ToolKind::Edit => IconName::FilePenLine,
        ToolKind::Delete => IconName::Trash,
        ToolKind::Move => IconName::FileSymlink,
        ToolKind::Search => IconName::Search,
        ToolKind::Execute => IconName::SquareTerminal,
        ToolKind::Think => IconName::Brain,
        ToolKind::Fetch => IconName::Globe,
        ToolKind::SwitchMode => IconName::Settings,
        _ => IconName::Wrench,
    }
}

impl ChatView {
    pub fn render_tool_message(
        &self,
        index: usize,
        tool: &ToolCallState,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let failed = tool.call().status == ToolCallStatus::Failed;
        let failure_revision = tool.failure_revision();
        let expanded = self
            .tool_expansion
            .get(&index)
            .filter(|state| state.failure_revision == failure_revision)
            .map_or(failed, |state| state.expanded);
        let (status, status_icon) = if tool.is_interrupted() {
            ("已中断", IconName::CirclePause)
        } else {
            match tool.call().status {
                ToolCallStatus::Pending => ("等待中", IconName::Clock),
                ToolCallStatus::InProgress => ("进行中", IconName::Loader),
                ToolCallStatus::Completed => ("已完成", IconName::CircleCheck),
                ToolCallStatus::Failed => ("失败", IconName::CircleX),
                _ => ("未知状态", IconName::CircleQuestionMark),
            }
        };
        let color = if failed { p.danger() } else { p.muted() };
        let details = format!("{} · {status}", tool.title());
        div()
            .id(("message", index))
            .aria_label(format!("工具调用：{} · {status}", tool.title()))
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id(("tool-header", index))
                    .w_full()
                    .min_w_0()
                    .tooltip(move |window, cx| {
                        let details = details.clone();
                        Tooltip::element(move |_, _| {
                            div().max_w(px(480.)).whitespace_normal().child(details.clone())
                        })
                        .build(window, cx)
                    })
                    .child(
                        Button::new(("toggle-tool", index))
                            .ghost()
                            .xsmall()
                            .w_full()
                            .h(px(24.))
                            .px_0()
                            .cursor_pointer()
                            .text_color(p.muted())
                            .accessibility_label(format!(
                                "{}工具详情：{} · {status}",
                                if expanded { "收起" } else { "展开" },
                                tool.title()
                            ))
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .justify_start()
                                    .gap_2()
                                    .text_size(px(12.))
                                    .text_left()
                                    .child(
                                        Icon::new(if expanded {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        })
                                        .size(px(14.))
                                        .flex_shrink_0(),
                                    )
                                    .child(
                                        Icon::new(tool_icon(tool.call().kind))
                                            .size(px(14.))
                                            .flex_shrink_0(),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .line_clamp(1)
                                            .child(tool.title().to_owned()),
                                    )
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .text_color(color)
                                            .child(
                                                if tool.call().status == ToolCallStatus::InProgress
                                                    && !tool.is_interrupted()
                                                {
                                                    Spinner::new()
                                                        .small()
                                                        .color(color)
                                                        .into_any_element()
                                                } else {
                                                    Icon::new(status_icon)
                                                        .size(px(14.))
                                                        .into_any_element()
                                                },
                                            )
                                            .when(failed || tool.is_interrupted(), |status_row| {
                                                status_row.child(status)
                                            }),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.tool_expansion.insert(
                                    index,
                                    ToolExpansion { expanded: !expanded, failure_revision },
                                );
                                cx.notify();
                            })),
                    ),
            )
            .when(expanded, |row| row.child(self.render_tool_content(index, tool, p)))
            .into_any_element()
    }
}
