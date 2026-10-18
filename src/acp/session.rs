use std::{collections::VecDeque, path::PathBuf, time::Duration};

use agent_client_protocol::{
    Agent, ConnectionTo, Error,
    schema::{
        ProtocolVersion,
        v1::{
            CancelNotification, ClientCapabilities, ClientSessionCapabilities,
            CompactionCapabilities, ContentBlock, Implementation, InitializeRequest,
            NewSessionRequest, NoticeCapabilities, PromptRequest, SessionId,
        },
    },
};
use async_channel::{Receiver, Sender};
use tokio::sync::watch;

use crate::acp::{Command, DeletionReport, Event, delete_sessions, list_sessions, load_session};

pub async fn run(
    connection: ConnectionTo<Agent>,
    mut cwd: PathBuf,
    commands: Receiver<Command>,
    events: Sender<Event>,
    cancel_tx: watch::Sender<bool>,
) -> Result<(), Error> {
    let setup = async {
        let init = tokio::time::timeout(
            Duration::from_secs(30),
            connection
                .send_request(
                    InitializeRequest::new(ProtocolVersion::V1)
                        .client_capabilities(
                            ClientCapabilities::new().session(
                                ClientSessionCapabilities::new()
                                    .notices(NoticeCapabilities::new())
                                    .compaction(CompactionCapabilities::new()),
                            ),
                        )
                        .client_info(Implementation::new(
                            "aries-orbit",
                            env!("CARGO_PKG_VERSION"),
                        )),
                )
                .block_task(),
        )
        .await
        .map_err(|_| timeout("初始化 Agent 超时"))??;
        if init.protocol_version != ProtocolVersion::V1 {
            return Err(Error::internal_error().data("Agent 未协商 ACP v1"));
        }
        Ok(init.agent_capabilities)
    };
    tokio::pin!(setup);
    let mut pending = VecDeque::new();
    let capabilities = loop {
        tokio::select! {
            biased;
            command = commands.recv() => match command {
                Ok(Command::Cancel) => return Err(Error::internal_error().data("已停止连接 Aries")),
                Ok(command) => pending.push_back(command),
                Err(_) => return Ok(()),
            },
            _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            result = &mut setup => break result?,
        }
    };
    let supports_list = capabilities.session_capabilities.list.is_some();
    let _ = events
        .send(Event::Connected {
            list_sessions: supports_list,
            load_session: capabilities.load_session,
        })
        .await;
    let mut session_id = None;

    loop {
        let command = match pending.pop_front() {
            Some(command) => command,
            None => tokio::select! {
                command = commands.recv() => match command { Ok(command) => command, Err(_) => break },
                _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            },
        };
        if matches!(command, Command::ListSessions) {
            request_sessions(&connection, &events, supports_list).await;
            continue;
        }
        if let Command::DeleteSessions(ids) = command {
            let result = delete_sessions::request(&connection, ids).await;
            if session_id
                .as_ref()
                .is_some_and(|id| result.deleted().contains(id))
            {
                session_id = None;
            }
            let _ = events.send(Event::SessionsDeleted(result)).await;
            continue;
        }
        if let Command::LoadSession(request) = command {
            let id = request.session_id.clone();
            let directory = request.cwd.clone();
            cancel_tx.send_replace(false);
            let result = if capabilities.load_session {
                load_session::request(&connection, *request).await
            } else {
                Err(Error::internal_error().data("Agent 不支持加载会话"))
            };
            if result.is_ok() {
                session_id = Some(id.clone());
                cwd = directory;
            }
            let _ = events
                .send(Event::SessionLoaded {
                    session_id: id,
                    result,
                })
                .await;
            continue;
        }
        let Command::Prompt(text) = command else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        cancel_tx.send_replace(false);
        if session_id.is_none() {
            let create = tokio::time::timeout(
                Duration::from_secs(30),
                connection
                    .send_request(NewSessionRequest::new(cwd.clone()))
                    .block_task(),
            );
            tokio::pin!(create);
            let session = loop {
                tokio::select! {
                    biased;
                    command = commands.recv() => match command {
                        Ok(Command::Cancel) => return Err(Error::internal_error().data("已停止创建会话")),
                        Ok(Command::ListSessions) => request_sessions(&connection, &events, supports_list).await,
                        Ok(Command::DeleteSessions(ids)) => reject_deletion(&events, ids).await,
                        Ok(Command::LoadSession(request)) => reject_load(&events, request.session_id).await,
                        Ok(Command::Prompt(_)) => {},
                        Err(_) => return Ok(()),
                    },
                    _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
                    result = &mut create => break result.map_err(|_| timeout("创建会话超时"))??,
                }
            };
            let _ = events.send(Event::Ready(session.session_id.clone())).await;
            session_id = Some(session.session_id);
        }
        let session_id = session_id
            .as_ref()
            .expect("session is created before prompting");
        let response = connection
            .send_request(PromptRequest::new(
                session_id.clone(),
                vec![ContentBlock::from(text)],
            ))
            .block_task();
        tokio::pin!(response);
        let deadline = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(deadline);
        let mut cancelling = false;
        let result = loop {
            tokio::select! {
                result = &mut response => break result,
                command = commands.recv() => match command {
                    Ok(Command::Cancel) if !cancelling => {
                        cancelling = true;
                        cancel_tx.send_replace(true);
                        connection.send_notification(CancelNotification::new(session_id.clone()))?;
                        deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(10));
                    }
                    Ok(Command::Cancel) => {}
                    Ok(Command::ListSessions) => request_sessions(&connection, &events, supports_list).await,
                    Ok(Command::DeleteSessions(ids)) => reject_deletion(&events, ids).await,
                    Ok(Command::LoadSession(request)) => reject_load(&events, request.session_id).await,
                    Ok(Command::Prompt(_)) => {}
                    Err(_) => return Ok(()),
                },
                _ = &mut deadline, if cancelling => return Err(timeout("Agent 未响应停止请求，连接已关闭")),
                _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            }
        };
        let _ = events
            .send(Event::Finished(result.map(|result| result.stop_reason)))
            .await;
    }
    Ok(())
}

async fn reject_deletion(events: &Sender<Event>, ids: Vec<SessionId>) {
    let _ = events
        .send(Event::SessionsDeleted(DeletionReport::rejected(
            ids,
            "请等待当前回复结束后再删除会话",
        )))
        .await;
}

async fn reject_load(events: &Sender<Event>, session_id: SessionId) {
    let _ = events
        .send(Event::SessionLoaded {
            session_id,
            result: Err(Error::internal_error().data("请等待当前回复结束后再加载会话")),
        })
        .await;
}

async fn request_sessions(
    connection: &ConnectionTo<Agent>,
    events: &Sender<Event>,
    supported: bool,
) {
    let result = if supported {
        list_sessions::request(connection, events)
    } else {
        Err(Error::internal_error().data("Agent 不支持会话列表"))
    };
    if let Err(error) = result {
        let _ = events.send(Event::Sessions(Err(error))).await;
    }
}

fn timeout(message: &str) -> Error {
    Error::internal_error().data(message)
}
