use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMessage {
    pub action: String, // "create", "update", "delete"
    pub collection: String,
    pub record: serde_json::Value,
}

#[derive(Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<RealtimeMessage>,
}

impl RealtimeHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeMessage> {
        self.sender.subscribe()
    }

    pub fn broadcast(&self, message: RealtimeMessage) {
        let _ = self.sender.send(message);
    }
}
