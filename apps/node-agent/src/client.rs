//! gRPC worker/agent client for the local node agent.

use moqentra_contracts::moqentra::worker::v1::{
    worker_agent_service_client::WorkerAgentServiceClient,
    worker_agent_service_open_stream_request::Payload as InPayload,
    worker_agent_service_open_stream_response::Payload as OutPayload, Ack, AckStatus, Command,
    Framework, Hello, Lease, ModelFormat, Progress, WorkerAgentServiceOpenStreamRequest,
    WorkerAgentServiceOpenStreamResponse, WorkerCapabilities,
};
use moqentra_worker_control::local_executor::{AcceleratorKind, NodeCapabilities};
use std::time::Duration;
use tokio::sync::mpsc;
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
        device_memory_bytes += d.memory_mib * 1024 * 1024;
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

fn ack_request(command_id: &str, status: AckStatus) -> WorkerAgentServiceOpenStreamRequest {
    WorkerAgentServiceOpenStreamRequest {
        payload: Some(InPayload::Ack(Ack {
            command_id: command_id.to_string(),
            status: status as i32,
        })),
    }
}

fn _progress_request(
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

/// Connect to the control plane and maintain the worker stream.
pub async fn run_worker_stream(
    dst: &str,
    capabilities: NodeCapabilities,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client: WorkerAgentServiceClient<Channel> =
        WorkerAgentServiceClient::connect(dst.to_string()).await?;
    let (tx, rx): (
        mpsc::Sender<WorkerAgentServiceOpenStreamRequest>,
        mpsc::Receiver<WorkerAgentServiceOpenStreamRequest>,
    ) = mpsc::channel(64);
    let request = tonic::Request::new(ReceiverStream::new(rx));

    let _: Result<(), _> = tx.send(hello_request(&capabilities, env!("CARGO_PKG_VERSION"))).await;

    let mut inbound = client.open_stream(request).await?.into_inner();
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
                        handle_payload(&tx, payload, &capabilities).await?;
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
    _caps: &NodeCapabilities,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match payload {
        OutPayload::Command(Command { command_id, .. }) => {
            tracing::info!(command_id = %command_id, "received command");
            tx.send(ack_request(&command_id, AckStatus::Received)).await?;
            // Simulated progress/result. Real implementation would launch
            // the OCI container via LocalExecutor and stream logs/metrics.
            tx.send(_progress_request(&command_id, 50, "processing")).await?;
            tx.send(WorkerAgentServiceOpenStreamRequest {
                payload: Some(InPayload::Result(
                    moqentra_contracts::moqentra::worker::v1::Result {
                        command_id,
                        success: true,
                        payload: vec![],
                        error: None,
                    },
                )),
            })
            .await?;
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
            tracing::info!(graceful = d.graceful, "received drain request");
            return Err("drain requested".into());
        }
        OutPayload::Error(e) => {
            tracing::error!(error = ?e, "control plane error");
            return Err("control plane error".into());
        }
    }
    Ok(())
}
