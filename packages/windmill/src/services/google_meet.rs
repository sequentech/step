// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tenant::get_tenant_by_id;
use deadpool_postgres::Transaction;
use google_calendar3::{
    api::{ConferenceData, ConferenceSolutionKey, CreateConferenceRequest, Event, EventDateTime},
    hyper_rustls, hyper_util,
    yup_oauth2::ServiceAccountAuthenticator,
    yup_oauth2::ServiceAccountKey,
    CalendarHub,
};
use rustls;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use tracing::{error, info, instrument};

#[derive(Deserialize, Debug, Clone)]
pub struct GenerateGoogleMeetBody {
    pub summary: String,
    pub description: String,
    pub start_date_time: String,
    pub end_date_time: String,
    pub time_zone: String,
    pub attendee_emails: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateGoogleMeetResponse {
    pub meet_link: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, EnumString)]
pub enum GoogleMeetError {
    ClientSecret(String),
    Json(String),
    OAuth2(String),
    GoogleApi(String),
    Http(String),
    DateTime(String),
    CalendarNotFound,
    MeetLinkNotFound,
    Other(String),
}

impl std::fmt::Display for GoogleMeetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoogleMeetError::ClientSecret(msg) => write!(f, "Client Secret not found: {}", msg),
            GoogleMeetError::Json(msg) => write!(f, "Json error: {}", msg),
            GoogleMeetError::OAuth2(msg) => write!(f, "OAuth2 error: {}", msg),
            GoogleMeetError::GoogleApi(msg) => write!(f, "Google API error: {}", msg),
            GoogleMeetError::Http(msg) => write!(f, "Http error: {}", msg),
            GoogleMeetError::DateTime(msg) => write!(f, "Date error: {}", msg),
            GoogleMeetError::CalendarNotFound => write!(f, "Calendar not found"),
            GoogleMeetError::MeetLinkNotFound => write!(f, "Meet link not found"),
            GoogleMeetError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

/// Implementation function for generating Google Meet links
/// Creates a calendar event with Google Meet integration using service account credentials
#[instrument(skip(hasura_transaction), err)]
pub async fn generate_google_meet_link_impl(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    meeting_data: &GenerateGoogleMeetBody,
) -> Result<String, GoogleMeetError> {
    // Get service account credentials from settings
    let settings = get_tenant_by_id(hasura_transaction, tenant_id)
        .await
        .map_err(|e| GoogleMeetError::ClientSecret(e.to_string()))?
        .settings
        .ok_or(GoogleMeetError::ClientSecret(
            "Tenant settings is null".to_string(),
        ))?;

    let gapi_key = settings
        .clone()
        .get("gapi_key")
        .ok_or(GoogleMeetError::ClientSecret(
            "gapi_key is null, no client secret in settings. Object must be named gapi_key"
                .to_string(),
        ))?
        .clone();

    let gapi_email = settings
        .clone()
        .get("gapi_email")
        .ok_or(GoogleMeetError::ClientSecret(
            "gapi_email is null. Field must be named gapi_email".to_string(),
        ))?
        .clone();

    // Parse service account key
    let service_account_key: ServiceAccountKey = match deserialize_value(gapi_key) {
        Ok(key) => key,
        Err(e) => {
            error!("Failed to parse service account key: {e:?}");
            return Err(GoogleMeetError::Json(
                "Failed to parse service account key".to_string(),
            ));
        }
    };

    let gapi_email_string: String = match deserialize_value(gapi_email) {
        Ok(key) => key,
        Err(e) => {
            error!("Failed to parse gapi_email: {e:?}");
            return Err(GoogleMeetError::Json(
                "Failed to parse gapi_email".to_string(),
            ));
        }
    };

    // Create authenticator with user impersonation
    // The service account must impersonate a Google Workspace user to create Google Meet conferences
    let auth = match ServiceAccountAuthenticator::builder(service_account_key)
        .subject(&gapi_email_string) // Impersonate the attendee (must be a Google Workspace user)
        .build()
        .await
    {
        Ok(auth) => auth,
        Err(e) => {
            error!("Failed to create authenticator: {e:?}");
            return Err(GoogleMeetError::OAuth2(
                "Failed to create authenticator".to_string(),
            ));
        }
    };

    let _ = rustls::crypto::ring::default_provider().install_default();

    // Create client and calendar hub
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_webpki_roots()
                .https_or_http()
                .enable_http1()
                .build(),
        );
    let hub = CalendarHub::new(client, auth);

    let conference_data = ConferenceData {
        create_request: Some(CreateConferenceRequest {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            conference_solution_key: Some(ConferenceSolutionKey {
                type_: Some("hangoutsMeet".to_string()),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let start_datetime = parse_datetime(&meeting_data.start_date_time, &meeting_data.time_zone)?;
    let end_datetime = parse_datetime(&meeting_data.end_date_time, &meeting_data.time_zone)?;
    let event = Event {
        summary: Some(meeting_data.summary.clone()),
        description: Some(meeting_data.description.clone()),
        start: Some(start_datetime),
        end: Some(end_datetime),
        conference_data: Some(conference_data),
        attendees: Some(
            meeting_data
                .attendee_emails
                .iter()
                .map(|email| google_calendar3::api::EventAttendee {
                    email: Some(email.clone()),
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    };

    // Insert the event into the calendar
    info!("Creating calendar event with Google Meet link");
    let result = hub
        .events()
        .insert(event, "primary")
        .conference_data_version(1) // Required for conference data
        .send_updates("all")
        .add_scope(google_calendar3::api::Scope::Event)
        .doit()
        .await;

    match result {
        Ok((_, created_event)) => {
            info!("Calendar event created successfully");

            // Extract Meet link from the response
            if let Some(conference_data) = created_event.conference_data {
                if let Some(entry_points) = conference_data.entry_points {
                    for entry_point in entry_points {
                        if entry_point.entry_point_type == Some("video".to_string()) {
                            if let Some(uri) = entry_point.uri {
                                info!("Google Meet link generated successfully.");
                                return Ok(uri);
                            }
                        }
                    }
                }
            }

            error!("Meet link not found in calendar event response");
            Err(GoogleMeetError::MeetLinkNotFound)
        }
        Err(e) => {
            error!("Failed to create calendar event: {e:?}");
            Err(GoogleMeetError::GoogleApi(
                "Failed to create calendar event".to_string(),
            ))
        }
    }
}

/// Parse datetime string with timezone into EventDateTime
#[instrument(err)]
fn parse_datetime(datetime_str: &str, timezone: &str) -> Result<EventDateTime, GoogleMeetError> {
    // The datetime should be in ISO 8601 format (e.g., "2025-09-29T12:45:00.000Z")
    let datetime_with_tz = ISO8601::to_date_utc(datetime_str)
        .map_err(|_| GoogleMeetError::DateTime(datetime_str.to_string()))?;

    Ok(EventDateTime {
        date_time: Some(datetime_with_tz),
        time_zone: Some(timezone.to_string()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        let result = parse_datetime("2025-09-29T12:45:00.000Z", "Europe/London");
        assert!(result.is_ok());

        let event_datetime = result.unwrap();
        assert_eq!(
            event_datetime.date_time.map(|dt| dt.to_string()),
            Some("2025-09-29 12:45:00 UTC".to_string())
        );
        assert_eq!(event_datetime.time_zone, Some("Europe/London".to_string()));
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = parse_datetime("invalid-date", "UTC");
        assert!(result.is_err());
    }
}
