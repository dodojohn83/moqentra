//! gRPC worker/agent client for the local node agent.

use moqentra_contracts::moqentra::worker::v1::{
    worker_agent_service_client::WorkerAgentServiceClient,
    worker_agent_service_open_stream_request::Payload as InPayload,
    worker_agent_service_open_stream_response::Payload as OutPayload, Ack, AckStatus, Command,
    Framework, Hello, Lease, LogChunk, ModelFormat, Progress, WorkerAgentServiceOpenStreamRequest,
    WorkerAgentServiceOpenStreamResponse, WorkerCapabilities,
};
use moqentra_types::AttemptId;
use moqentra_worker_control::local_executor::{
    AcceleratorKind, AllocationRequest, BindMount, ContainerConfig, LocalExecutor, NodeCapabilities,
};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

fn to_worker_capabilities(caps: &NodeCapabilities, agent_version: &str) -> WorkerCapabilities {
    let mut device_labels = Vec::new();
    let mut device_memory_bytes: u64 = 0;
    let mut supports_gpu = false;
    let mut supports_npu = false;
    let mut driver_version = "n/a".to_string();
    let runtime_version = caps.container_runtime.clone();
    let mut runtimes = Vec::new();

    for d in &caps.devices {
        if !d.healthy {
            continue;
        }
        device_labels.push(d.uuid.clone());
        device_memory_bytes =
            device_memory_bytes.saturating_add(d.memory_mib.saturating_mul(1024 * 1024));
        match d.kind {
            AcceleratorKind::Nvidia | AcceleratorKind::Amd => supports_gpu = true,
            AcceleratorKind::Ascend => supports_npu = true,
            AcceleratorKind::Cpu => {}
        }
        if driver_version == "n/a" {
            driver_version = d.driver_version.clone();
        }
        if !d.runtime.is_empty() && !runtimes.contains(&d.runtime) {
            runtimes.push(d.runtime.clone());
        }
    }
    if runtimes.is_empty() {
        runtimes.push(caps.container_runtime.clone());
    }
    if device_labels.is_empty() {
        device_labels.push("cpu".to_string());
    }

    WorkerCapabilities {
        agent_build_version: agent_version.to_string(),
        contract_version: "1".to_string(),
        frameworks: vec![Framework {
            name: "PyTorch".to_string(),
            version: "2.6.0".to_string(),
        }],
        hardware_label: caps.node_id.to_string(),
        device_labels,
        driver_version,
        runtime_version,
        runtimes,
        model_formats: vec![ModelFormat {
            name: "onnx".to_string(),
            version: vec!["1.17".to_string()],
        }],
        collective_backend: "".to_string(),
        device_memory_bytes,
        max_parallelism: caps.cpu_cores,
        supports_gpu,
        supports_npu,
    }
}

fn hello_request(
    caps: &NodeCapabilities,
    agent_version: &str,
) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Hello(Hello {
            node_id: caps.node_id.to_string(),
            agent_version: agent_version.to_string(),
            capabilities: Some(to_worker_capabilities(caps, agent_version)),
        })),
    }
}

fn heartbeat_request(seq: u64) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Heartbeat(
            moqentra_contracts::moqentra::worker::v1::Heartbeat {
                sequence: seq,
                timestamp: None,
            },
        )),
    }
}

#[allow(clippy::as_conversions)]
fn ack_request(command_id: &str, status: AckStatus) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Ack(Ack {
            command_id: command_id.to_string(),
            status: status as i32,
        })),
    }
}

fn progress_request(
    command_id: &str,
    percent: u32,
    message: &str,
) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Progress(Progress {
            command_id: command_id.to_string(),
            percent_complete: percent,
            message: message.to_string(),
        })),
    }
}

fn log_chunk_request(command_id: &str, data: Vec<u8>) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::LogChunk(LogChunk {
            command_id: command_id.to_string(),
            data,
        })),
    }
}

fn result_request(command_id: String, success: bool) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Result(
            moqentra_contracts::moqentra::worker::v1::Result {
                command_id,
                success,
                payload: vec![],
                error: None,
            },
        )),
    }
}

#[derive(serde::Deserialize)]
struct RunContainerCommand {
    attempt_id: String,
    cpu_cores: u32,
    memory_mib: u64,
    devices: Vec<AcceleratorKind>,
    device_count: u32,
    #[serde(default)]
    env: BTreeMap<String, String>,
    container: ContainerConfig,
}

type ChildrenMap = HashMap<String, tokio::process::Child>;

