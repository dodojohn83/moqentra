//! Moqentra `moqentra-worker-control` crate.
//!
//! Implements the gRPC control surface and local OCI executor for worker agents.

pub mod local_executor;
pub mod session;

pub use moqentra_contracts::moqentra::worker::v1::{
    worker_agent_service_client::WorkerAgentServiceClient,
    worker_agent_service_server::WorkerAgentServiceServer, WorkerAgentServiceOpenStreamRequest,
    WorkerAgentServiceOpenStreamResponse,
};
pub use session::{AgentSession, SessionManager, WorkerControlService};

/// Build a [`WorkerAgentServiceServer`] for the control plane.
pub fn worker_service_server(
    service: WorkerControlService,
) -> WorkerAgentServiceServer<WorkerControlService> {
    WorkerAgentServiceServer::new(service)
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
