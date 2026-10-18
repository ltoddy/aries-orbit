use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Textarea;
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn composer(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = self.can_send(cx);
        div().flex_shrink_0().p_4().bg(p.background()).child(
            div()
                .w_full()
                .max_w(px(768.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_2()
                .when_some(self.permissions.front(), |composer, permission| {
                    composer.child(
                        div()
                            .p_3()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(p.border())
                            .bg(p.input())
                            .child(
                                div()
                                    .mb_2()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(permission.title().to_owned()),
                            )
                            .child(div().flex().flex_wrap().gap_2().children(
                                permission.options().iter().enumerate().map(|(index, option)| {
                                    let id = option.option_id.clone();
                                    Button::new(("permission", index))
                                        .label(option.name.clone())
                                        .small()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.choose_permission(id.clone(), cx)
                                        }))
                                }),
                            )),
                    )
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .px_2()
                        .text_size(px(12.))
                        .text_color(p.muted())
                        .child(
                            div()
                                .id("status")
                                .aria_value(self.status.clone())
                                .truncate()
                                .child(self.status.clone()),
                        )
                        .when_some(self.tokens, |row, tokens| {
                            row.child(format!("{tokens} tokens"))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap_3()
                        .p_2()
                        .rounded(px(24.))
                        .border_1()
                        .border_color(p.border())
                        .bg(p.input())
                        .child(
                            div().id("composer-input").flex_1().min_w_0().px_2().child(
                                Textarea::new(&self.input)
                                    .appearance(false)
                                    .bordered(false)
                                    .aria_label("消息")
                                    .w_full(),
                            ),
                        )
                        .child(
                            Button::new("send")
                                .primary()
                                .icon(
                                    Icon::new(if self.chat.is_busy() {
                                        IconName::Square
                                    } else {
                                        IconName::Send
                                    })
                                    .size(px(17.)),
                                )
                                .accessibility_label(if self.chat.is_busy() {
                                    "停止回复"
                                } else {
                                    "发送消息"
                                })
                                .size(px(40.))
                                .rounded(px(14.))
                                .disabled(if self.chat.is_busy() {
                                    self.stopping
                                } else {
                                    !can_send
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if this.chat.is_busy() {
                                        this.stop(window, cx);
                                    } else {
                                        this.send(window, cx);
                                    }
                                })),
                        ),
                )
                .child(
                    div()
                        .px_2()
                        .text_size(px(11.))
                        .text_color(p.muted())
                        .child("Enter 发送，Shift + Enter 换行"),
                ),
        )
    }
}
