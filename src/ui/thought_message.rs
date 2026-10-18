use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::chat::Message;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn render_thought_message(
        &mut self,
        index: usize,
        message: &Message,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let expanded = if self.chat.is_streaming(index) {
            !self.collapsed_streaming_thoughts.contains(&index)
        } else {
            self.expanded_thoughts.contains(&index)
        };
        div()
            .id(("message", index))
            .aria_label(message.title().to_owned())
            .aria_value(message.text().to_owned())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Button::new(("toggle-thought", index))
                    .ghost()
                    .xsmall()
                    .w_full()
                    .h(px(24.))
                    .px_0()
                    .cursor_pointer()
                    .text_color(p.muted())
                    .accessibility_label(if expanded {
                        "收起思考过程"
                    } else {
                        "展开思考过程"
                    })
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
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
                            .child(Icon::new(IconName::Brain).size(px(14.)).flex_shrink_0())
                            .child(message.title().to_owned()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let toggled = if this.chat.is_streaming(index) {
                            &mut this.collapsed_streaming_thoughts
                        } else {
                            &mut this.expanded_thoughts
                        };
                        if !toggled.remove(&index) {
                            toggled.insert(index);
                        }
                        cx.notify();
                    })),
            )
            .when(expanded, |body| {
                body.child(
                    div().w_full().min_w_0().child(
                        TextView::new(&self.markdown_state(index, message.text(), cx))
                            .selectable(true)
                            .scrollable(false)
                            .w_full()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.muted()),
                    ),
                )
            })
            .into_any_element()
    }
}
