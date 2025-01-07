use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub enabled: bool,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub area_name: String,
    pub middle_name: String,
    pub mobile_number: String,
    pub otp_method: String,
    pub embassy: String,
    pub country: String,
    pub id_card_number: String,
    pub id_card_type: String,
}