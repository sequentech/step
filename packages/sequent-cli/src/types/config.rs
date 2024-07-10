use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConfigData {
    pub auth_token: String,
    pub endpoint_url: String,
}
