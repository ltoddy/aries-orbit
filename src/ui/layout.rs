use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, ButtonVariants},
        input::Textarea,
        tooltip::Tooltip,
    },
    prelude::*,
    *,
};

use crate::{
    preferences::Appearance,
    ui::{ChatView, palette::Palette},
};

impl ChatView {
    pub fn render_layout(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(cx.theme().mode);
        div()
            .size_full()
            .flex()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(".SystemUIFont")
            .text_size(px(14.))
            .child(self.sidebar(p, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.timeline(p, cx))
                    .child(self.composer(p, cx)),
            )
    }

    fn sidebar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace_path = self.cwd.to_string_lossy().into_owned();
        div()
            .id("sidebar")
            .w(px(288.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(p.sidebar)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .h(px(56.))
                    .flex_shrink_0()
                    .px_4()
                    .flex()
                    .items_center()
                    .gap_3()
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .size(px(32.))
                            .rounded(px(12.))
                            .bg(p.foreground)
                            .text_color(p.sidebar)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Bot).size(px(17.))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Aries Orbit"))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child("与你的 Agent 一起工作"),
                            ),
                    ),
            )
            .child(
                div().p_3().child(
                    Button::new("new-chat")
                        .label("新建对话")
                        .icon(Icon::new(IconName::Plus).size(px(15.)))
                        .w_full()
                        .h(px(38.))
                        .rounded(px(12.))
                        .disabled(self.chat.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx))),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .child(
                        div()
                            .px_2()
                            .mb_2()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child("对话"),
                    )
                    .child(
                        div()
                            .id("current-chat")
                            .px_3()
                            .py_2()
                            .rounded(px(12.))
                            .bg(p.hover)
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.chat.title()),
                            )
                            .child(div().mt_1().text_size(px(12.)).text_color(p.muted).child(
                                if self.session_id.is_some() {
                                    "当前会话"
                                } else {
                                    "发送消息开始对话"
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(p.border)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_1()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(Icon::new(IconName::Folder).size(px(13.)))
                            .id("workspace")
                            .tooltip(move |window, cx| {
                                Tooltip::new(workspace_path.clone()).build(window, cx)
                            })
                            .child(
                                div().truncate().child(
                                    self.cwd
                                        .file_name()
                                        .unwrap_or(self.cwd.as_os_str())
                                        .to_string_lossy()
                                        .into_owned(),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_1()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(p.border)
                            .child(
                                div()
                                    .px_2()
                                    .pt_1()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child("主题"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .w_full()
                                    .gap_1()
                                    .child(
                                        Button::new("theme-system")
                                            .label("系统")
                                            .tooltip("跟随系统主题")
                                            .ghost()
                                            .small()
                                            .flex_1()
                                            .min_w_0()
                                            .rounded(px(8.))
                                            .when(self.appearance == Appearance::System, |button| {
                                                button.bg(p.hover)
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.set_appearance(Appearance::System, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("theme-light")
                                            .label("Light")
                                            .icon(Icon::new(IconName::Sun).size(px(13.)))
                                            .ghost()
                                            .small()
                                            .flex_1()
                                            .min_w_0()
                                            .rounded(px(8.))
                                            .when(self.appearance == Appearance::Light, |button| {
                                                button.bg(p.hover)
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.set_appearance(Appearance::Light, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("theme-dark")
                                            .label("Dark")
                                            .icon(Icon::new(IconName::Moon).size(px(13.)))
                                            .ghost()
                                            .small()
                                            .flex_1()
                                            .min_w_0()
                                            .rounded(px(8.))
                                            .when(self.appearance == Appearance::Dark, |button| {
                                                button.bg(p.hover)
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.set_appearance(Appearance::Dark, cx)
                                            })),
                                    ),
                            ),
                    )
                    .when_some(self.preference_error.clone(), |footer, error| {
                        footer.child(div().text_size(px(11.)).text_color(p.danger).child(error))
                    }),
            )
    }

    fn composer(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = !self.input.read(cx).value().trim().is_empty() && !self.disconnected;
        div().flex_shrink_0().p_4().bg(p.background).child(
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
                            .border_color(p.border)
                            .bg(p.input)
                            .child(
                                div()
                                    .mb_2()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(permission.title.clone()),
                            )
                            .child(
                                div().flex().flex_wrap().gap_2().children(
                                    permission.options.iter().enumerate().map(
                                        |(index, (id, name))| {
                                            let id = id.clone();
                                            Button::new(("permission", index))
                                                .label(name.clone())
                                                .small()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.choose_permission(id.clone(), cx)
                                                }))
                                        },
                                    ),
                                ),
                            ),
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
                        .text_color(p.muted)
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
                        .border_color(p.border)
                        .bg(p.input)
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
                                    Icon::new(if self.chat.busy {
                                        IconName::Square
                                    } else {
                                        IconName::Send
                                    })
                                    .size(px(17.)),
                                )
                                .accessibility_label(if self.chat.busy {
                                    "停止回复"
                                } else {
                                    "发送消息"
                                })
                                .size(px(40.))
                                .rounded(px(14.))
                                .disabled(if self.chat.busy {
                                    self.stopping
                                } else {
                                    !can_send
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if this.chat.busy {
                                        this.stop(cx);
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
                        .text_color(p.muted)
                        .child("Enter 发送，Shift + Enter 换行"),
                ),
        )
    }
}
