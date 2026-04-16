use crate::strategies::Strategy;

pub struct VwapReversion {
    pub name: String,
}

impl VwapReversion {
    pub fn new() -> Self {
        Self {
            name: "VWAP Mean Reversion".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Strategy for VwapReversion {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&self) {
        tracing::info!("Starting VWAP Mean Reversion strategy...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
