use axum::{
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
// Use the non-blocking reqwest client
use reqwest::Client as HttpClient; // <--- Changed this line
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, sync::Arc};
use thiserror::Error;
use tracing::{error, info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;
use serde_json;

// --- Combined App State ---
#[derive(Clone)]
struct AppState {
    config: AppConfig,
    http_client: Arc<HttpClient>, // Now holds the async client
}

// --- Configuration ---
#[derive(Clone)]
struct AppConfig {
    target_sms_provider_account_sid: String,
    target_sms_provider_auth_token: String,
    target_sms_provider_from_number: String,
    target_sms_provider_api_base: String,
}

// --- AWS SNS Request Mimic ---
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SnsPublishRequest {
    action: String,
    version: String,
    phone_number: String,
    message: String,
}

// --- AWS SNS Response Mimic ---
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PublishResult {
    message_id: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ResponseMetadata {
    request_id: String,
}

#[derive(Serialize, Debug)]
#[serde(rename = "PublishResponse", rename_all = "PascalCase")]
struct SnsPublishResponse {
    #[serde(rename = "xmlns")]
    xmlns: String,
    publish_result: PublishResult,
    response_metadata: ResponseMetadata,
}

impl SnsPublishResponse {
    fn new(message_id: String, request_id: String) -> Self {
        SnsPublishResponse {
            xmlns: "http://sns.amazonaws.com/doc/2010-03-31/".to_string(),
            publish_result: PublishResult { message_id },
            response_metadata: ResponseMetadata { request_id },
        }
    }
}

// --- Target SMS Provider Logic (Example: Twilio) ---
#[derive(Serialize)]
struct TwilioMessageRequest {
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "Body")]
    body: String,
}

#[derive(Deserialize, Debug)]
struct TwilioMessageResponse {
    sid: String,
}

#[derive(Debug, Error)]
enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Invalid incoming request: {0}")]
    BadRequest(String),
    // reqwest::Error can be directly converted using #[from]
    #[error("Failed to call downstream SMS provider: {0}")]
    DownstreamHttp(#[from] reqwest::Error),
    #[error("Downstream SMS provider reported failure: {0}")]
    DownstreamApi(String),
    #[error("Internal server error: {0}")]
    Internal(String),
    // JoinError is no longer needed as we removed spawn_blocking
    // #[error("Task join error: {0}")]
    // JoinError(#[from] tokio::task::JoinError),
}

// Implement IntoResponse for our custom error
impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        error!("Error processing request: {}", self);
        let (status, body) = match self {
            ProxyError::BadRequest(msg) => (StatusCode::BAD_REQUEST, format!("BadRequest: {}", msg)),
            ProxyError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("ConfigurationError: {}", msg)),
            ProxyError::DownstreamHttp(_) | ProxyError::DownstreamApi(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "FailedDependency: Error communicating with downstream SMS provider".to_string())
            }
            ProxyError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("InternalError: {}", msg)),
            // ProxyError::JoinError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError: Background task failed".to_string()),
        };
        (status, body).into_response()
    }
}


// --- Axum Handler ---
#[instrument(skip(state, headers, form))]
async fn handle_sns_publish(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<SnsPublishRequest>,
) -> Result<impl IntoResponse, ProxyError> {

    info!("Received SNS Publish request for number: {}", form.phone_number);
    if form.action != "Publish" {
        return Err(ProxyError::BadRequest("Action must be Publish".to_string()));
    }
    // WARNING: No Signature V4 validation

    // --- Call Downstream Provider (Twilio Example) directly using async client ---
    let twilio_api_url = format!(
        "{}/Accounts/{}/Messages.json",
        state.config.target_sms_provider_api_base.trim_end_matches('/'),
        state.config.target_sms_provider_account_sid
    );

    // Create the request body - cloning strings here is fine
    let twilio_request = TwilioMessageRequest {
        to: form.phone_number.clone(),
        from: state.config.target_sms_provider_from_number.clone(),
        body: form.message.clone(),
    };

    info!("Sending request to Twilio API: {}", twilio_api_url);

    // Use the async client directly and .await the futures
    let twilio_response = state.http_client // Access the async client from state
        .post(&twilio_api_url)
        .basic_auth(&state.config.target_sms_provider_account_sid, Some(&state.config.target_sms_provider_auth_token))
        .form(&twilio_request)
        .send()
        .await?; // <--- .await the send() future

    info!("Received response from Twilio: status={}", twilio_response.status());

    let response_status = twilio_response.status();
    // Use .text() which is also async
    let response_text = twilio_response.text().await?; // <--- .await the text() future

    if !response_status.is_success() {
         error!("Twilio API Error: Status={}, Body={}", response_status, response_text);
         return Err(ProxyError::DownstreamApi(format!(
             "Twilio returned status {}",
             response_status
         )));
    }

    let twilio_message: TwilioMessageResponse = serde_json::from_str(&response_text)
        .map_err(|e| ProxyError::DownstreamApi(format!("Failed to parse Twilio success response: {}. Body: {}", e, response_text)))?;


    info!("Successfully sent message via Twilio. SID: {}", twilio_message.sid);

    // --- Construct AWS SNS Mimic Response ---
    let response = SnsPublishResponse::new(
        twilio_message.sid,
        Uuid::new_v4().to_string(),
    );

    let xml_response = format!(
        r#"<PublishResponse xmlns="{}">
             <PublishResult><MessageId>{}</MessageId></PublishResult>
             <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
           </PublishResponse>"#,
        response.xmlns,
        response.publish_result.message_id,
        response.response_metadata.request_id
    );

    Ok((StatusCode::OK, [("content-type", "text/xml")], xml_response))
}


// --- Main Function ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig {
        target_sms_provider_account_sid: env::var("TWILIO_ACCOUNT_SID")
            // .expect("TWILIO_ACCOUNT_SID not set"),
            .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_auth_token: env::var("TWILIO_AUTH_TOKEN")
            // .expect("TWILIO_AUTH_TOKEN not set"),
            .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_from_number: env::var("TWILIO_FROM_NUMBER")
            // .expect("TWILIO_FROM_NUMBER not set"),
            .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_api_base: env::var("TWILIO_API_BASE")
            .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
    };

    // Create the async reqwest client
    let http_client = Arc::new(HttpClient::new()); // <--- Creates the async client
    let app_state = Arc::new(AppState { config, http_client });

    let app = Router::new()
        .route("/", post(handle_sns_publish))
        .with_state(app_state);

    let port = env::var("PORT").unwrap_or_else(|_| "7700".to_string());
    let addr_str = format!("0.0.0.0:{}", port);
    let addr: SocketAddr = addr_str.parse()?;

    info!("SNS SMS Proxy listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}