mod list;
mod project;
mod session;
mod toolbar;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::preferences::Appearance;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn sidebar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace_path = self.cwd.to_string_lossy().into_owned();
        div()
            .id("sidebar")
            .w(px(288.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(p.sidebar())
            .border_r_1()
            .border_color(p.border())
            .child(
                div()
                    .h(px(56.))
                    .flex_shrink_0()
                    .px_4()
                    .flex()
                    .items_center()
                    .gap_3()
                    .border_b_1()
                    .border_color(p.border())
                    .child(
                        div()
                            .size(px(32.))
                            .rounded(px(12.))
                            .bg(p.foreground())
                            .text_color(p.sidebar())
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
                                    .text_color(p.muted())
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
                        .disabled(!self.can_start_new_chat())
                        .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx))),
                ),
            )
            .child(self.render_sessions(p, cx))
            .child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(p.border())
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
                            .text_color(p.muted())
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
                            .border_color(p.border())
                            .child(
                                div()
                                    .px_2()
                                    .pt_1()
                                    .text_size(px(12.))
                                    .text_color(p.muted())
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
                                                button.bg(p.hover())
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
                                                button.bg(p.hover())
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
                                                button.bg(p.hover())
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.set_appearance(Appearance::Dark, cx)
                                            })),
                                    ),
                            ),
                    )
                    .when_some(self.preference_error.clone(), |footer, error| {
                        footer.child(div().text_size(px(11.)).text_color(p.danger()).child(error))
                    }),
            )
    }
}
