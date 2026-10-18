mod actions;
mod commands;
mod composer;
mod events;
mod layout;
mod load_session;
mod media;
mod new_chat;
mod palette;
mod session_actions;
mod session_list;
mod sidebar;
mod thought_message;
mod timeline;
mod user_message;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::AcpAgentConfig;
use agent_client_protocol::schema::v1::{SessionId, SessionNotification};
use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::acp::{Bridge, Permission};
use crate::preferences::{Appearance, Preferences};
use crate::session::chat::Chat;
use crate::session::deletion::SessionDeletion;
use crate::session::list::SessionList;
use crate::session::load::SessionLoad;
use crate::ui::commands::SlashMenu;
use crate::ui::new_chat::NewChatAction;

pub struct ChatView {
    cwd: PathBuf,
    choosing_directory: bool,
    session_directory_selected: bool,
    preferences_path: Option<PathBuf>,
    agent: AcpAgentConfig,
    appearance: Appearance,
    input: Entity<TextareaState>,
    command_menu: SlashMenu,
    chat: Chat,
    bridge: Option<Bridge>,
    receiver: Option<Task<()>>,
    session_id: Option<SessionId>,
    pending_updates: Vec<SessionNotification>,
    drafts: HashMap<SessionId, String>,
    loading_session: Option<SessionLoad>,
    creating_session: Option<NewChatAction>,
    load_supported: bool,
    session_list: SessionList,
    session_scroll: ScrollHandle,
    current_session_anchor: ScrollAnchor,
    session_deletion: SessionDeletion,
    disconnected: bool,
    stopping: bool,
    status: String,
    tokens: Option<u64>,
    permissions: VecDeque<Permission>,
    preference_error: Option<String>,
    scroll: ScrollHandle,
    expanded_thoughts: HashSet<usize>,
    collapsed_streaming_thoughts: HashSet<usize>,
    images: HashMap<usize, Vec<Arc<Image>>>,
    attachment_images: HashMap<(usize, usize), Arc<Image>>,
    _subscriptions: Vec<Subscription>,
}

impl fmt::Debug for ChatView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChatView")
            .field("cwd", &self.cwd)
            .field("appearance", &self.appearance)
            .field("session_id", &self.session_id)
            .field("status", &self.status)
            .field("message_count", &self.chat.messages().len())
            .field("disconnected", &self.disconnected)
            .finish_non_exhaustive()
    }
}

impl ChatView {
    pub fn new(
        cwd: PathBuf,
        preferences_path: Option<PathBuf>,
        agent: AcpAgentConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let appearance = preferences_path
            .as_deref()
            .and_then(|path| Preferences::load(path).ok())
            .unwrap_or_default()
            .appearance();
        palette::apply(appearance, cx);
        let system_theme = cx.observe_window_appearance(window, |this, window, cx| {
            if this.appearance == Appearance::System {
                palette::sync(window.appearance().into(), cx);
                cx.notify();
            }
        });
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("发送消息…")
                .auto_grow(2, 6)
                .submit_on_enter(true)
        });
        let changes = cx.subscribe_in(&input, window, |this, input, event, window, cx| {
            if let InputEvent::PressEnter { shift: false, .. } = event {
                let composing =
                    input.update(cx, |input, cx| input.marked_text_range(window, cx).is_some());
                if !composing && !this.complete_selected_command(window, cx) {
                    this.send(window, cx);
                }
            }
            match event {
                InputEvent::Change => this.command_menu.reset(),
                InputEvent::Focus => this.command_menu.focus(),
                InputEvent::Blur => this.command_menu.blur(),
                _ => {}
            }
            cx.notify();
        });
        let quit = cx.on_app_quit(|this, _| {
            this.dismiss_permissions();
            let bridge = this.bridge.take();
            async move {
                if let Some(bridge) = bridge {
                    bridge.shutdown().await;
                }
            }
        });
        cx.defer_in(window, |this, window, cx| this.connect(window, cx));
        let session_scroll = ScrollHandle::new();
        let current_session_anchor = ScrollAnchor::for_handle(session_scroll.clone());
        Self {
            cwd,
            choosing_directory: false,
            session_directory_selected: false,
            preferences_path,
            agent,
            appearance,
            input,
            command_menu: SlashMenu::default(),
            chat: Chat::default(),
            bridge: None,
            receiver: None,
            session_id: None,
            pending_updates: Vec::new(),
            drafts: HashMap::new(),
            loading_session: None,
            creating_session: None,
            load_supported: false,
            session_list: SessionList::default(),
            session_scroll,
            current_session_anchor,
            session_deletion: SessionDeletion::default(),
            disconnected: false,
            stopping: false,
            status: "就绪".into(),
            tokens: None,
            permissions: VecDeque::new(),
            preference_error: None,
            scroll: ScrollHandle::new(),
            expanded_thoughts: HashSet::new(),
            collapsed_streaming_thoughts: HashSet::new(),
            images: HashMap::new(),
            attachment_images: HashMap::new(),
            _subscriptions: vec![changes, quit, system_theme],
        }
    }

    fn set_appearance(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        palette::apply(appearance, cx);
        self.preference_error = match &self.preferences_path {
            Some(path) => Preferences::new(appearance)
                .save(path)
                .err()
                .map(|error| format!("主题已切换，但无法保存偏好：{error}")),
            None => Some("主题已切换；未找到可保存偏好的配置目录".into()),
        };
        cx.notify();
    }
}

impl Render for ChatView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_layout(cx)
    }
}
