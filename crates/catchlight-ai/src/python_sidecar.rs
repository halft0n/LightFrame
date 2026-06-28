use catchlight_core::Result;
use serde_json::Value;

pub struct PythonSidecar {
    _process: tokio::process::Child,
}

impl PythonSidecar {
    pub async fn spawn() -> Result<Self> {
        let process = tokio::process::Command::new("python3")
            .args(["-m", "catchlight_ai_py"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                catchlight_core::Error::Ai(format!("failed to spawn Python sidecar: {e}"))
            })?;

        Ok(Self { _process: process })
    }

    pub async fn call(&mut self, _method: &str, _params: Value) -> Result<Value> {
        // TODO: implement JSON-RPC over stdin/stdout
        Err(catchlight_core::Error::Ai("not yet implemented".into()))
    }
}
