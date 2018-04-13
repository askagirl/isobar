use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessages {
    OpenWorkspace { paths: Vec<PathBuf> },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    OpenWindow { workspace_id: usize },
}
