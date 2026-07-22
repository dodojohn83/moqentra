//! Moqentra `moqentra-worker-control` crate.
//!
//! Implements the gRPC control surface and local OCI executor for worker agents.

use moqentra_contracts::moqentra::worker::v1::{
    worker_agent_service_client::WorkerAgentServiceClient,
    worker_agent_service_server::{WorkerAgentService, WorkerAgentServiceServer},
    WorkerAgentServiceOpenStreamRequest, WorkerAgentServiceOpenStreamResponse,
};
use std::pin::Pin;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

pub mod local_executor;

/// gRPC service handling the worker/agent bidirectional stream.
#[derive(Debug, Default)]
pub struct WorkerControlService;

#[tonic::async_trait]
impl WorkerAgentService for WorkerControlService {
    type OpenStreamStream =
        Pin<Box<dyn Stream<Item = Result<WorkerAgentServiceOpenStreamResponse, Status>> + Send>>;

    async fn open_stream(
        &self,
        request: Request<Streaming<WorkerAgentServiceOpenStreamRequest>>,
    ) -> Result<Response<Self::OpenStreamStream>, Status> {
        let mut inbound = request.into_inner();

        let outbound = async_stream::try_stream! {
            while let Some(msg) = inbound.message().await? {
                let response = match msg.payload {
                    Some(_) => WorkerAgentServiceOpenStreamResponse {
                        payload: Some(moqentra_contracts::moqentra::worker::v1::worker_agent_service_open_stream_response::Payload::Command(
                            moqentra_contracts::moqentra::worker::v1::Command::default(),
                        )),
                    },
                    None => continue,
                };
                yield response;
            }
        };

        Ok(Response::new(Box::pin(outbound) as Self::OpenStreamStream))
    }
}

/// Build a [`WorkerAgentServiceServer`] for the control plane.
pub fn worker_service_server() -> WorkerAgentServiceServer<WorkerControlService> {
    WorkerAgentServiceServer::new(WorkerControlService)
}

/// Connect a worker agent client to `dst`.
pub async fn connect_client<D>(
    dst: D,
) -> Result<WorkerAgentServiceClient<tonic::transport::Channel>, tonic::transport::Error>
where
    D: TryInto<tonic::transport::Endpoint>,
    D::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    WorkerAgentServiceClient::connect(dst).await
}
