use crate::strategies::Strategy;

pub struct GammaScalping {
    pub name: String,
}

impl GammaScalping {
    pub fn new() -> Self {
        Self {
            name: "Gamma Scalping".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Strategy for GammaScalping {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&self) {
        tracing::info!("Starting Gamma Scalping strategy...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
