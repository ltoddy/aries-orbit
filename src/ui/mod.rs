mod layout;
mod media;
mod palette;
mod timeline;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};

use agent_client_protocol::{
    AcpAgentConfig,
    schema::v1::{SessionUpdate, StopReason},
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
};

pub struct ChatView {
    cwd: PathBuf,
    preferences_path: Option<PathBuf>,
    agent: AcpAgentConfig,
    appearance: Appearance,
    input: Entity<TextareaState>,
    chat: Chat,
    bridge: Option<Bridge>,
    receiver: Option<Task<()>>,
    session_id: Option<String>,
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
            .map(Preferences::load)
            .unwrap_or_default()
            .appearance;
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
        Self {
            cwd,
            preferences_path,
            agent,
            appearance,
            input,
            chat: Chat::default(),
            bridge: None,
            receiver: None,
            session_id: None,
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

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disconnected {
            return;
        }
        let Some(prompt) = self.chat.begin_turn(&self.input.read(cx).value()) else {
            return;
        };
        self.status = if self.bridge.is_none() {
            "正在连接 Aries…"
        } else {
            "思考中…"
        }
        .into();
        if self.bridge.is_none() {
            match Bridge::start(self.agent.clone(), self.cwd.clone()) {
                Ok(bridge) => {
                    let events = bridge.events();
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
                    return;
                }
            }
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
        if !self.chat.busy || self.stopping {
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
        if self.chat.busy {
            return;
        }
        self.dismiss_permissions();
        self.receiver = None;
        self.bridge = None;
        self.chat.reset();
        self.expanded_updates.clear();
        self.images.clear();
        self.session_id = None;
        self.disconnected = false;
        self.stopping = false;
        self.status = "就绪".into();
        self.tokens = None;
        self.input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    fn apply_event(&mut self, event: Event, cx: &mut Context<Self>) {
        let follow = self.scroll.max_offset().y + self.scroll.offset().y < px(48.);
        match event {
            Event::Ready(id) => {
                self.session_id = Some(id);
                self.chat.end_segment();
                self.status = "思考中…".into();
            }
            Event::Update(notification) => {
                if let SessionUpdate::UsageUpdate(usage) = &notification.update {
                    self.tokens = Some(usage.used);
                }
                if self.chat.busy && !self.stopping && self.permissions.is_empty() {
                    match &notification.update {
                        SessionUpdate::AgentThoughtChunk(_) => self.status = "思考中…".into(),
                        SessionUpdate::AgentMessageChunk(_) => self.status = "正在回复…".into(),
                        SessionUpdate::ToolCall(tool) => self.status = tool.title.clone(),
                        _ => {}
                    }
                }
                let index = self.chat.receive(*notification);
                let message = &self.chat.messages[index];
                let images = if let Some(compaction) = &message.compaction {
                    media::images(compaction)
                } else {
                    media::images(
                        &message
                            .updates
                            .last()
                            .expect("received notification is retained")
                            .update,
                    )
                };
                if images.is_empty() {
                    self.images.remove(&index);
                } else {
                    self.images.insert(index, images);
                }
            }
            Event::Permission(permission) => {
                if self.stopping {
                    let _ = permission.response.send(None);
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
                self.chat.finish(result.err());
            }
            Event::Disconnected(error) => {
                self.dismiss_permissions();
                self.stopping = false;
                self.disconnected = true;
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

    fn choose_permission(&mut self, option: String, cx: &mut Context<Self>) {
        if let Some(permission) = self.permissions.pop_front() {
            let _ = permission.response.send(Some(option));
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
            let _ = permission.response.send(None);
        }
    }

    fn set_appearance(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        palette::apply(appearance, cx);
        self.preference_error = match &self.preferences_path {
            Some(path) => Preferences { appearance }
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
