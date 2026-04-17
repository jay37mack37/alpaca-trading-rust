use crate::strategies::Strategy;

pub struct PutCallParity {
    pub name: String,
}

impl PutCallParity {
    pub fn new() -> Self {
        Self {
            name: "Put-Call Parity".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Strategy for PutCallParity {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&self) {
        tracing::info!("Starting Put-Call Parity strategy...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
