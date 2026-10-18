use agent_client_protocol::schema::v1::{TerminalOutputRequest, TerminalOutputResponse};
use agent_client_protocol::{Error, Responder};

use crate::acp::terminal::SharedTerminals;

pub async fn handle(
    request: TerminalOutputRequest,
    responder: Responder<TerminalOutputResponse>,
    terminals: SharedTerminals,
) -> Result<(), Error> {
    responder.respond_with_result(terminals.output(&request.session_id, &request.terminal_id))
}
