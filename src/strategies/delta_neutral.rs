use crate::strategies::Strategy;

pub struct DeltaNeutral {
    pub name: String,
}

impl DeltaNeutral {
    pub fn new() -> Self {
        Self {
            name: "0DTE Delta-Neutral".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Strategy for DeltaNeutral {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&self) {
        tracing::info!("Starting 0DTE Delta-Neutral strategy...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
