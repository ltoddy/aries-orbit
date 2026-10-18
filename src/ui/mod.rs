mod layout;
mod load_session;
mod media;
mod new_chat;
mod palette;
mod session_actions;
mod sessions;
mod timeline;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    path::PathBuf,
    sync::Arc,
};

use agent_client_protocol::{
    AcpAgentConfig,
    schema::v1::{PermissionOptionId, SessionId, SessionUpdate, StopReason},
};
use gpui_kit::{
    component::input::{InputEvent, TextareaState},
    prelude::*,
    *,
};

use crate::{
    acp::{Bridge, Command, Event, Permission},
    chat::Chat,
    preferences::{Appearance, Preferences},
    session_actions::SessionActions,
    session_list::SessionList,
    session_load::SessionLoad,
    ui::new_chat::NewChatAction,
};

pub struct ChatView {
    cwd: PathBuf,
    choosing_directory: bool,
    session_directory_selected: bool,
    preferences_path: Option<PathBuf>,
    agent: AcpAgentConfig,
    appearance: Appearance,
    input: Entity<TextareaState>,
    chat: Chat,
    bridge: Option<Bridge>,
    receiver: Option<Task<()>>,
    session_id: Option<SessionId>,
    loading_session: Option<SessionLoad>,
    load_supported: bool,
    session_list: SessionList,
    session_scroll: ScrollHandle,
    current_session_anchor: ScrollAnchor,
    session_actions: SessionActions,
    disconnected: bool,
    stopping: bool,
    status: String,
    tokens: Option<u64>,
    permissions: VecDeque<Permission>,
    preference_error: Option<String>,
    scroll: ScrollHandle,
    expanded_updates: HashSet<usize>,
    images: HashMap<usize, Vec<Arc<Image>>>,
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
                let composing = input.update(cx, |input, cx| {
                    input.marked_text_range(window, cx).is_some()
                });
                if !composing {
                    this.send(window, cx);
                }
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
            chat: Chat::default(),
            bridge: None,
            receiver: None,
            session_id: None,
            loading_session: None,
            load_supported: false,
            session_list: SessionList::default(),
            session_scroll,
            current_session_anchor,
            session_actions: SessionActions::default(),
            disconnected: false,
            stopping: false,
            status: "就绪".into(),
            tokens: None,
            permissions: VecDeque::new(),
            preference_error: None,
            scroll: ScrollHandle::new(),
            expanded_updates: HashSet::new(),
            images: HashMap::new(),
            _subscriptions: vec![changes, quit, system_theme],
        }
    }

    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.bridge.is_some() {
            return;
        }
        self.disconnected = false;
        self.load_supported = false;
        self.session_list.connect();
        self.status = "正在连接 Aries…".into();
        match Bridge::start(self.agent.clone(), self.cwd.clone()) {
            Ok(bridge) => {
                let events = bridge.events().clone();
                self.receiver = Some(cx.spawn_in(window, async move |view, cx| {
                    while let Ok(event) = events.recv().await {
                        if view
                            .update_in(cx, |this, _, cx| this.apply_event(event, cx))
                            .is_err()
                        {
                            return;
                        }
                    }
                    let _ = view.update_in(cx, |this, _, cx| {
                        if !this.disconnected {
                            this.apply_event(
                                Event::Disconnected("Agent 连接已结束，请新建对话".into()),
                                cx,
                            );
                        }
                    });
                }));
                self.bridge = Some(bridge);
            }
            Err(error) => {
                self.apply_event(Event::Disconnected(format!("无法启动 Agent：{error}")), cx);
            }
        }
        cx.notify();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disconnected
            || self.chat.is_busy()
            || self.session_actions.is_busy()
            || self.loading_session.is_some()
            || self.choosing_directory
            || self.input.read(cx).value().trim().is_empty()
        {
            return;
        }
        if self.session_id.is_none() && !self.session_directory_selected {
            self.choose_session_directory(NewChatAction::SendPrompt, window, cx);
            return;
        }
        let Some(prompt) = self.chat.begin_turn(&self.input.read(cx).value()) else {
            return;
        };
        if self.bridge.is_none() {
            self.connect(window, cx);
        }
        if !self.session_list.is_connecting() && !self.disconnected {
            self.status = "思考中…".into();
        }
        if self
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(Command::Prompt(prompt)).is_ok())
        {
            self.input
                .update(cx, |input, cx| input.set_value("", window, cx));
        } else {
            self.apply_event(Event::Disconnected("无法发送消息，请新建对话".into()), cx);
        }
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        if !self.chat.is_busy() || self.stopping {
            return;
        }
        self.dismiss_permissions();
        if self
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(Command::Cancel).is_ok())
        {
            self.stopping = true;
            self.status = "正在停止…".into();
        } else {
            self.apply_event(Event::Disconnected("连接已关闭，请新建对话".into()), cx);
        }
        cx.notify();
    }

    fn new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.choose_session_directory(NewChatAction::Create, window, cx);
    }

    fn reset_chat(&mut self) {
        self.chat.reset();
        self.expanded_updates.clear();
        self.images.clear();
        self.session_id = None;
        self.session_directory_selected = false;
        self.loading_session = None;
        self.stopping = false;
        self.status = "就绪".into();
        self.tokens = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
    }

    fn apply_event(&mut self, event: Event, cx: &mut Context<Self>) {
        let follow = self.scroll.max_offset().y + self.scroll.offset().y < px(48.);
        match event {
            Event::Connected {
                list_sessions,
                load_session,
            } => {
                self.session_list.connected(list_sessions);
                self.load_supported = load_session;
                if !self.stopping {
                    self.status = if self.chat.is_busy() {
                        "思考中…"
                    } else {
                        "就绪"
                    }
                    .into();
                }
                self.refresh_sessions(cx);
            }
            Event::Sessions(result) => {
                self.session_list.receive(result);
                if self.session_list.take_refresh_request() {
                    self.refresh_sessions(cx);
                }
            }
            Event::SessionsDeleted(result) => self.apply_session_deletion(result, cx),
            Event::SessionLoaded { session_id, result } => {
                self.finish_session_load(session_id, result, cx);
            }
            Event::Ready(id) => {
                self.session_id = Some(id);
                self.chat.end_segment();
                self.status = "思考中…".into();
            }
            Event::Update(notification) => {
                if let SessionUpdate::SessionInfoUpdate(update) = &notification.update {
                    self.session_list.update(&notification.session_id, update);
                }
                if let Some(loading) = &mut self.loading_session
                    && loading.session_id() == &notification.session_id
                {
                    loading.receive(*notification);
                    cx.notify();
                    return;
                }
                if self.session_id.as_ref() != Some(&notification.session_id)
                    && !(self.session_id.is_none() && self.chat.is_busy())
                {
                    cx.notify();
                    return;
                }
                if let SessionUpdate::UsageUpdate(usage) = &notification.update {
                    self.tokens = Some(usage.used);
                }
                if self.chat.is_busy() && !self.stopping && self.permissions.is_empty() {
                    match &notification.update {
                        SessionUpdate::AgentThoughtChunk(_) => self.status = "思考中…".into(),
                        SessionUpdate::AgentMessageChunk(_) => self.status = "正在回复…".into(),
                        SessionUpdate::ToolCall(tool) => self.status = tool.title.clone(),
                        _ => {}
                    }
                }
                let index = self.chat.receive(*notification);
                self.cache_message_images(index);
            }
            Event::Permission(permission) => {
                if self.stopping {
                    permission.respond(None);
                } else {
                    self.permissions.push_back(permission);
                    self.status = "等待你的选择".into();
                }
            }
            Event::Finished(result) => {
                self.dismiss_permissions();
                self.stopping = false;
                self.status = match &result {
                    Ok(StopReason::Cancelled) => "已停止",
                    Ok(_) => "就绪",
                    Err(_) => "回复失败",
                }
                .into();
                self.chat
                    .finish(result.err().map(|error| error.to_string()));
                self.refresh_sessions(cx);
            }
            Event::Disconnected(error) => {
                self.dismiss_permissions();
                self.stopping = false;
                self.disconnected = true;
                self.load_supported = false;
                self.loading_session = None;
                self.session_list.disconnect(error.clone());
                self.session_actions.disconnect();
                self.status = "连接已断开".into();
                self.chat
                    .finish(Some(format!("{error}\n请新建对话重新连接 Aries。")));
            }
        }
        if follow {
            self.scroll.scroll_to_bottom();
        }
        cx.notify();
    }

    fn cache_message_images(&mut self, index: usize) {
        let message = &self.chat.messages()[index];
        let images = if let Some(compaction) = message.compaction() {
            media::images(compaction).unwrap_or_default()
        } else if let Some(notification) = message.updates().last() {
            media::images(&notification.update).unwrap_or_default()
        } else {
            Vec::new()
        };
        if images.is_empty() {
            self.images.remove(&index);
        } else {
            self.images.insert(index, images);
        }
    }

    fn choose_permission(&mut self, option: PermissionOptionId, cx: &mut Context<Self>) {
        if let Some(permission) = self.permissions.pop_front() {
            permission.respond(Some(option));
        }
        self.status = if self.permissions.is_empty() {
            "正在执行…"
        } else {
            "等待你的选择"
        }
        .into();
        cx.notify();
    }

    fn dismiss_permissions(&mut self) {
        for permission in self.permissions.drain(..) {
            permission.respond(None);
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