/// Connect to the control plane and maintain the worker stream.
pub async fn run_worker_stream(
    dst: &str,
    capabilities: NodeCapabilities,
    executor: Arc<Mutex<LocalExecutor>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let children: Arc<tokio::sync::Mutex<ChildrenMap>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let mut client: WorkerAgentServiceClient<Channel> =
        WorkerAgentServiceClient::connect(dst.to_string()).await?;
    let (tx, rx): (
        mpsc::Sender<WorkerAgentServiceOpenStreamRequest>,
        mpsc::Receiver<WorkerAgentServiceOpenStreamRequest>,
    ) = mpsc::channel(64);
    let request = tonic::Request::new(ReceiverStream::new(rx));

    let _: Result<(), _> = tx.send(hello_request(&capabilities, env!("CARGO_PKG_VERSION"))).await;

    let mut inbound = client.open_stream(request).await?.into_inner();

    // On startup, reconcile any containers from a previous process that have
    // already exceeded their lease deadline.
    let node_id = capabilities.node_id.to_string();
    let _ = executor
        .lock()
        .await
        .reconcile_containers(&node_id, &[], moqentra_types::UtcTimestamp::now())
        .await;

    let mut heartbeat_seq: u64 = 0;
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                heartbeat_seq += 1;
                if tx.send(heartbeat_request(heartbeat_seq)).await.is_err() {
                    break;
                }
            }
            msg = inbound.message() => {
                match msg {
                    Ok(Some(WorkerAgentServiceOpenStreamResponse { payload: Some(payload) })) => {
                        if let Err(e) = handle_payload(&tx, payload, &capabilities, &executor, &children).await {
                            tracing::error!(error = %e, "failed to handle worker payload");
                        }
                    }
                    Ok(Some(WorkerAgentServiceOpenStreamResponse { payload: None })) => {}
                    Ok(None) => {
                        tracing::info!("control plane closed worker stream");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "worker stream error");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_payload(
    tx: &mpsc::Sender<WorkerAgentServiceOpenStreamRequest>,
    payload: OutPayload,
    caps: &NodeCapabilities,
    executor: &Arc<Mutex<LocalExecutor>>,
    children: &Arc<Mutex<ChildrenMap>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match payload {
        OutPayload::Command(Command {
            command_id,
            command_type,
            payload,
            deadline,
            ..
        }) => {
            tracing::info!(command_id = %command_id, command_type = %command_type, "received command");
            tx.send(ack_request(&command_id, AckStatus::Received)).await?;

            if command_type == "RUN_CONTAINER" || command_type == "run_container" {
                run_container_command(
                    tx,
                    &command_id,
                    &payload,
                    deadline.as_ref(),
                    caps,
                    executor,
                    children,
                )
                .await?;
            } else {
                tx.send(progress_request(&command_id, 50, "processing")).await?;
                tx.send(result_request(command_id, true)).await?;
            }
        }
        OutPayload::Lease(Lease {
            lease_id,
            attempt_id,
            task_id,
            ..
        }) => {
            tracing::info!(%lease_id, %attempt_id, %task_id, "received lease");
        }
        OutPayload::Drain(d) => {
            if d.command_id.is_empty() {
                tracing::info!(graceful = d.graceful, "node drain requested");
                return Err("drain requested".into());
            }
            cancel_container(&d.command_id, d.graceful, children).await?;
        }
        OutPayload::Error(e) => {
            tracing::error!(error = ?e, "control plane error");
            return Err("control plane error".into());
        }
    }
    Ok(())
}

fn deadline_to_rfc3339(ts: &prost_types::Timestamp) -> String {
    let dt = time::OffsetDateTime::from_unix_timestamp(ts.seconds)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
        .checked_add(time::Duration::nanoseconds(i64::from(ts.nanos)))
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    moqentra_types::UtcTimestamp::new(dt).to_string()
}

async fn run_container_command(
    tx: &mpsc::Sender<WorkerAgentServiceOpenStreamRequest>,
    command_id: &str,
    payload: &[u8],
    deadline: Option<&prost_types::Timestamp>,
    caps: &NodeCapabilities,
    executor: &Arc<Mutex<LocalExecutor>>,
    children: &Arc<Mutex<ChildrenMap>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cmd: RunContainerCommand = serde_json::from_slice(payload)?;
    let attempt_id = AttemptId::try_from(cmd.attempt_id.as_str())?;

    // Allocate resources before launching the container.
    let mut guard = executor.lock().await;
    let allocation = guard.allocate(
        &AllocationRequest {
            attempt_id,
            cpu_cores: cmd.cpu_cores,
            memory_mib: cmd.memory_mib,
            devices: cmd.devices,
            device_count: cmd.device_count,
        },
        caps.node_id,
        caps,
        1,
    )?;
    let device_uuids: Vec<String> = allocation.device_uuids.iter().cloned().collect();
    let allocation_id = allocation.id.clone();
    let workspace = guard.workspace_root().to_path_buf();
    let mut container_config = cmd.container;
    let env_overrides = cmd.env;
    container_config
        .labels
        .insert("moqentra.io/node-id".to_string(), caps.node_id.to_string());
    container_config
        .labels
        .insert("moqentra.io/attempt-id".to_string(), cmd.attempt_id.clone());
    container_config
        .labels
        .insert("moqentra.io/command-id".to_string(), command_id.to_string());
    if let Some(ts) = deadline {
        container_config.labels.insert(
            "moqentra.io/lease-deadline".to_string(),
            deadline_to_rfc3339(ts),
        );
    }
    drop(guard);

    // Prepare bind mount sources under the controlled workspace and bind target.
    prepare_workspace_mounts(&workspace, &mut container_config).await?;

    let nv_devices = if device_uuids.is_empty() {
        None
    } else {
        Some(device_uuids.join(","))
    };

    let guard = executor.lock().await;
    let mut child = guard
        .run_container(&container_config, &env_overrides, nv_devices.as_deref())
        .await?;
    drop(guard);

    let stdout = child.stdout.take().ok_or("missing stdout")?;
    let stderr = child.stderr.take().ok_or("missing stderr")?;

    let mut guard: tokio::sync::MutexGuard<'_, ChildrenMap> = children.lock().await;
    guard.insert(command_id.to_string(), child);
    drop(guard);

    tx.send(progress_request(command_id, 25, "container started")).await?;

    let tx_out = tx.clone();
    let tx_err = tx.clone();
    let cid_out = command_id.to_string();
    let cid_err = command_id.to_string();

    let log_out = tokio::spawn(stream_lines(BufReader::new(stdout), cid_out, tx_out));
    let log_err = tokio::spawn(stream_lines(BufReader::new(stderr), cid_err, tx_err));

    let status: std::process::ExitStatus = {
        let mut guard: tokio::sync::MutexGuard<'_, ChildrenMap> = children.lock().await;
        let child = guard.get_mut(command_id).ok_or("container disappeared from tracking")?;
        child.wait().await?
    };

    children.lock().await.remove(command_id);

    let _ = tokio::try_join!(log_out, log_err);

    let mut guard = executor.lock().await;
    let _ = guard.release(&allocation_id);
    drop(guard);

    let success = status.success();
    tx.send(result_request(command_id.to_string(), success)).await?;
    Ok(())
}

async fn cancel_container(
    command_id: &str,
    graceful: bool,
    children: &Arc<tokio::sync::Mutex<ChildrenMap>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut guard: tokio::sync::MutexGuard<'_, ChildrenMap> = children.lock().await;
    let Some(child) = guard.get_mut(command_id) else {
        return Ok(());
    };

    if graceful {
        // Send SIGTERM first, then SIGKILL after a grace period if the
        // process is still tracked.
        if let Some(pid) = child.id() {
            let _ = std::process::Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
        }
        let children: Arc<tokio::sync::Mutex<ChildrenMap>> = Arc::clone(children);
        let cid = command_id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let mut guard: tokio::sync::MutexGuard<'_, ChildrenMap> = children.lock().await;
            if let Some(c) = guard.get_mut(&cid) {
                let _ = c.kill().await;
            }
        });
    } else {
        child.kill().await?;
    }
    Ok(())
}

