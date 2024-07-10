use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionEventRequest {
    pub name: String,
    pub description: String,
    pub encryption_protocol: String,
    pub tenant_id: String,
    pub presentation: serde_json::Value,
    pub is_archived: bool,
}

#[derive(Deserialize)]
pub struct CreateElectionEventResponse {
    pub id: String,
}
