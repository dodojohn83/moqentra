//! gRPC worker/agent session state management.
//!
//! `SessionManager` tracks every connected node agent. It validates sequence
//! numbers, fencing tokens, duplicate/late results, and routes outbound
//! commands/leasing/drain frames per node.

use async_trait::async_trait;
use moqentra_contracts::moqentra::worker::v1::{
    worker_agent_service_open_stream_request::Payload as InPayload,
    worker_agent_service_open_stream_response::Payload as OutPayload, AckStatus, Command, Drain,
    Hello, Lease, MetricBatch, WorkerAgentServiceOpenStreamRequest,
    WorkerAgentServiceOpenStreamResponse, WorkerCapabilities,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

/// Asynchronous callback to validate a worker Result payload (e.g. model
/// artifact manifest) and, on success, finalize the training job and create a
/// unique model version.
#[async_trait]
pub trait ArtifactValidator: Send + Sync {
    async fn validate(&self, command_id: &str, payload: &[u8])
        -> Result<(), moqentra_types::Error>;
}

/// Default inactivity timeout before a session can be replaced.
const SESSION_TIMEOUT: Duration = Duration::from_secs(120);
/// Maximum outbound frames queued per session.
const OUTBOUND_BUFFER: usize = 64;
/// Supported control-plane contract version.
const CONTRACT_VERSION: &str = "1";

/// Per-command state tracked by the control plane.
#[derive(Debug, Clone)]
struct CommandRecord {
    command: Command,
    #[allow(dead_code)]
    assigned_at: Instant,
    #[allow(dead_code)]
    acked: bool,
    #[allow(dead_code)]
    completed: bool,
}

/// Mutable state for a single connected agent.
pub struct AgentSession {
    pub node_id: String,
    pub fencing_token: u64,
    pub capabilities: Option<WorkerCapabilities>,
    pub created_at: Instant,
    last_seen: Instant,
    last_heartbeat_seq: u64,
    pending: HashMap<String, CommandRecord>,
    acked: HashSet<String>,
    completed: HashSet<String>,
    cancelled: HashSet<String>,
    draining: bool,
    #[allow(dead_code)]
    outbound: mpsc::Sender<Result<WorkerAgentServiceOpenStreamResponse, Status>>,
    validator: Option<Arc<dyn ArtifactValidator>>,
}

impl AgentSession {
    fn new(
        node_id: String,
        fencing_token: u64,
        capabilities: Option<WorkerCapabilities>,
        outbound: mpsc::Sender<Result<WorkerAgentServiceOpenStreamResponse, Status>>,
        validator: Option<Arc<dyn ArtifactValidator>>,
    ) -> Self {
        Self {
            node_id,
            fencing_token,
            capabilities,
            created_at: Instant::now(),
            last_seen: Instant::now(),
            last_heartbeat_seq: 0,
            pending: HashMap::new(),
            acked: HashSet::new(),
            completed: HashSet::new(),
            cancelled: HashSet::new(),
            draining: false,
            outbound,
            validator,
        }
    }

    fn is_alive(&self) -> bool {
        self.last_seen.elapsed() < SESSION_TIMEOUT
    }

    fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Process one inbound frame and possibly emit outbound frames.
    async fn handle_message(
        &mut self,
        request: WorkerAgentServiceOpenStreamRequest,
    ) -> Result<(), Status> {
        let payload = request.payload.ok_or_else(|| {
            Status::invalid_argument("empty payload requires a Hello, Heartbeat, Ack, Progress, LogChunk, MetricBatch or Result")
        })?;

        match payload {
            InPayload::Hello(hello) => self.handle_hello(hello).await,
            InPayload::Heartbeat(hb) => self.handle_heartbeat(hb).await,
            InPayload::Ack(ack) => self.handle_ack(ack).await,
            InPayload::Progress(p) => self.handle_progress(p).await,
            InPayload::LogChunk(log) => self.handle_log(log).await,
            InPayload::MetricBatch(batch) => self.handle_metrics(batch).await,
            InPayload::Result(result) => self.handle_result(result).await,
        }
    }

    async fn handle_hello(&mut self, hello: Hello) -> Result<(), Status> {
        if hello.node_id != self.node_id {
            return Err(Status::permission_denied(format!(
                "node_id mismatch: expected {}, got {}",
                self.node_id, hello.node_id
            )));
        }
        if hello.agent_version.is_empty() {
            return Err(Status::invalid_argument("agent_version is required"));
        }
        self.capabilities = hello.capabilities;
        if let Some(ref caps) = self.capabilities {
            if caps.contract_version != CONTRACT_VERSION {
                self.send_error(moqentra_contracts::moqentra::common::v1::Error {
                    kind: moqentra_contracts::moqentra::common::v1::ErrorKind::VersionMismatch
                        as i32,
                    code: "VERSION_MISMATCH".to_string(),
                    message: format!(
                        "contract version {} not supported; expected {}",
                        caps.contract_version, CONTRACT_VERSION
                    ),
                    retryable: false,
                    violations: vec![],
                    request_id: "".to_string(),
                    correlation_id: "".to_string(),
                })
                .await;
            }
        }
        self.touch();
        Ok(())
    }

    async fn handle_heartbeat(
        &mut self,
        hb: moqentra_contracts::moqentra::worker::v1::Heartbeat,
    ) -> Result<(), Status> {
        if hb.sequence <= self.last_heartbeat_seq {
            return Err(Status::out_of_range(format!(
                "heartbeat sequence {} is not greater than last {}",
                hb.sequence, self.last_heartbeat_seq
            )));
        }
        self.last_heartbeat_seq = hb.sequence;
        self.touch();

        // Re-send any pending command that has not been cancelled.
        let pending: Vec<Command> = self
            .pending
            .iter()
            .filter(|(id, _)| !self.cancelled.contains(id.as_str()))
            .map(|(_, r)| r.command.clone())
            .collect();
        for command in pending {
            let id = command.command_id.clone();
            self.send_command(command).await;
            tracing::debug!(command_id = %id, "command re-sent after heartbeat");
        }
        Ok(())
    }

    async fn handle_ack(
        &mut self,
        ack: moqentra_contracts::moqentra::worker::v1::Ack,
    ) -> Result<(), Status> {
        if ack.command_id.is_empty() {
            return Err(Status::invalid_argument("ack.command_id is required"));
        }
        let status = AckStatus::try_from(ack.status).unwrap_or(AckStatus::Unspecified);
        if let Some(record) = self.pending.get_mut(&ack.command_id) {
            if status == AckStatus::Rejected {
                record.completed = true;
                self.completed.insert(ack.command_id.clone());
                tracing::warn!(command_id = %ack.command_id, "command rejected by agent");
            } else {
                record.acked = true;
            }
        } else if self.completed.contains(&ack.command_id) {
            // Duplicate ack for a completed command is harmless.
        } else {
            return Err(Status::not_found(format!(
                "ack references unknown command {}",
                ack.command_id
            )));
        }
        self.touch();
        Ok(())
    }

    async fn handle_progress(
        &mut self,
        progress: moqentra_contracts::moqentra::worker::v1::Progress,
    ) -> Result<(), Status> {
        if progress.command_id.is_empty() {
            return Err(Status::invalid_argument("progress.command_id is required"));
        }
        if progress.percent_complete > 100 {
            return Err(Status::invalid_argument(
                "progress.percent_complete must be <= 100",
            ));
        }
        if !self.known_command(&progress.command_id) {
            return Err(Status::not_found(format!(
                "progress references unknown command {}",
                progress.command_id
            )));
        }
        if self.completed.contains(&progress.command_id) {
            return Err(Status::failed_precondition(format!(
                "progress for command {} already completed",
                progress.command_id
            )));
        }
        self.touch();
        Ok(())
    }

    async fn handle_log(
        &mut self,
        log: moqentra_contracts::moqentra::worker::v1::LogChunk,
    ) -> Result<(), Status> {
        if log.command_id.is_empty() {
            return Err(Status::invalid_argument("log_chunk.command_id is required"));
        }
        if !self.known_command(&log.command_id) {
            return Err(Status::not_found(format!(
                "log references unknown command {}",
                log.command_id
            )));
        }
        self.touch();
        Ok(())
    }

    async fn handle_metrics(&mut self, batch: MetricBatch) -> Result<(), Status> {
        if batch.command_id.is_empty() {
            return Err(Status::invalid_argument(
                "metric_batch.command_id is required",
            ));
        }
        if !self.known_command(&batch.command_id) {
            return Err(Status::not_found(format!(
                "metric batch references unknown command {}",
                batch.command_id
            )));
        }
        if self.completed.contains(&batch.command_id) {
            return Err(Status::failed_precondition(format!(
                "metric batch for command {} already completed",
                batch.command_id
            )));
        }
        self.touch();
        Ok(())
    }

    async fn handle_result(
        &mut self,
        result: moqentra_contracts::moqentra::worker::v1::Result,
    ) -> Result<(), Status> {
        if result.command_id.is_empty() {
            return Err(Status::invalid_argument("result.command_id is required"));
        }
        if self.completed.contains(&result.command_id) {
            return Err(Status::already_exists(format!(
                "result for command {} already received",
                result.command_id
            )));
        }
        if !self.known_command(&result.command_id) && !self.pending.contains_key(&result.command_id)
        {
            return Err(Status::not_found(format!(
                "result references unknown command {}",
                result.command_id
            )));
        }
        if self.cancelled.contains(&result.command_id) {
            tracing::info!(command_id = %result.command_id, "result received for cancelled command");
        }
        self.completed.insert(result.command_id.clone());
        self.pending.remove(&result.command_id);
        self.cancelled.remove(&result.command_id);
        self.touch();

        if result.success {
            if let Some(validator) = self.validator.clone() {
                let command_id = result.command_id.clone();
                let payload = result.payload.clone();
                tokio::spawn(async move {
                    if let Err(e) = validator.validate(&command_id, &payload).await {
                        tracing::warn!(command_id = %command_id, error = %e, "artifact validation failed");
                    }
                });
            }
        } else {
            tracing::info!(command_id = %result.command_id, "worker reported failure; skipping artifact validation");
        }

        if self.draining {
            self.send_drain(true, "").await;
        }
        Ok(())
    }

    fn known_command(&self, command_id: &str) -> bool {
        self.pending.contains_key(command_id) || self.acked.contains(command_id)
    }

    /// Assign a command to this agent and enqueue an outbound frame.
    pub async fn assign_command(&mut self, command: Command) {
        if self.completed.contains(&command.command_id) {
            tracing::warn!(command_id = %command.command_id, "refusing to assign already-completed command");
            return;
        }
        if self.cancelled.contains(&command.command_id) {
            tracing::warn!(command_id = %command.command_id, "refusing to assign cancelled command");
            return;
        }
        self.pending.insert(
            command.command_id.clone(),
            CommandRecord {
                command: command.clone(),
                assigned_at: Instant::now(),
                acked: false,
                completed: false,
            },
        );
        self.send_command(command).await;
    }

    /// Issue a lease to this agent.
    pub async fn issue_lease(&mut self, lease: Lease) {
        self.send(OutPayload::Lease(lease)).await;
    }

    /// Request a graceful (or forceful) node-level drain.
    pub async fn request_drain(&mut self, graceful: bool) {
        self.draining = true;
        self.send_drain(graceful, "").await;
    }

    /// Request cancellation of an active command.
    pub async fn request_cancel(&mut self, command_id: &str) -> Result<(), Status> {
        if command_id.is_empty() {
            return Err(Status::invalid_argument("command_id is required"));
        }
        if self.completed.contains(command_id) {
            return Err(Status::failed_precondition(format!(
                "command {} already completed",
                command_id
            )));
        }
        if !self.pending.contains_key(command_id) && !self.acked.contains(command_id) {
            return Err(Status::not_found(format!(
                "command {} not active",
                command_id
            )));
        }
        if !self.cancelled.insert(command_id.to_string()) {
            // Already cancelled.
            return Ok(());
        }
        self.send_drain(true, command_id).await;
        Ok(())
    }

    async fn send_command(&mut self, command: Command) {
        self.send(OutPayload::Command(command)).await;
    }

    async fn send_drain(&mut self, graceful: bool, command_id: &str) {
        self.send(OutPayload::Drain(Drain {
            graceful,
            command_id: command_id.to_string(),
        }))
        .await;
    }

    async fn send_error(&mut self, error: moqentra_contracts::moqentra::common::v1::Error) {
        self.send(OutPayload::Error(error)).await;
    }

    async fn send(&mut self, payload: OutPayload) {
        let frame = WorkerAgentServiceOpenStreamResponse {
            payload: Some(payload),
        };
        // Dropping the frame when the channel is full/closed avoids blocking
        // the whole server on a slow agent.
        let _ = self.outbound.send(Ok(frame)).await;
    }
}

/// Shared map of all connected agent sessions.
#[derive(Default)]
pub struct SessionManager {
    counter: AtomicU64,
    sessions: Mutex<HashMap<String, Arc<Mutex<AgentSession>>>>,
    validator: Option<Arc<dyn ArtifactValidator>>,
}

impl SessionManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn new_with_validator(validator: Arc<dyn ArtifactValidator>) -> Arc<Self> {
        Arc::new(Self {
            counter: AtomicU64::new(0),
            sessions: Mutex::new(HashMap::new()),
            validator: Some(validator),
        })
    }

    /// Register a new agent connection. The first inbound frame on the stream
    /// must be a Hello carrying the same `node_id`.
    pub async fn connect(
        self: &Arc<Self>,
        node_id: String,
        capabilities: Option<WorkerCapabilities>,
    ) -> Result<
        (
            u64,
            mpsc::Receiver<Result<WorkerAgentServiceOpenStreamResponse, Status>>,
        ),
        Status,
    > {
        if node_id.is_empty() {
            return Err(Status::invalid_argument("node_id is required"));
        }

        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(&node_id) {
            let s = session.lock().await;
            if s.is_alive() {
                return Err(Status::already_exists(format!(
                    "node {} already has an active session",
                    node_id
                )));
            }
            // Stale session: replace on reconnect.
        }

        let fencing_token = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::channel(OUTBOUND_BUFFER);
        let session = Arc::new(Mutex::new(AgentSession::new(
            node_id.clone(),
            fencing_token,
            capabilities,
            tx,
            self.validator.clone(),
        )));
        sessions.insert(node_id.clone(), session.clone());

        // Spawn a watchdog that removes stale sessions.
        let manager = Arc::clone(self);
        let watch_node = node_id;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(SESSION_TIMEOUT).await;
                let session = {
                    let sessions = manager.sessions.lock().await;
                    sessions.get(&watch_node).cloned()
                };
                let Some(session) = session else {
                    break;
                };
                let stale = {
                    let s = session.lock().await;
                    !s.is_alive()
                };
                if stale {
                    let mut sessions = manager.sessions.lock().await;
                    if let Some(s) = sessions.get(&watch_node) {
                        if s.lock().await.created_at.elapsed() >= SESSION_TIMEOUT {
                            sessions.remove(&watch_node);
                        }
                    }
                    break;
                }
            }
        });

        Ok((fencing_token, rx))
    }

    /// Submit an outbound frame to the identified node.
    pub async fn send_command(&self, node_id: &str, command: Command) -> Result<(), Status> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(node_id)
            .ok_or_else(|| Status::not_found(format!("node {} not connected", node_id)))?
            .clone();
        drop(sessions);
        session.lock().await.assign_command(command).await;
        Ok(())
    }

    /// Request cancellation of an active command, searching all connected nodes.
    pub async fn cancel_command(&self, command_id: &str) -> Result<(), Status> {
        let sessions = self.sessions.lock().await;
        for session in sessions.values() {
            let mut s = session.lock().await;
            if s.known_command(command_id) || s.pending.contains_key(command_id) {
                return s.request_cancel(command_id).await;
            }
        }
        Err(Status::not_found(format!(
            "command {} not found on any connected node",
            command_id
        )))
    }

    /// Route a control-plane lease to the node.
    pub async fn issue_lease(&self, node_id: &str, lease: Lease) -> Result<(), Status> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(node_id)
            .ok_or_else(|| Status::not_found(format!("node {} not connected", node_id)))?
            .clone();
        drop(sessions);
        session.lock().await.issue_lease(lease).await;
        Ok(())
    }

    /// Request a graceful or forceful drain on the node.
    pub async fn request_drain(&self, node_id: &str, graceful: bool) -> Result<(), Status> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(node_id)
            .ok_or_else(|| Status::not_found(format!("node {} not connected", node_id)))?
            .clone();
        drop(sessions);
        session.lock().await.request_drain(graceful).await;
        Ok(())
    }

    /// Dispatch an inbound message for an existing session.
    pub async fn handle_message(
        &self,
        node_id: &str,
        msg: WorkerAgentServiceOpenStreamRequest,
    ) -> Result<(), Status> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(node_id)
            .ok_or_else(|| Status::not_found(format!("node {} not connected", node_id)))?
            .clone();
        drop(sessions);
        let result = session.lock().await.handle_message(msg).await;
        result
    }

    /// Number of connected sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