async fn prepare_workspace_mounts(
    workspace: &Path,
    container: &mut ContainerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for mount in &container.bind_mounts {
        let src = Path::new(&mount.source);
        if !src.is_absolute() || !is_within_workspace(src, workspace).await {
            return Err(format!("bind mount {} escapes workspace", mount.source).into());
        }
    }

    // Always ensure input is read-only and output/checkpoint are writable.
    let required = vec![
        ("input", "/input", true),
        ("output", "/output", false),
        ("checkpoint", "/checkpoint", false),
    ];
    for (name, target, read_only) in required {
        let source = workspace.join(name).to_string_lossy().to_string();
        tokio::fs::create_dir_all(&source).await?;
        if !container.bind_mounts.iter().any(|m| m.target == target) {
            container.bind_mounts.push(BindMount {
                source,
                target: target.to_string(),
                read_only,
            });
        }
    }
    Ok(())
}

async fn is_within_workspace(path: &Path, workspace: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return false;
    }
    let Ok(workspace) = tokio::fs::canonicalize(workspace).await else {
        return false;
    };
    let Ok(target) = tokio::fs::canonicalize(path).await else {
        return false;
    };
    target.starts_with(&workspace)
}

async fn stream_lines<R>(
    reader: R,
    command_id: String,
    tx: mpsc::Sender<WorkerAgentServiceOpenStreamRequest>,
) where
    R: tokio::io::AsyncBufRead + Unpin,
{
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if tx.send(log_chunk_request(&command_id, line.into_bytes())).await.is_err() {
            break;
        }
    }
}
