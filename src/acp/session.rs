use std::{path::PathBuf, time::Duration};

use agent_client_protocol::{
    Agent, ConnectionTo, Error,
    schema::{
        ProtocolVersion,
        v1::{
            CancelNotification, ClientCapabilities, ClientSessionCapabilities,
            CompactionCapabilities, ContentBlock, Implementation, InitializeRequest,
            NewSessionRequest, NoticeCapabilities, PromptRequest,
        },
    },
};
use async_channel::{Receiver, Sender};
use tokio::sync::watch;

use crate::acp::{Command, Event};

pub async fn run(
    connection: ConnectionTo<Agent>,
    cwd: PathBuf,
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
        let session = tokio::time::timeout(
            Duration::from_secs(30),
            connection
                .send_request(NewSessionRequest::new(cwd))
                .block_task(),
        )
        .await
        .map_err(|_| timeout("创建会话超时"))??;
        Ok(session.session_id)
    };
    tokio::pin!(setup);
    let mut pending = None;
    let session_id = loop {
        tokio::select! {
            biased;
            command = commands.recv() => match command {
                Ok(Command::Cancel) => return Err(Error::internal_error().data("已停止连接 Aries")),
                Ok(command @ Command::Prompt(_)) => pending = Some(command),
                Err(_) => return Ok(()),
            },
            _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            result = &mut setup => break result?,
        }
    };
    let _ = events.send(Event::Ready(session_id.to_string())).await;

    loop {
        let command = match pending.take() {
            Some(command) => command,
            None => tokio::select! {
                command = commands.recv() => match command { Ok(command) => command, Err(_) => break },
                _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            },
        };
        let Command::Prompt(text) = command else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        cancel_tx.send_replace(false);
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
                    Ok(Command::Prompt(_)) => {}
                    Err(_) => return Ok(()),
                },
                _ = &mut deadline, if cancelling => return Err(timeout("Agent 未响应停止请求，连接已关闭")),
                _ = connection.incoming_closed() => return Err(Error::internal_error().data("Agent 已关闭连接")),
            }
        };
        let _ = events
            .send(Event::Finished(
                result
                    .map(|result| result.stop_reason)
                    .map_err(|error| error.to_string()),
            ))
            .await;
    }
    Ok(())
}

fn timeout(message: &str) -> Error {
    Error::internal_error().data(message)
}
