use std::collections::HashSet;
use std::path::Path;

use agent_client_protocol::schema::v1::SessionId;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::acp::{Command, DeletionReport};
use crate::ui::ChatView;
use crate::ui::palette::Palette;

impl ChatView {
    pub fn can_manage_sessions(&self) -> bool {
        !self.disconnected
            && self.is_session_idle()
            && !self.session_list.is_loading()
            && self.session_list.is_supported()
    }

    pub fn session_ids(&self, cwd: Option<&Path>) -> Vec<SessionId> {
        let mut ids: Vec<_> = self
            .session_list
            .entries()
            .iter()
            .filter(|session| cwd.is_none_or(|cwd| cwd == session.cwd))
            .map(|session| session.session_id.clone())
            .collect();
        if cwd.is_none_or(|cwd| cwd == self.cwd)
            && let Some(id) = &self.session_id
            && !ids.contains(id)
        {
            ids.push(id.clone());
        }
        ids
    }

    pub fn prepare_session_deletion(
        &mut self,
        ids: Vec<SessionId>,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if !self.can_manage_sessions() {
            return;
        }
        let available: HashSet<_> = self.session_ids(None).into_iter().collect();
        let mut seen = HashSet::new();
        let ids: Vec<_> = ids
            .into_iter()
            .filter(|id| available.contains(id) && seen.insert(id.clone()))
            .collect();
        if ids.is_empty() {
            return;
        }
        self.session_deletion.prepare_deletion(ids, label);
        cx.notify();
    }

    fn confirm_session_deletion(&mut self, cx: &mut Context<Self>) {
        if self.disconnected
            || self.chat.is_busy()
            || self.session_deletion.is_deleting()
            || self.session_list.is_loading()
        {
            return;
        }
        let Some(ids) = self.session_deletion.begin_deletion() else {
            return;
        };
        if !self
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(Command::DeleteSessions(ids)).is_ok())
        {
            self.session_deletion.fail_deletion("无法发送删除请求，请重新连接后重试。".into());
        }
        cx.notify();
    }

    pub fn apply_session_deletion(
        &mut self,
        result: DeletionReport,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (deleted, failed) = result.into_parts();
        let deleted: HashSet<_> = deleted.into_iter().collect();
        self.drafts.retain(|id, _| !deleted.contains(id));
        if self.session_id.as_ref().is_some_and(|id| deleted.contains(id)) {
            self.reset_chat();
            self.input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.session_list.remove_sessions(&deleted);
        let notice = if failed.is_empty() {
            format!("已删除 {} 个会话", deleted.len())
        } else {
            format!("已删除 {} 个，{} 个删除失败", deleted.len(), failed.len())
        };
        let error = (!failed.is_empty()).then(|| {
            failed
                .iter()
                .map(|(id, error)| {
                    let title = self
                        .session_list
                        .session(id)
                        .and_then(|session| session.title.as_deref())
                        .filter(|title| !title.trim().is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| id.to_string());
                    format!("{title}：{error}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        });
        self.session_deletion.finish_deletion(notice, error);
        self.session_list.take_refresh_request();
        self.refresh_sessions(cx);
        cx.notify();
    }

    pub fn render_session_actions(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let actions = &self.session_deletion;
        div()
            .flex_shrink_0()
            .when_some(actions.pending_deletion(), |panel, pending| {
                panel.child(
                    div()
                        .p_3()
                        .mb_2()
                        .rounded(px(10.))
                        .border_1()
                        .border_color(p.border())
                        .bg(p.input())
                        .child(div().text_size(px(12.)).child(format!(
                            "删除{}（{} 个会话）？",
                            pending.label(),
                            pending.session_count()
                        )))
                        .child(
                            div()
                                .mt_2()
                                .text_size(px(12.))
                                .text_color(p.muted())
                                .child("会话记录将被删除，项目文件不受影响。"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .mt_2()
                                .child(
                                    Button::new("confirm-session-deletion")
                                        .label("删除")
                                        .danger()
                                        .small()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.confirm_session_deletion(cx)
                                        })),
                                )
                                .child(
                                    Button::new("cancel-session-deletion")
                                        .label("取消")
                                        .small()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.session_deletion.cancel_deletion();
                                            if this.session_list.take_refresh_request() {
                                                this.refresh_sessions(cx);
                                            }
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
            })
            .when(actions.is_deleting(), |panel| {
                panel.child(
                    div().p_2().text_size(px(12.)).text_color(p.muted()).child("正在删除会话…"),
                )
            })
            .when_some(actions.notice().map(str::to_owned), |panel, notice| {
                panel.child(div().p_2().text_size(px(12.)).text_color(p.muted()).child(notice))
            })
            .when_some(actions.error().map(str::to_owned), |panel, error| {
                panel.child(
                    div()
                        .id("session-deletion-error")
                        .max_h(px(120.))
                        .overflow_y_scroll()
                        .p_2()
                        .text_size(px(12.))
                        .text_color(p.danger())
                        .child(error),
                )
            })
    }
}
