use std::time::Duration;

use agent_client_protocol::schema::v1::LoadSessionRequest;
use agent_client_protocol::{Agent, ConnectionTo, Error};

pub async fn request(
    connection: &ConnectionTo<Agent>,
    request: LoadSessionRequest,
) -> Result<(), Error> {
    if !request.cwd.is_absolute() {
        return Err(Error::invalid_params().data("会话工作目录必须是绝对路径"));
    }
    tokio::time::timeout(Duration::from_secs(30), connection.send_request(request).block_task())
        .await
        .map_err(|_| Error::internal_error().data("加载会话超时"))?
        .map(|_| ())
}
