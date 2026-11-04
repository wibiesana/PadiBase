use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMessage {
    pub action: String, // "create", "update", "delete"
    pub table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    pub record: serde_json::Value,
}

impl RealtimeMessage {
    pub fn new(action: impl Into<String>, table: impl Into<String>, record: serde_json::Value) -> Self {
        let t = table.into();
        Self {
            action: action.into(),
            table: t.clone(),
            collection: Some(t),
            record,
        }
    }
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
