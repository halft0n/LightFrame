use crate::python_sidecar::PythonSidecar;
use catchlight_core::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

pub struct AiDispatcher {
    python: Option<Arc<Mutex<PythonSidecar>>>,
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
                    self.python = Some(Arc::new(Mutex::new(sidecar)));
                }
                Err(e) => {
                    tracing::warn!("Python sidecar unavailable, AI features degraded: {e}");
                }
            }
        }
        Ok(())
    }

    pub fn python_sidecar(&self) -> Option<Arc<Mutex<PythonSidecar>>> {
        self.python.clone()
    }
}

impl Default for AiDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_dispatcher_has_no_python() {
        let d = AiDispatcher::new();
        assert!(!d.is_python_available());
    }

    #[test]
    fn default_dispatcher_has_no_python() {
        let d = AiDispatcher::default();
        assert!(!d.is_python_available());
    }

    #[tokio::test]
    async fn ensure_python_degrades_gracefully() {
        let mut d = AiDispatcher::new();
        let result = d.ensure_python().await;
        assert!(
            result.is_ok(),
            "should not error even if Python unavailable"
        );
    }
}
