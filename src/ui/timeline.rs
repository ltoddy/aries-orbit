use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::text::TextView;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::chat::{Message, Role};
use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn render_timeline(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        if self.chat.messages().is_empty()
            && self.chat.error().is_none()
            && self.loading_session.is_none()
        {
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
                                .border_color(p.border())
                                .bg(p.input())
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
                                .text_color(p.muted())
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
                            .messages()
                            .iter()
                            .enumerate()
                            .map(|(index, message)| self.timeline_item(index, message, p, cx)),
                    )
                    .when(self.chat.is_busy() || self.loading_session.is_some(), |list| {
                        list.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted())
                                .child(self.status.clone()),
                        )
                    })
                    .when_some(self.chat.error().map(str::to_owned), |list, error| {
                        list.child(
                            div()
                                .id("chat-error")
                                .rounded(px(12.))
                                .p_3()
                                .border_1()
                                .border_color(p.border())
                                .text_color(p.danger())
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
        if let Some(tool) = message.tool_call() {
            return self.render_tool_message(index, tool, p, cx);
        }
        if message.role() == Role::User {
            return self.render_user_message(index, message, p);
        }
        if message.role() == Role::Thought {
            return self.render_thought_message(index, message, p, cx);
        }
        let card = message.role() == Role::Event;
        let body = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .when(card, |body| {
                body.p_3().rounded(px(12.)).bg(p.input()).border_1().border_color(p.border())
            })
            .when(card || !message.updates().is_empty(), |body| {
                body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(p.muted())
                        .when(card, |header| {
                            header.child(Icon::new(IconName::Activity).size(px(14.)))
                        })
                        .child(message.title().to_owned()),
                )
            })
            .child(
                TextView::markdown(
                    ("markdown", index),
                    if message.text().is_empty() {
                        "（空内容）".into()
                    } else {
                        message.text().to_owned()
                    },
                )
                .selectable(true)
                .scrollable(false)
                .w_full()
                .text_size(px(14.))
                .line_height(px(25.))
                .into_any_element(),
            )
            .when_some(self.images.get(&index), |body, images| {
                body.child(div().id(("images", index)).w_full().flex().flex_col().gap_2().children(
                    images.iter().map(|image| {
                        img(image.clone())
                            .w_full()
                            .h(px(200.))
                            .object_fit(ObjectFit::Contain)
                            .with_fallback(|| div().child("图片无法预览").into_any_element())
                    }),
                ))
            });
        div()
            .id(("message", index))
            .aria_label(message.title().to_owned())
            .aria_value(message.text().to_owned())
            .w_full()
            .flex()
            .child(div().min_w_0().w_full().child(body))
            .into_any_element()
    }
}
