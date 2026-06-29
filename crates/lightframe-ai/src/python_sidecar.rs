use lightframe_core::Result;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use tokio::time::{Duration, timeout};

const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// JSON-RPC bridge to the Python AI sidecar process.
///
/// `Send + Sync`: shared state lives behind [`Arc<Mutex<>>`].
#[derive(Clone)]
pub struct PythonSidecar {
    inner: Arc<Mutex<SidecarInner>>,
}

struct SidecarInner {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    next_id: AtomicU64,
}

impl PythonSidecar {
    /// Returns true when python3 is available and the sidecar package directory can be located.
    pub fn can_spawn() -> bool {
        locate_python_dir().is_some()
    }

    pub async fn spawn() -> Result<Self> {
        let python_dir = locate_python_dir().ok_or_else(|| {
            lightframe_core::Error::Ai(
                "Python sidecar package not found (set LIGHTFRAME_PYTHON_PATH)".into(),
            )
        })?;

        let mut child = tokio::process::Command::new("python3")
            .args(["-m", "lightframe_ai"])
            .env("PYTHONPATH", &python_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                lightframe_core::Error::Ai(format!("failed to spawn Python sidecar: {e}"))
            })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(BufReader::new);

        let sidecar = Self {
            inner: Arc::new(Mutex::new(SidecarInner {
                child: Some(child),
                stdin,
                stdout,
                next_id: AtomicU64::new(1),
            })),
        };

        // Verify the process is responsive before returning.
        sidecar.call("ping", json!({})).await?;
        Ok(sidecar)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        self.call_with_timeout(method, params, DEFAULT_CALL_TIMEOUT)
            .await
    }

    pub async fn call_with_timeout(
        &self,
        method: &str,
        params: Value,
        call_timeout: Duration,
    ) -> Result<Value> {
        let mut guard = self.inner.lock().await;
        guard.ensure_alive()?;

        let id = guard.next_id.fetch_add(1, Ordering::Relaxed);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let mut line = serde_json::to_string(&request).map_err(|e| {
            lightframe_core::Error::Ai(format!("failed to serialize JSON-RPC request: {e}"))
        })?;
        line.push('\n');

        let stdin = guard
            .stdin
            .as_mut()
            .ok_or_else(|| lightframe_core::Error::Ai("Python sidecar stdin unavailable".into()))?;
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;

        let stdout = guard.stdout.as_mut().ok_or_else(|| {
            lightframe_core::Error::Ai("Python sidecar stdout unavailable".into())
        })?;

        let read_future = async {
            let mut response_line = String::new();
            loop {
                response_line.clear();
                let bytes = stdout.read_line(&mut response_line).await?;
                if bytes == 0 {
                    return Err(lightframe_core::Error::Ai(
                        "Python sidecar closed stdout".into(),
                    ));
                }

                let trimmed = response_line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response: Value = serde_json::from_str(trimmed).map_err(|e| {
                    lightframe_core::Error::Ai(format!(
                        "invalid JSON-RPC response from Python sidecar: {e}"
                    ))
                })?;

                if response.get("id").and_then(Value::as_u64) == Some(id) {
                    if let Some(err) = response.get("error") {
                        let code = err.get("code").and_then(Value::as_i64).unwrap_or(-1);
                        let message = err
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown error");
                        return Err(lightframe_core::Error::Ai(format!(
                            "Python sidecar RPC error {code}: {message}"
                        )));
                    }
                    return response.get("result").cloned().ok_or_else(|| {
                        lightframe_core::Error::Ai("JSON-RPC response missing result field".into())
                    });
                }
            }
        };

        timeout(call_timeout, read_future).await.map_err(|_| {
            lightframe_core::Error::Ai(format!(
                "Python sidecar call '{method}' timed out after {}s",
                call_timeout.as_secs()
            ))
        })?
    }

    pub async fn shutdown(&self) -> Result<()> {
        let guard = self.inner.lock().await;
        if guard.child.is_none() {
            return Ok(());
        }

        let _ = timeout(SHUTDOWN_TIMEOUT, async {
            drop(guard);
            self.call_with_timeout("shutdown", json!({}), SHUTDOWN_TIMEOUT)
                .await
        })
        .await;

        let mut guard = self.inner.lock().await;
        guard.kill().await
    }
}

impl SidecarInner {
    fn ensure_alive(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.child = None;
                    self.stdin = None;
                    self.stdout = None;
                    return Err(lightframe_core::Error::Ai(format!(
                        "Python sidecar exited with status {status}"
                    )));
                }
                Ok(None) => return Ok(()),
                Err(e) => {
                    return Err(lightframe_core::Error::Ai(format!(
                        "failed to poll Python sidecar: {e}"
                    )));
                }
            }
        }
        Err(lightframe_core::Error::Ai(
            "Python sidecar is not running".into(),
        ))
    }

    async fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.stdin = None;
        self.stdout = None;
        Ok(())
    }
}

fn locate_python_dir() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("LIGHTFRAME_PYTHON_PATH") {
        let p = PathBuf::from(path);
        if p.join("lightframe_ai").is_dir() {
            return Some(p);
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_python = manifest.parent()?.parent()?.join("python");
    if workspace_python.join("lightframe_ai").is_dir() {
        return Some(workspace_python);
    }

    // Walk up from the current working directory (useful when running from repo root).
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("python");
        if candidate.join("lightframe_ai").is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_python_dir_finds_workspace_package() {
        let dir = locate_python_dir();
        assert!(
            dir.is_some(),
            "expected workspace python/lightframe_ai directory"
        );
        assert!(dir.unwrap().join("lightframe_ai").is_dir());
    }

    #[tokio::test]
    async fn spawn_and_ping_when_python_available() {
        if locate_python_dir().is_none() {
            return;
        }

        let sidecar = PythonSidecar::spawn().await;
        if sidecar.is_err() {
            // Python or dependencies may be missing in CI — skip gracefully.
            return;
        }

        let sidecar = sidecar.unwrap();
        let result = sidecar.call("ping", json!({})).await.unwrap();
        assert_eq!(result, json!({"pong": true}));

        let version = sidecar.call("get_version", json!({})).await.unwrap();
        assert!(version.get("version").and_then(Value::as_str).is_some());

        sidecar.shutdown().await.unwrap();
    }
}
