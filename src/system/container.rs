// src/system/container.rs

use reqwest::Client;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ContainerStatus {
    pub container_running: bool,
    pub service_online: bool,
    pub available_models: Vec<String>,
    pub message: String,
}

fn detect_runtime() -> &'static str {
    if Command::new("podman")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        "podman"
    } else if Command::new("docker")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        "docker"
    } else {
        "podman"
    }
}

/// Checks if the name/ID refers to a Podman Pod
pub fn is_pod(name: &str) -> bool {
    let rt = detect_runtime();
    if rt != "podman" {
        return false;
    }
    Command::new(rt)
        .args(["pod", "exists", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Executes `podman pod start <name>` or `<runtime> start <name>`
pub fn start_container(name: &str) -> Result<String, String> {
    let rt = detect_runtime();
    let is_pod_target = rt == "podman" && is_pod(name);
    let args = if is_pod_target {
        vec!["pod", "start", name]
    } else {
        vec!["start", name]
    };

    let output = Command::new(rt)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", rt, e))?;

    if output.status.success() {
        let kind = if is_pod_target { "Pod" } else { "Container" };
        Ok(format!("{} {} '{}' started.", rt, kind, name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("{} start failed: {}", rt, err.trim()))
    }
}

/// Executes `podman pod stop <name>` or `<runtime> stop <name>`
pub fn stop_container(name: &str) -> Result<String, String> {
    let rt = detect_runtime();
    let is_pod_target = rt == "podman" && is_pod(name);
    let args = if is_pod_target {
        vec!["pod", "stop", name]
    } else {
        vec!["stop", name]
    };

    let output = Command::new(rt)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", rt, e))?;

    if output.status.success() {
        let kind = if is_pod_target { "Pod" } else { "Container" };
        Ok(format!("{} {} '{}' stopped.", rt, kind, name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("{} stop failed: {}", rt, err.trim()))
    }
}

/// Checks whether a container or pod is currently running
pub fn is_container_running(name: &str) -> bool {
    let rt = detect_runtime();
    if rt == "podman" && is_pod(name) {
        let output = Command::new(rt)
            .args(["pod", "inspect", "-f", "{{.State}}", name])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let state = stdout.trim();
                state.eq_ignore_ascii_case("running") || state.eq_ignore_ascii_case("degraded")
            }
            _ => false,
        }
    } else {
        let output = Command::new(rt)
            .args(["inspect", "-f", "{{.State.Running}}", name])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.trim() == "true"
            }
            _ => false,
        }
    }
}

/// Asynchronous health check querying Ollama's HTTP API at `/api/tags`
pub async fn check_service_health(container_name: &str, endpoint_url: &str) -> ContainerStatus {
    let c_name = container_name.to_string();
    let running = tokio::task::spawn_blocking(move || is_container_running(&c_name))
        .await
        .unwrap_or(false);

    let client = Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .unwrap_or_default();

    let url = format!("{}/api/tags", endpoint_url.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let mut models = Vec::new();
            if let Ok(json) = resp.json::<Value>().await
                && let Some(arr) = json.get("models").and_then(|v| v.as_array())
            {
                for m in arr {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        models.push(name.to_string());
                    }
                }
            }

            ContainerStatus {
                container_running: running,
                service_online: true,
                available_models: models,
                message: "Ollama service is online & responsive.".to_string(),
            }
        }
        Ok(resp) => ContainerStatus {
            container_running: running,
            service_online: false,
            available_models: Vec::new(),
            message: format!("HTTP error: status {}", resp.status()),
        },
        Err(_) => ContainerStatus {
            container_running: running,
            service_online: false,
            available_models: Vec::new(),
            message: "Cannot connect to service (daemon unreachable).".to_string(),
        },
    }
}
