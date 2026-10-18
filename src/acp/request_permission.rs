use agent_client_protocol::{
    Agent, ConnectionTo, Error, Responder,
    schema::v1::{
        RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
        SelectedPermissionOutcome,
    },
};
use async_channel::Sender;
use tokio::sync::{oneshot, watch};

use super::{Event, Permission};

pub async fn handle(
    request: RequestPermissionRequest,
    responder: Responder<RequestPermissionResponse>,
    connection: ConnectionTo<Agent>,
    events: &Sender<Event>,
    mut cancelled: watch::Receiver<bool>,
) -> Result<(), Error> {
    let (response, answer) = oneshot::channel();
    let options = request
        .options
        .iter()
        .map(|option| (option.option_id.to_string(), option.name.clone()))
        .collect();
    let permission = Permission {
        title: request
            .tool_call
            .fields
            .title
            .unwrap_or_else(|| "Agent 请求授权".into()),
        options,
        response,
    };
    let already_cancelled = *cancelled.borrow_and_update();
    if already_cancelled || events.send(Event::Permission(permission)).await.is_err() {
        return responder.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ));
    }
    connection.spawn(async move {
        let choice = tokio::select! {
            answer = answer => answer.ok().flatten(),
            _ = cancelled.changed() => None,
        };
        let option = choice.and_then(|id| {
            request
                .options
                .into_iter()
                .find(|option| option.option_id.to_string() == id)
        });
        let outcome = option
            .map(|option| {
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option.option_id))
            })
            .unwrap_or(RequestPermissionOutcome::Cancelled);
        responder.respond(RequestPermissionResponse::new(outcome))
    })
}
