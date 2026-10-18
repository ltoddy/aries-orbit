use agent_client_protocol::{
    Error,
    schema::v1::{LoadSessionRequest, SessionId, SessionInfo},
};
use gpui_kit::*;

use crate::{acp::Command, session_load::SessionLoad, ui::ChatView};

impl ChatView {
    pub fn can_load_sessions(&self) -> bool {
        self.load_supported
            && !self.disconnected
            && !self.chat.is_busy()
            && !self.session_actions.is_busy()
            && !self.session_list.is_loading()
            && self.loading_session.is_none()
            && !self.choosing_directory
    }

    pub fn load_session(&mut self, session: SessionInfo, cx: &mut Context<Self>) {
        if !self.can_load_sessions() || self.session_id.as_ref() == Some(&session.session_id) {
            return;
        }
        let request = LoadSessionRequest::new(session.session_id.clone(), session.cwd.clone());
        if !self
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(Command::LoadSession(Box::new(request))).is_ok())
        {
            self.status = "加载失败".into();
            self.chat
                .finish(Some("无法发送加载请求，请重新连接后重试。".into()));
            cx.notify();
            return;
        }
        self.dismiss_permissions();
        self.chat.finish(None);
        self.loading_session = Some(SessionLoad::new(session));
        self.status = "正在加载会话…".into();
        cx.notify();
    }

    pub fn finish_session_load(
        &mut self,
        id: SessionId,
        result: Result<(), Error>,
        cx: &mut Context<Self>,
    ) {
        if !self
            .loading_session
            .as_ref()
            .is_some_and(|loading| loading.session_id() == &id)
        {
            return;
        }
        let Some(loading) = self.loading_session.take() else {
            return;
        };
        self.dismiss_permissions();
        match result {
            Ok(()) => {
                let (session, mut chat, tokens) = loading.into_parts();
                chat.end_segment();
                self.reset_chat();
                self.chat = chat;
                self.cwd = session.cwd;
                self.session_id = Some(session.session_id);
                self.tokens = tokens;
                for index in 0..self.chat.messages().len() {
                    self.cache_message_images(index);
                }
                self.scroll.scroll_to_bottom();
            }
            Err(error) => {
                self.status = "加载失败".into();
                self.chat.finish(Some(format!("无法加载会话：{error}")));
            }
        }
        if self.session_list.take_refresh_request() {
            self.refresh_sessions(cx);
        }
        cx.notify();
    }
}
