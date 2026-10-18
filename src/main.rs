#![warn(missing_debug_implementations)]

mod acp;
mod chat;
mod preferences;
mod session_actions;
mod session_list;
mod session_load;
mod session_update;
mod ui;

use gpui_kit::{assets::AllAssets, *};

use crate::{acp::aries_config, preferences::Preferences, ui::ChatView};

gpui_kit::actions!(orbit, [Quit]);

fn main() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?.canonicalize()?;
    gpui_kit::application()
        .with_assets(AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("ctrl-q", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.), px(800.)),
                    cx,
                ))),
                window_min_size: Some(size(px(760.), px(520.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Aries Orbit".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
                window.on_window_should_close(cx, |_, cx| {
                    cx.quit();
                    false
                });
                cx.new(|cx| ChatView::new(cwd, Preferences::path(), aries_config(), window, cx))
            }) {
                eprintln!("无法创建窗口：{error:#}");
                cx.quit();
            }
            cx.activate(true);
        });
    Ok(())
}
