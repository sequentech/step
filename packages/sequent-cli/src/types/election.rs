use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateElectionRequest {
    pub name: String,
    pub description: String,
    pub election_event_id: String,
    pub tenant_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateElectionResponse {
    pub id: String,
}
