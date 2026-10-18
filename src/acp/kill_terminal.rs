use agent_client_protocol::schema::v1::{KillTerminalRequest, KillTerminalResponse};
use agent_client_protocol::{Agent, ConnectionTo, Error, Responder};

use crate::acp::terminal::SharedTerminals;

pub async fn handle(
    request: KillTerminalRequest,
    responder: Responder<KillTerminalResponse>,
    connection: ConnectionTo<Agent>,
    terminals: SharedTerminals,
) -> Result<(), Error> {
    connection.spawn(async move {
        let result = terminals.kill(&request.session_id, &request.terminal_id).await;
        responder.respond_with_result(result.map(|()| KillTerminalResponse::new()))
    })
}
