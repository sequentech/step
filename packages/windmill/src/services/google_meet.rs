// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use google_calendar3::{
    api::{ConferenceData, ConferenceSolutionKey, CreateConferenceRequest, Event, EventDateTime},
    hyper, hyper_rustls, hyper_util,
    yup_oauth2::authenticator::Authenticator,
    yup_oauth2::ServiceAccountAuthenticator,
    yup_oauth2::ServiceAccountKey,
    CalendarHub,
};

use sequent_core::services::date::ISO8601;
use serde::{Deserialize, Serialize};
use std::env;
use strum_macros::{Display, EnumString};
use tracing::{error, info, instrument};

#[derive(Deserialize, Debug, Clone)]
pub struct GenerateGoogleMeetBody {
    pub summary: String,
    pub description: String,
    pub start_date_time: String,
    pub end_date_time: String,
    pub time_zone: String,
    pub attendee_email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateGoogleMeetResponse {
    pub meet_link: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
pub enum GoogleMeetError {
    EnvVar(String),
    Json(String),
    OAuth2(String),
    GoogleApi(String),
    Http(String),
    DateTime(String),
    CalendarNotFound,
    MeetLinkNotFound,
    Other(String),
}

/// Implementation function for generating Google Meet links
/// Creates a calendar event with Google Meet integration using service account credentials
#[instrument(skip(meeting_data))]
pub async fn generate_google_meet_link_impl(
    meeting_data: &GenerateGoogleMeetBody,
) -> Result<String, GoogleMeetError> {
    info!("Starting Google Meet link generation for: {meeting_data:?}");

    // 1. Get service account credentials from environment
    let service_account_key_json = env::var("GOOGLE_CALENDAR_API_KEY")
        .map_err(|_| GoogleMeetError::EnvVar("GOOGLE_CALENDAR_API_KEY".to_string()))?;
    let calendar_id = env::var("GOOGLE_CALENDAR_CLIENT_ID")
        .map_err(|_| GoogleMeetError::EnvVar("GOOGLE_CALENDAR_CLIENT_ID".to_string()))?;

    // 2. Parse service account key
    let service_account_key: ServiceAccountKey = serde_json::from_str(&service_account_key_json)
        .map_err(|_| GoogleMeetError::Json("Failed to parse service account key".to_string()))?;

    // 3. Create authenticator
    let auth = ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .map_err(|_| GoogleMeetError::OAuth2("Failed to create authenticator".to_string()))?;

    // 4. Create HTTP client and Calendar hub
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|_| GoogleMeetError::Http("Failed to create HTTP client".to_string()))?;
    let connector = connector.https_or_http().enable_http1().build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        );
    let mut hub = CalendarHub::new(client, auth);

    // 5. Parse start and end date times
    let start_datetime = parse_datetime(&meeting_data.start_date_time, &meeting_data.time_zone)?;
    let end_datetime = parse_datetime(&meeting_data.end_date_time, &meeting_data.time_zone)?;

    // 6. Create conference data for Google Meet
    let conference_data = ConferenceData {
        create_request: Some(CreateConferenceRequest {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            conference_solution_key: Some(ConferenceSolutionKey {
                type_: Some("hangoutsMeet".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    // 7. Create calendar event
    let event = Event {
        summary: Some(meeting_data.summary.clone()),
        description: Some(meeting_data.description.clone()),
        start: Some(start_datetime),
        end: Some(end_datetime),
        conference_data: Some(conference_data),
        attendees: Some(vec![google_calendar3::api::EventAttendee {
            email: Some(meeting_data.attendee_email.clone()),
            ..Default::default()
        }]),
        ..Default::default()
    };

    // 8. Insert the event into the calendar
    info!("Creating calendar event with Google Meet link");
    let result = hub
        .events()
        .insert(event, &calendar_id)
        .conference_data_version(1) // Required for conference data
        .send_updates("all") // Send invitations to attendees
        .doit()
        .await;

    match result {
        Ok((_, created_event)) => {
            info!("Calendar event created successfully");

            // 9. Extract Meet link from the response
            if let Some(conference_data) = created_event.conference_data {
                if let Some(entry_points) = conference_data.entry_points {
                    for entry_point in entry_points {
                        if entry_point.entry_point_type == Some("video".to_string()) {
                            if let Some(uri) = entry_point.uri {
                                info!("Google Meet link generated successfully: {}", uri);
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
            error!("Failed to create calendar event: {:?}", e);
            Err(GoogleMeetError::GoogleApi(e.to_string()))
        }
    }
}

/// Parse datetime string with timezone into EventDateTime
fn parse_datetime(datetime_str: &str, timezone: &str) -> Result<EventDateTime, GoogleMeetError> {
    // The datetime should be in ISO 8601 format (e.g., "2023-12-25T10:00:00")
    // We'll combine it with the timezone
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
        let result = parse_datetime("2023-12-25T10:00:00", "America/New_York");
        assert!(result.is_ok());

        let event_datetime = result.unwrap();
        assert_eq!(
            event_datetime.date_time,
            Some("2023-12-25T10:00:00:00".to_string())
        );
        assert_eq!(
            event_datetime.time_zone,
            Some("America/New_York".to_string())
        );
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = parse_datetime("invalid-date", "UTC");
        assert!(result.is_err());
    }
}
