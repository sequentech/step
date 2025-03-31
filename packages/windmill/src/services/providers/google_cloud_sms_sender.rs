use reqwest::Client;
use serde::Serialize;
use std::error::Error;


#[derive(Serialize)]
struct SessionlessSmsRequest {
    from_phone: String,
    phone_number: Vec<String>,
    message: Vec<String>,
}

pub struct GoogleCloudSmsSender {
    client: Client,
    access_token: String,
    from_phone: String,
}

impl GoogleCloudSmsSender {
    pub fn new() -> Self {
        let access_token = std::env::var("GOOGLE_CLOUD_ACCESS_TOKEN")
            .expect("GOOGLE_CLOUD_ACCESS_TOKEN environment variable not set");
        let from_phone = std::env::var("SMS_FROM_PHONE")
        .expect("SMS_FROM_PHONE environment variable not set");
        Self {
            client: Client::new(),
            access_token,
            from_phone,
        }
    }

    /// Sends an SMS using the Sessionless Outbound SMS API.
    pub async fn send_sms(&self, receiver: String, message: String) -> Result<String, Box<dyn Error>> {

        let url = std::env::var("GOOGLE_OUTBOUND_SMS_API_URL")
        .expect("GOOGLE_OUTBOUND_SMS_API_URL environment variable not set");

        let payload = SessionlessSmsRequest {
            from_phone: self.from_phone.clone(),
            phone_number: vec![receiver],
            message: vec![message]
        };

        let response = self.client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;
        Ok(format!("Status: {}, Response: {}", status, response_text))
    }
}