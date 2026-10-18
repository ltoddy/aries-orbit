use gpui_kit::{
    assets::IconName,
    component::{
        Icon, Sizable,
        button::{Button, ButtonVariants},
        text::TextView,
    },
    prelude::*,
    *,
};

use super::{ChatView, palette::Palette};
use crate::{
    chat::{Message, Role},
    session_update::code_block,
};

impl ChatView {
    pub fn timeline(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        if self.chat.messages.is_empty() {
            return div()
                .flex_1()
                .min_h_0()
                .px_6()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_4()
                        .max_w(px(480.))
                        .text_center()
                        .child(
                            div()
                                .size(px(56.))
                                .rounded(px(16.))
                                .border_1()
                                .border_color(p.border)
                                .bg(p.input)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Icon::new(IconName::Bot).size(px(26.))),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(22.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("开始与 Aries 对话"),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(24.))
                                .text_color(p.muted)
                                .child("聊聊你的想法，或让 Aries 帮你完成项目中的任务。"),
                        ),
                )
                .into_any_element();
        }
        div()
            .id("timeline")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .px_6()
            .py_8()
            .child(
                div()
                    .max_w(px(768.))
                    .w_full()
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .children(
                        self.chat
                            .messages
                            .iter()
                            .enumerate()
                            .map(|(index, message)| self.timeline_item(index, message, p, cx)),
                    )
                    .when(self.chat.busy, |list| {
                        list.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(self.status.clone()),
                        )
                    })
                    .when_some(self.chat.error.clone(), |list, error| {
                        list.child(
                            div()
                                .id("chat-error")
                                .rounded(px(12.))
                                .p_3()
                                .border_1()
                                .border_color(p.border)
                                .text_color(p.danger)
                                .child(error),
                        )
                    }),
            )
            .into_any_element()
    }

    fn timeline_item(
        &self,
        index: usize,
        message: &Message,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let user = message.role == Role::User;
        let card = matches!(message.role, Role::Thought | Role::Event);
        let expanded = self.expanded_updates.contains(&index);
        let body = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .when(card, |body| {
                body.p_3()
                    .rounded(px(12.))
                    .bg(p.input)
                    .border_1()
                    .border_color(p.border)
            })
            .when(card || !message.updates.is_empty(), |body| {
                body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .when(card, |header| {
                            header.child(
                                Icon::new(if message.role == Role::Thought {
                                    IconName::Brain
                                } else {
                                    IconName::Activity
                                })
                                .size(px(14.)),
                            )
                        })
                        .child(message.title.clone()),
                )
            })
            .child(if user {
                div()
                    .line_height(px(24.))
                    .child(message.text.clone())
                    .into_any_element()
            } else {
                TextView::markdown(
                    ("markdown", index),
                    if message.text.is_empty() {
                        "（空内容）".into()
                    } else {
                        message.text.clone()
                    },
                )
                .selectable(true)
                .scrollable(false)
                .w_full()
                .text_size(px(14.))
                .line_height(px(25.))
                .into_any_element()
            })
            .when_some(self.images.get(&index), |body, images| {
                body.child(
                    div()
                        .id(("images", index))
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(images.iter().map(|image| {
                            img(image.clone())
                                .w_full()
                                .h(px(200.))
                                .object_fit(ObjectFit::Contain)
                                .with_fallback(|| {
                                    div()
                                        .child("图片无法预览，可展开完整数据查看")
                                        .into_any_element()
                                })
                        })),
                )
            })
            .when(!message.updates.is_empty(), |body| {
                body.child(
                    Button::new(("update-details", index))
                        .ghost()
                        .xsmall()
                        .self_start()
                        .label(if expanded {
                            "收起完整数据"
                        } else {
                            "完整数据"
                        })
                        .icon(
                            Icon::new(if expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .size(px(12.)),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.expanded_updates.remove(&index) {
                                this.expanded_updates.insert(index);
                            }
                            cx.notify();
                        })),
                )
            })
            .when(expanded, |body| {
                let raw = serde_json::to_string_pretty(&message.updates)
                    .expect("received ACP notifications serialize");
                body.child(
                    div()
                        .id(("raw-update", index))
                        .aria_value(raw.clone())
                        .max_h(px(320.))
                        .overflow_y_scroll()
                        .p_2()
                        .rounded(px(8.))
                        .bg(p.background)
                        .child(
                            TextView::markdown(("raw-markdown", index), code_block("json", &raw))
                                .selectable(true)
                                .scrollable(false)
                                .w_full()
                                .text_size(px(12.)),
                        ),
                )
            });
        div()
            .id(("message", index))
            .aria_label(message.title.clone())
            .aria_value(message.text.clone())
            .w_full()
            .flex()
            .when(user, |row| row.justify_end())
            .child(
                div()
                    .min_w_0()
                    .when(user, |bubble| {
                        bubble
                            .max_w(relative(0.72))
                            .px_3()
                            .py_2()
                            .rounded(px(16.))
                            .bg(p.bubble)
                    })
                    .when(!user, |bubble| bubble.w_full())
                    .child(body),
            )
            .into_any_element()
    }
}
