use std::path::PathBuf;

use gpui_kit::*;

use crate::ui::ChatView;

#[derive(Clone, Copy, Debug)]
pub enum NewChatAction {
    Create,
    SendPrompt,
}

impl ChatView {
    pub fn can_start_new_chat(&self) -> bool {
        self.is_session_idle()
    }

    pub fn choose_session_directory(
        &mut self,
        action: NewChatAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_start_new_chat() {
            return;
        }
        self.choosing_directory = true;
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("选择会话工作目录".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result: anyhow::Result<Option<PathBuf>> = async {
                let Some(path) = selection.await??.and_then(|paths| paths.into_iter().next())
                else {
                    return Ok(None);
                };
                let directory = cx
                    .background_executor()
                    .spawn(async move {
                        let directory = path.canonicalize()?;
                        if !directory.is_dir() {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::NotADirectory,
                                "请选择文件夹",
                            ));
                        }
                        Ok(directory)
                    })
                    .await?;
                Ok(Some(directory))
            }
            .await;
            let _ = view.update_in(cx, |this, window, cx| {
                this.choosing_directory = false;
                match result {
                    Ok(Some(directory)) => this.start_new_chat(directory, action, window, cx),
                    Ok(None) => {}
                    Err(error) => {
                        this.status = "选择目录失败".into();
                        this.chat.finish(Some(format!("无法选择工作目录：{error}")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn start_new_chat(
        &mut self,
        directory: PathBuf,
        action: NewChatAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_start_new_chat() {
            return;
        }
        self.dismiss_permissions();
        self.save_draft(cx);
        if self.disconnected {
            self.receiver = None;
            self.bridge = None;
        }
        self.reset_chat();
        self.cwd = directory;
        self.session_directory_selected = true;
        self.disconnected = false;
        self.input.update(cx, |input, cx| {
            if matches!(action, NewChatAction::Create) {
                input.set_value("", window, cx);
            }
            input.focus(window, cx);
        });
        self.connect(window, cx);
        if matches!(action, NewChatAction::SendPrompt) {
            self.send(window, cx);
        }
        cx.notify();
    }
}
