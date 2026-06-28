use crate::python_sidecar::PythonSidecar;
use catchlight_core::Result;
use tracing::info;

pub struct AiDispatcher {
    python: Option<PythonSidecar>,
}

impl AiDispatcher {
    pub fn new() -> Self {
        Self { python: None }
    }

    pub fn is_python_available(&self) -> bool {
        self.python.is_some()
    }

    pub async fn ensure_python(&mut self) -> Result<()> {
        if self.python.is_none() {
            match PythonSidecar::spawn().await {
                Ok(sidecar) => {
                    info!("Python AI sidecar started");
                    self.python = Some(sidecar);
                }
                Err(e) => {
                    tracing::warn!("Python sidecar unavailable, AI features degraded: {e}");
                }
            }
        }
        Ok(())
    }
}

impl Default for AiDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
