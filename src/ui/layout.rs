use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn render_layout(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(cx.theme().mode);
        div()
            .size_full()
            .flex()
            .bg(p.background())
            .text_color(p.foreground())
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
                    .child(
                        div()
                            .h(px(44.))
                            .flex_shrink_0()
                            .px_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .child(self.render_theme_button(p, cx)),
                    )
                    .child(self.render_timeline(p, cx))
                    .child(self.composer(p, cx)),
            )
    }
}
