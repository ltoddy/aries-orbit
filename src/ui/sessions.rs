use std::{collections::BTreeMap, path::Path};

use agent_client_protocol::schema::v1::SessionInfo;
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable, Icon, Sizable,
        button::{Button, ButtonVariants},
        menu::{DropdownMenu, PopupMenuItem},
        tooltip::Tooltip,
    },
    prelude::*,
    *,
};

use crate::{
    acp::Command,
    ui::{ChatView, palette::Palette},
};

impl ChatView {
    fn locate_current_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self
            .session_id
            .as_ref()
            .and_then(|id| self.session_list.session(id))
            .map(|session| session.cwd.clone())
            .unwrap_or_else(|| self.cwd.clone());
        self.session_list.expand_project(&cwd);
        let anchor = self.current_session_anchor.clone();
        window.on_next_frame(move |window, cx| {
            anchor.scroll_to(window, cx);
            window.on_next_frame(|window, _| window.refresh());
        });
        cx.notify();
    }

    pub fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        if self.disconnected || !self.session_list.is_supported() {
            return;
        }
        if self.session_list.is_loading()
            || self.session_actions.is_busy()
            || self.loading_session.is_some()
        {
            self.session_list.defer_refresh();
            return;
        }
        if !self.session_list.begin_refresh() {
            return;
        }
        if !self
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(Command::ListSessions).is_ok())
        {
            self.session_list
                .fail_refresh("无法获取会话列表，请新建对话重新连接 Aries。".into());
        }
        cx.notify();
    }

    pub fn render_sessions(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let list = &self.session_list;
        let current_listed = self
            .session_id
            .as_ref()
            .is_some_and(|id| list.session(id).is_some());
        let mut groups: BTreeMap<&Path, Vec<&SessionInfo>> = BTreeMap::new();
        for session in list.entries() {
            groups.entry(&session.cwd).or_default().push(session);
        }
        if !current_listed {
            groups.entry(&self.cwd).or_default();
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .px_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_shrink_0()
                    .pl_2()
                    .mb_2()
                    .text_size(px(12.))
                    .text_color(p.muted())
                    .child("对话")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("expand-all-sessions")
                                    .icon(Icon::new(IconName::ListChevronsUpDown).size(px(14.)))
                                    .tooltip("展开全部")
                                    .accessibility_label("展开全部项目")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.session_list.expand_all();
                                        this.session_list.expand_project(&this.cwd);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("collapse-all-sessions")
                                    .icon(Icon::new(IconName::ListChevronsDownUp).size(px(14.)))
                                    .tooltip("收起全部")
                                    .accessibility_label("收起全部项目")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.session_list.collapse_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("locate-current-session")
                                    .icon(Icon::new(IconName::LocateFixed).size(px(14.)))
                                    .tooltip("定位当前会话")
                                    .accessibility_label("定位当前会话")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.locate_current_session(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("refresh-sessions")
                                    .label("刷新")
                                    .tooltip("重新获取会话列表")
                                    .ghost()
                                    .small()
                                    .disabled(
                                        list.is_loading()
                                            || !list.is_supported()
                                            || self.disconnected
                                            || self.session_actions.is_busy()
                                            || self.loading_session.is_some(),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.refresh_sessions(cx)),
                                    ),
                            ),
                    ),
            )
            .child(self.render_session_actions(p, cx))
            .child(
                div()
                    .id("session-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.session_scroll)
                    .pb_3()
                    .children(groups.into_iter().map(|(cwd, sessions)| {
                        let expanded = list.is_project_expanded(cwd);
                        let include_current = !current_listed && cwd == self.cwd;
                        let count = sessions.len() + usize::from(include_current);
                        let directory =
                            cwd.file_name().unwrap_or(cwd.as_os_str()).to_string_lossy();
                        let path = cwd.to_path_buf();
                        let delete_path = path.clone();
                        let has_sessions = !self.session_ids(Some(cwd)).is_empty();
                        let view = cx.weak_entity();
                        div()
                            .mb_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "cwd:{}",
                                            cwd.display()
                                        )))
                                        .ghost()
                                        .small()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .w_full()
                                                .min_w_0()
                                                .flex()
                                                .items_center()
                                                .justify_start()
                                                .gap_1()
                                                .text_left()
                                                .child(
                                                    Icon::new(if expanded {
                                                        IconName::FolderOpen
                                                    } else {
                                                        IconName::Folder
                                                    })
                                                    .size(px(14.))
                                                    .flex_shrink_0(),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .truncate()
                                                        .child(format!("{directory} ({count})")),
                                                ),
                                        )
                                        .tooltip(cwd.to_string_lossy().into_owned())
                                        .accessibility_label(format!(
                                            "{}目录：{}",
                                            if expanded { "收起" } else { "展开" },
                                            cwd.display()
                                        ))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.session_list.toggle_project(&path);
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "project-menu:{}",
                                            cwd.display()
                                        )))
                                        .icon(Icon::new(IconName::Ellipsis).size(px(13.)))
                                        .ghost()
                                        .small()
                                        .flex_shrink_0()
                                        .tooltip("项目操作")
                                        .accessibility_label(format!(
                                            "项目 {} 的更多操作",
                                            cwd.display()
                                        ))
                                        .disabled(!self.can_manage_sessions() || !has_sessions)
                                        .dropdown_menu_with_anchor(
                                            Anchor::TopRight,
                                            move |menu, _, _| {
                                                let view = view.clone();
                                                let delete_path = delete_path.clone();
                                                menu.item(
                                                    PopupMenuItem::new("删除项目会话")
                                                        .icon(IconName::Trash)
                                                        .on_click(move |_, _, cx| {
                                                            let _ = view.update(cx, |this, cx| {
                                                                let ids = this.session_ids(Some(
                                                                    &delete_path,
                                                                ));
                                                                this.prepare_session_deletion(
                                                                    ids,
                                                                    format!(
                                                                        "项目 {} 下的会话",
                                                                        delete_path.display()
                                                                    ),
                                                                    cx,
                                                                );
                                                            });
                                                        }),
                                                )
                                            },
                                        ),
                                    ),
                            )
                            .when(expanded, |group| {
                                group.child(
                                    div()
                                        .mt_1()
                                        .ml_2()
                                        .pl_2()
                                        .border_l_1()
                                        .border_color(p.border())
                                        .when(include_current, |rows| {
                                            rows.child(self.render_current_session(p, cx))
                                        })
                                        .children(sessions.into_iter().map(|session| {
                                            self.render_session_row(session, p, cx)
                                        })),
                                )
                            })
                    }))
                    .when(list.is_loading(), |list| {
                        list.child(
                            div()
                                .p_3()
                                .text_size(px(12.))
                                .text_color(p.muted())
                                .child("正在加载会话…"),
                        )
                    })
                    .when(list.is_unsupported(), |list| {
                        list.child(
                            div()
                                .p_3()
                                .text_size(px(12.))
                                .text_color(p.muted())
                                .child("Agent 不支持会话列表"),
                        )
                    })
                    .when(
                        list.is_supported()
                            && !list.is_loading()
                            && list.error().is_none()
                            && list.entries().is_empty(),
                        |list| {
                            list.child(
                                div()
                                    .p_3()
                                    .text_size(px(12.))
                                    .text_color(p.muted())
                                    .child("暂无历史会话"),
                            )
                        },
                    )
                    .when_some(list.error().map(str::to_owned), |list, error| {
                        list.child(
                            div()
                                .p_3()
                                .text_size(px(12.))
                                .text_color(p.danger())
                                .child(error),
                        )
                    }),
            )
    }

    fn render_current_session(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        if let Some(id) = &self.session_id {
            return self
                .render_session_row(
                    &SessionInfo::new(id.clone(), self.cwd.clone()).title(self.chat.title()),
                    p,
                    cx,
                )
                .into_any_element();
        }
        div()
            .id("current-chat")
            .anchor_scroll(Some(self.current_session_anchor.clone()))
            .px_3()
            .py_2()
            .mb_1()
            .rounded(px(12.))
            .bg(p.hover())
            .child(
                div()
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.chat.title()),
            )
            .child(
                div()
                    .mt_1()
                    .text_size(px(12.))
                    .text_color(p.muted())
                    .child("发送消息开始对话"),
            )
            .into_any_element()
    }

    fn render_session_row(
        &self,
        session: &SessionInfo,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = session.session_id.to_string();
        let current = self.session_id.as_ref() == Some(&session.session_id);
        let loading = self
            .loading_session
            .as_ref()
            .is_some_and(|loading| loading.session_id() == &session.session_id);
        let title = session
            .title
            .as_ref()
            .filter(|title| !title.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                if current {
                    self.chat.title()
                } else {
                    id.clone()
                }
            });
        let details = format!(
            "{}\n{}\n{}{}",
            title,
            session.cwd.display(),
            id,
            session
                .updated_at
                .as_ref()
                .map(|time| format!("\n更新时间：{time}"))
                .unwrap_or_default()
        );
        let delete_id = session.session_id.clone();
        let delete_title = title.clone();
        let load_session = session.clone();
        let view = cx.weak_entity();
        div()
            .id(SharedString::from(format!("session:{id}")))
            .when(current, |row| {
                row.anchor_scroll(Some(self.current_session_anchor.clone()))
            })
            .px_3()
            .py_2()
            .mb_1()
            .rounded(px(12.))
            .when(current || loading, |row| row.bg(p.hover()))
            .tooltip(move |window, cx| Tooltip::new(details.clone()).build(window, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new(SharedString::from(format!("load-session:{id}")))
                            .ghost()
                            .small()
                            .h_auto()
                            .px_0()
                            .flex_1()
                            .min_w_0()
                            .disabled(!self.can_load_sessions())
                            .accessibility_label(format!("打开会话 {title}"))
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_left()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.load_session(load_session.clone(), cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("session-menu:{id}")))
                            .icon(Icon::new(IconName::Ellipsis).size(px(13.)))
                            .ghost()
                            .small()
                            .flex_shrink_0()
                            .tooltip("会话操作")
                            .accessibility_label(format!("会话 {delete_title} 的更多操作"))
                            .disabled(!self.can_manage_sessions())
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                                let view = view.clone();
                                let delete_id = delete_id.clone();
                                let delete_title = delete_title.clone();
                                menu.item(
                                    PopupMenuItem::new("删除会话")
                                        .icon(IconName::Trash)
                                        .on_click(move |_, _, cx| {
                                            let _ = view.update(cx, |this, cx| {
                                                this.prepare_session_deletion(
                                                    vec![delete_id.clone()],
                                                    format!("会话“{delete_title}”"),
                                                    cx,
                                                );
                                            });
                                        }),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}
