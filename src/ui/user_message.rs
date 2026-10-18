use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::session::chat::Message;
use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn render_user_message(&self, index: usize, message: &Message, p: Palette) -> AnyElement {
        div()
            .id(("message", index))
            .aria_label("用户消息")
            .aria_value(message.text().to_owned())
            .w_full()
            .flex()
            .justify_end()
            .child(
                div()
                    .min_w_0()
                    .max_w(relative(0.72))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(16.))
                    .bg(p.bubble())
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(message.attachments().map(|(attachment_index, content)| {
                        self.render_attachment(index, attachment_index, content, p)
                    }))
                    .when(!message.text().is_empty(), |bubble| {
                        bubble.child(
                            div()
                                .whitespace_normal()
                                .line_height(px(24.))
                                .child(message.text().to_owned()),
                        )
                    }),
            )
            .into_any_element()
    }
}
