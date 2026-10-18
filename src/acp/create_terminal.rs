use agent_client_protocol::schema::v1::{CreateTerminalRequest, CreateTerminalResponse};
use agent_client_protocol::{Error, Responder};

use crate::acp::terminal::SharedTerminals;

pub async fn handle(
    request: CreateTerminalRequest,
    responder: Responder<CreateTerminalResponse>,
    terminals: SharedTerminals,
) -> Result<(), Error> {
    responder.respond_with_result(terminals.create(request).map(CreateTerminalResponse::new))
}
