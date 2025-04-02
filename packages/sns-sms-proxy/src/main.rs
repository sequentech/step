use axum::{
    extract::{ConnectInfo, Form, State},
    http::{HeaderMap, HeaderName, StatusCode}, // Import HeaderName here
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, sync::Arc, time::Duration}; // Import Duration
use thiserror::Error;
// Import TraceLayer
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, instrument, trace, warn, Level, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;
use serde_json;


// --- Combined App State ---
#[derive(Clone)]
struct AppState {
    config: AppConfig,
    http_client: Arc<HttpClient>,
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
#[derive(Deserialize, Debug, Clone)]
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
#[derive(Serialize, Debug)]
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
    #[error("Failed to call downstream SMS provider: {0}")]
    DownstreamHttp(#[from] reqwest::Error),
    #[error("Downstream SMS provider reported failure: {0}")]
    DownstreamApi(String),
    #[error("Internal server error: {0}")]
    Internal(String),
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
        };
        (status, body).into_response()
    }
}


// --- Axum Handler (for POST /) ---
#[instrument(skip(state, headers, form), fields(source_ip, request_id = %Uuid::new_v4()))]
async fn handle_sns_publish(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<SnsPublishRequest>,
) -> Result<impl IntoResponse, ProxyError> {

    Span::current().record("source_ip", &tracing::field::display(&addr));

    info!(
        action = %form.action,
        target_phone_number = %form.phone_number, // Consider masking
        message_length = %form.message.len(),
        "Processing matched SNS Publish request (POST /)"
    );

    trace!("Incoming headers for POST /:");
    // Iterate over incoming headers
    for (name, value) in headers.iter() {
        // Use as_str() on HeaderName directly
        if name.as_str().to_lowercase() != "authorization" {
             // FIX: Use ?name (Debug) for name and ?value (Debug) for value
             // FIX: Correct typo value? to ?value
             trace!(header_name = ?name, header_value = ?value);
        } else {
             // FIX: Use ?name (Debug) for name
             trace!(header_name = ?name, header_value = "***REDACTED***");
        }
    }

    trace!(parsed_request_body = ?form, "Parsed incoming request body for POST /");


    if form.action != "Publish" {
         error!(action = %form.action, "Received request with invalid action in POST /");
         return Err(ProxyError::BadRequest(format!("Action must be Publish, got: {}", form.action)));
    }

    let twilio_api_url = format!(
        "{}/Accounts/{}/Messages.json",
        state.config.target_sms_provider_api_base.trim_end_matches('/'),
        state.config.target_sms_provider_account_sid
    );

    let twilio_request = TwilioMessageRequest {
        to: form.phone_number.clone(),
        from: state.config.target_sms_provider_from_number.clone(),
        body: form.message.clone(),
    };

    debug!(downstream_url = %twilio_api_url, downstream_request = ?twilio_request, "Sending request to downstream SMS provider");

    let downstream_req_builder = state.http_client
        .post(&twilio_api_url)
        .basic_auth(&state.config.target_sms_provider_account_sid, Some(&state.config.target_sms_provider_auth_token))
        .form(&twilio_request);

    // REMOVED: Loop attempting to log outgoing headers using faulty placeholder
    // trace!("Sending downstream headers:");
    // for (name, value) in downstream_req_builder.headers_debug() { ... }

    let twilio_response = downstream_req_builder.send().await?;

    let response_status = twilio_response.status();
    let response_bytes = twilio_response.bytes().await?;
    let response_text = String::from_utf8_lossy(&response_bytes).to_string();


    debug!(
        downstream_status = %response_status,
        downstream_body = %response_text,
        "Received response from downstream SMS provider"
    );


    if !response_status.is_success() {
         error!("Downstream SMS provider API Error reported");
         return Err(ProxyError::DownstreamApi(format!(
             "Downstream provider returned status {}",
             response_status
         )));
    }

    let twilio_message: TwilioMessageResponse = serde_json::from_str(&response_text)
        .map_err(|e| {
            error!(error = %e, "Failed to parse downstream success response");
            ProxyError::DownstreamApi(format!("Failed to parse downstream success response: {}", e))
        })?;


    info!(message_sid = %twilio_message.sid, "Successfully processed request via downstream provider");

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
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "trace".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig {
        target_sms_provider_account_sid: env::var("TWILIO_ACCOUNT_SID")
        .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_auth_token: env::var("TWILIO_AUTH_TOKEN")
        .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_from_number: env::var("TWILIO_FROM_NUMBER")
        .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
        target_sms_provider_api_base: env::var("TWILIO_API_BASE")
            .unwrap_or_else(|_| "https://api.twilio.com/2010-04-01".to_string()),
    };

    let http_client = Arc::new(HttpClient::new());
    let app_state = Arc::new(AppState { config, http_client });

    let app_routes = Router::new()
        .route("/", post(handle_sns_publish))
        .with_state(app_state);

    // Apply the TraceLayer to log all requests
    let app = app_routes.layer(
        TraceLayer::new_for_http()
    );

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr_str = format!("0.0.0.0:{}", port);
    let addr: SocketAddr = addr_str.parse()?;

    info!("SNS SMS Proxy listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>()
    ).await?;

    Ok(())
}

// REMOVED: Faulty/Placeholder RequestBuilderExt trait and impl
// trait RequestBuilderExt { ... }
// impl RequestBuilderExt for reqwest::RequestBuilder { ... }