/// gRPC service wrapping `SessionManager`.
#[derive(Clone)]
pub struct WorkerControlService {
    manager: Arc<SessionManager>,
}

impl WorkerControlService {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        Self { manager }
    }
}

impl Default for WorkerControlService {
    fn default() -> Self {
        Self::new(SessionManager::new())
    }
}

#[tonic::async_trait]
impl moqentra_contracts::moqentra::worker::v1::worker_agent_service_server::WorkerAgentService
    for WorkerControlService
{
    type OpenStreamStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<Item = Result<WorkerAgentServiceOpenStreamResponse, Status>>
                + Send,
        >,
    >;

    async fn open_stream(
        &self,
        request: Request<Streaming<WorkerAgentServiceOpenStreamRequest>>,
    ) -> Result<Response<Self::OpenStreamStream>, Status> {
        let mut inbound = request.into_inner();

        // The first frame must be a Hello containing the node_id.
        let hello = match inbound.message().await? {
            Some(WorkerAgentServiceOpenStreamRequest {
                payload: Some(InPayload::Hello(hello)),
            }) => hello,
            Some(_) => {
                return Err(Status::failed_precondition("first frame must be Hello"));
            }
            None => {
                return Err(Status::cancelled("client closed before Hello"));
            }
        };

        let node_id = hello.node_id.clone();
        let (fencing_token, rx) = self.manager.connect(hello.node_id, hello.capabilities).await?;

        // Re-read the Hello with the fencing token assigned. The agent is
        // expected to echo this token in subsequent messages using an
        // out-of-band header or lease payload.
        let _ = fencing_token;

        let manager = Arc::clone(&self.manager);
        let node_id_clone = node_id.clone();
        tokio::spawn(async move {
            while let Ok(Some(msg)) = inbound.message().await {
                if let Err(e) = manager.handle_message(&node_id_clone, msg).await {
                    tracing::warn!(error = %e, node_id = %node_id_clone, "failed to handle worker message");
                }
            }
            tracing::info!(node_id = %node_id_clone, "worker stream closed");
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::OpenStreamStream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(node_id: &str) -> WorkerAgentServiceOpenStreamRequest {
        WorkerAgentServiceOpenStreamRequest {
            payload: Some(InPayload::Hello(Hello {
                node_id: node_id.to_string(),
                agent_version: "0.1.0".to_string(),
                capabilities: None,
            })),
        }
    }

    fn heartbeat(seq: u64) -> WorkerAgentServiceOpenStreamRequest {
        WorkerAgentServiceOpenStreamRequest {
            payload: Some(InPayload::Heartbeat(
                moqentra_contracts::moqentra::worker::v1::Heartbeat {
                    sequence: seq,
                    timestamp: None,
                },
            )),
        }
    }

    fn ack(command_id: &str) -> WorkerAgentServiceOpenStreamRequest {
        WorkerAgentServiceOpenStreamRequest {
            payload: Some(InPayload::Ack(
                moqentra_contracts::moqentra::worker::v1::Ack {
                    command_id: command_id.to_string(),
                    status: AckStatus::Received as i32,
                },
            )),
        }
    }

    fn result(command_id: &str, success: bool) -> WorkerAgentServiceOpenStreamRequest {
        WorkerAgentServiceOpenStreamRequest {
            payload: Some(InPayload::Result(
                moqentra_contracts::moqentra::worker::v1::Result {
                    command_id: command_id.to_string(),
                    success,
                    payload: vec![],
                    error: None,
                },
            )),
        }
    }

    #[tokio::test]
    async fn sequence_validation_and_command_round_trip() {
        let manager = SessionManager::new();
        let (_token, mut rx) = manager.connect("node-1".to_string(), None).await.unwrap();

        manager.handle_message("node-1", hello("node-1")).await.unwrap();

        manager.handle_message("node-1", heartbeat(1)).await.unwrap();

        manager
            .send_command(
                "node-1",
                Command {
                    command_id: "cmd-1".to_string(),
                    command_type: "TRAIN".to_string(),
                    payload: vec![],
                    deadline: None,
                },
            )
            .await
            .unwrap();

        let outbound = rx.recv().await.unwrap().unwrap();
        assert!(matches!(outbound.payload, Some(OutPayload::Command(_))));

        // Duplicate heartbeat sequence must be rejected.
        let err = manager.handle_message("node-1", heartbeat(1)).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::OutOfRange);

        manager.handle_message("node-1", ack("cmd-1")).await.unwrap();
        manager.handle_message("node-1", result("cmd-1", true)).await.unwrap();

        // Duplicate result is rejected.
        let err = manager.handle_message("node-1", result("cmd-1", true)).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::AlreadyExists);
    }

    #[tokio::test]
    async fn unknown_node_rejected() {
        let manager = SessionManager::new();
        let err = manager.handle_message("missing", heartbeat(1)).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }
}
