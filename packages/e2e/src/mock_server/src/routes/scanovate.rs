// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Mock of the Scanovate B-Trust identity verification API (v3.8.2), used
//! by the `scanovate-authenticator` Keycloak extension in development and
//! load tests.

use chrono::{Months, NaiveDate, Utc};
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::status::Custom;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

use crate::services::user::random_user_by_country;
use crate::types::user::User;

const MOCK_SERVER_PUBLIC_URL_ENV: &str = "MOCK_SERVER_PUBLIC_URL";
const MOCK_SERVER_PORT_ENV: &str = "MOCK_SERVER_PORT";
const DEFAULT_MOCK_SERVER_PORT: &str = "8500";
const COUNTRY_PARAM: &str = "country";
const OUTCOME_PARAM: &str = "mock_outcome";
const MOCK_DOB: &str = "01/01/1990";
const INPUT_DATE_FORMAT: &str = "%d/%m/%Y";
const SCANOVATE_DATE_FORMAT: &str = "%d.%m.%Y";
const SCANOVATE_ISO_DATE_FORMAT: &str = "%Y-%m-%dT00:00:00.000+0000";

/// Result the mocked flow produces, selectable through the `mock_outcome`
/// flow parameter or from the mocked flow page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockOutcome {
    #[default]
    Success,
    LowBiometricScore,
    LivenessFailed,
    DocumentAuthenticationFailed,
    MaxTrials,
}

const ALL_OUTCOMES: [MockOutcome; 5] = [
    MockOutcome::Success,
    MockOutcome::LowBiometricScore,
    MockOutcome::LivenessFailed,
    MockOutcome::DocumentAuthenticationFailed,
    MockOutcome::MaxTrials,
];

impl fmt::Display for MockOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            MockOutcome::Success => "success",
            MockOutcome::LowBiometricScore => "low_biometric_score",
            MockOutcome::LivenessFailed => "liveness_failed",
            MockOutcome::DocumentAuthenticationFailed => "document_authentication_failed",
            MockOutcome::MaxTrials => "max_trials",
        };
        write!(f, "{value}")
    }
}

impl FromStr for MockOutcome {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ALL_OUTCOMES
            .into_iter()
            .find(|outcome| outcome.to_string() == value)
            .ok_or_else(|| format!("unknown mock outcome {value}"))
    }
}

#[derive(Debug, Clone)]
struct MockSession {
    flow_id: i64,
    id_number: Option<String>,
    redirect_url: Option<String>,
    country: Option<String>,
    outcome: MockOutcome,
}

/// Sessions created through `/flow/v3/link`, indexed by process id.
#[derive(Default)]
pub struct MockSessions(Mutex<HashMap<String, MockSession>>);

impl MockSessions {
    fn get(&self, process_id: &str) -> Option<MockSession> {
        self.0.lock().ok()?.get(process_id).cloned()
    }

    fn insert(&self, process_id: String, session: MockSession) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.insert(process_id, session);
        }
    }

    fn set_outcome(&self, process_id: &str, outcome: MockOutcome) -> bool {
        match self.0.lock() {
            Ok(mut sessions) => match sessions.get_mut(process_id) {
                Some(session) => {
                    session.outcome = outcome;
                    true
                }
                None => false,
            },
            Err(_) => false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    flow_id: i64,
    id_number: Option<String>,
    identifier_id: Option<String>,
    redirect_url: Option<String>,
    params: Option<HashMap<String, String>>,
}

fn public_url() -> String {
    std::env::var(MOCK_SERVER_PUBLIC_URL_ENV)
        .ok()
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| {
            let port = std::env::var(MOCK_SERVER_PORT_ENV)
                .unwrap_or_else(|_| DEFAULT_MOCK_SERVER_PORT.to_string());
            format!("http://127.0.0.1:{port}")
        })
}

fn not_found(process_id: &str) -> Custom<Json<Value>> {
    Custom(
        Status::NotFound,
        Json(json!({ "detail": format!("No inquiry with sessionId {process_id}") })),
    )
}

#[post("/auth/token")]
pub async fn auth_token() -> Json<Value> {
    Json(json!({
        "access_token": Uuid::new_v4().to_string(),
        "token_type": "Bearer",
        "expires_in": 86400,
        "default_url": ""
    }))
}

#[post("/flow/v3/link", data = "<request>")]
pub async fn flow_link(request: Json<LinkRequest>, sessions: &State<MockSessions>) -> Json<Value> {
    let request = request.into_inner();
    let process_id = request
        .identifier_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let params = request.params.unwrap_or_default();
    let outcome = params
        .get(OUTCOME_PARAM)
        .and_then(|value| value.parse().ok())
        .unwrap_or_default();

    sessions.insert(
        process_id.clone(),
        MockSession {
            flow_id: request.flow_id,
            id_number: request.id_number,
            redirect_url: request.redirect_url,
            country: params.get(COUNTRY_PARAM).cloned(),
            outcome,
        },
    );

    Json(json!({
        "success": true,
        "errorCode": 0,
        "data": format!(
            "{}/flow?urlId={}&cid=mock&process_id={process_id}",
            public_url(),
            Uuid::new_v4()
        )
    }))
}

/// Page standing in for the B-Trust flow UI: lets the tester pick the
/// outcome and then returns to the `redirect_url`.
#[get("/flow?<process_id>")]
pub async fn flow_page(
    process_id: String,
    sessions: &State<MockSessions>,
) -> Result<RawHtml<String>, Custom<Json<Value>>> {
    let session = sessions
        .get(&process_id)
        .ok_or_else(|| not_found(&process_id))?;
    let links: String = ALL_OUTCOMES
        .iter()
        .map(|outcome| {
            let checked = if *outcome == session.outcome { " (default)" } else { "" };
            format!(
                "<li><a href=\"/flow/complete?process_id={process_id}&outcome={outcome}\">{outcome}</a>{checked}</li>"
            )
        })
        .collect();
    Ok(RawHtml(format!(
        "<!DOCTYPE html><html><head><title>B-Trust mock</title></head><body>\
         <h1>B-Trust mock flow {}</h1><p>Process id: {process_id}</p>\
         <p>Choose the verification outcome:</p><ul>{links}</ul></body></html>",
        session.flow_id
    )))
}

#[get("/flow/complete?<process_id>&<outcome>")]
pub async fn flow_complete(
    process_id: String,
    outcome: String,
    sessions: &State<MockSessions>,
) -> Result<Redirect, Custom<Json<Value>>> {
    let outcome: MockOutcome = outcome
        .parse()
        .map_err(|err: String| Custom(Status::BadRequest, Json(json!({ "detail": err }))))?;
    if !sessions.set_outcome(&process_id, outcome) {
        return Err(not_found(&process_id));
    }
    let session = sessions
        .get(&process_id)
        .ok_or_else(|| not_found(&process_id))?;
    let redirect_url = session.redirect_url.ok_or_else(|| {
        Custom(
            Status::BadRequest,
            Json(json!({ "detail": "session has no redirect_url" })),
        )
    })?;
    let separator = if redirect_url.contains('?') { '&' } else { '?' };
    Ok(Redirect::to(format!(
        "{redirect_url}{separator}processId={process_id}&token={process_id}"
    )))
}

#[get("/api/v3/mobile_interaction/<session>/token")]
pub async fn session_token(
    session: String,
    sessions: &State<MockSessions>,
) -> Result<Json<Value>, Custom<Json<Value>>> {
    sessions.get(&session).ok_or_else(|| not_found(&session))?;
    Ok(Json(json!({ "token": session, "processId": session })))
}

#[get("/api/v3/mobile_interaction/v2/<session_token>/results_with_image_names")]
pub async fn results_with_image_names(
    session_token: String,
    sessions: &State<MockSessions>,
) -> Result<Json<Value>, Custom<Json<Value>>> {
    let session = sessions
        .get(&session_token)
        .ok_or_else(|| not_found(&session_token))?;

    let user = match session.country.as_deref() {
        Some(country) => random_user_by_country(country).map_err(|err| {
            Custom(
                Status::InternalServerError,
                Json(json!({ "detail": format!("DB error: {err}") })),
            )
        })?,
        None => None,
    };

    Ok(Json(mock_results(&session_token, &session, user)))
}

fn scanovate_dates(input: &str) -> (String, String) {
    match NaiveDate::parse_from_str(input, INPUT_DATE_FORMAT) {
        Ok(date) => (
            date.format(SCANOVATE_DATE_FORMAT).to_string(),
            date.format(SCANOVATE_ISO_DATE_FORMAT).to_string(),
        ),
        Err(_) => (input.to_string(), input.to_string()),
    }
}

fn mock_results(process_id: &str, session: &MockSession, user: Option<User>) -> Value {
    let (first_name, last_name, id_number, dob) = match user {
        Some(user) => (
            user.first_name,
            user.last_name,
            user.id_card_number,
            user.date_of_birth,
        ),
        None => (
            "JUAN".to_string(),
            "DELA CRUZ".to_string(),
            session
                .id_number
                .clone()
                .unwrap_or_else(|| "P1234567A".to_string()),
            MOCK_DOB.to_string(),
        ),
    };
    let (dob, date_of_birth) = scanovate_dates(&dob);
    let today = Utc::now().date_naive();
    let issue = today.checked_sub_months(Months::new(12)).unwrap_or(today);
    let expiry = today.checked_add_months(Months::new(48)).unwrap_or(today);

    let ocr = json!({
        "process": "ocr",
        "success": session.outcome != MockOutcome::DocumentAuthenticationFailed,
        "count": 1,
        "sessionId": process_id,
        "serviceSessionId": Uuid::new_v4().to_string(),
        "docType": "MRZ",
        "ocrType": "MRZ",
        "standardDocType": "MRZ",
        "documentType": "P",
        "mrzType": "TD3",
        "firstName": first_name,
        "lastName": last_name,
        "idNumber": id_number,
        "documentNumber": id_number,
        "gender": "M",
        "countryCode": "PH",
        "nationality": { "name": "Philippines", "alpha2": "PH", "alpha3": "PHL" },
        "issuingCountry": { "name": "Philippines", "alpha2": "PH", "alpha3": "PHL" },
        "dateOfBirth": date_of_birth,
        "dob": dob,
        "issueDate": issue.format(SCANOVATE_ISO_DATE_FORMAT).to_string(),
        "issuingDate": issue.format(SCANOVATE_DATE_FORMAT).to_string(),
        "expirationDate": expiry.format(SCANOVATE_ISO_DATE_FORMAT).to_string(),
        "expiryDate": expiry.format(SCANOVATE_DATE_FORMAT).to_string(),
        "authentication": {
            "verify.facePosition": true,
            "verify.faceSize": true,
            "verify.expiryDate": true,
            "verify.mrzChecksum": true,
            "verify.mrzReadSuccess": true,
            "verify.documentInFrame": true,
            "verify.templateMatching": true
        },
        "faceImage": format!("{process_id}/ocr/face_image.jpg"),
        "frontImage": format!("{process_id}/ocr/front_image.jpg"),
        "cardImage": format!("{process_id}/ocr/card_image.jpg"),
        "scanVideo": format!("{process_id}/ocr/scan_video.webm"),
        "scanDuration": 12000
    });
    let liveness_passed = session.outcome != MockOutcome::LivenessFailed;
    let liveness = json!({
        "process": "liveness_plus",
        "success": liveness_passed,
        "sessionId": process_id,
        "serviceSessionId": Uuid::new_v4().to_string(),
        "count": 1,
        "duration": 9000,
        "livenessCheck": liveness_passed,
        "presentationAttackDetection": liveness_passed,
        "injectionAttackDetection": true,
        "threshold": 0.56,
        "score": if liveness_passed { 0.97 } else { 0.01 },
        "faceImage": format!("{process_id}/liveness/face_image.jpg")
    });
    let biometric = json!({
        "process": "biometric_match",
        "success": true,
        "score": if session.outcome == MockOutcome::LowBiometricScore { 0.21 } else { 0.86 }
    });
    let document_liveness = json!({
        "process": "document_liveness_plus",
        "success": true,
        "status": "passed",
        "results": [],
        "missingFields": []
    });

    let (success, error_code, error_message, results_list) = match session.outcome {
        MockOutcome::Success | MockOutcome::LowBiometricScore => (
            true,
            0,
            "",
            vec![ocr, liveness, biometric, document_liveness],
        ),
        MockOutcome::LivenessFailed => (false, -1, "", vec![ocr, liveness]),
        MockOutcome::DocumentAuthenticationFailed => {
            (false, 1026, "Document authentication failed", vec![ocr])
        }
        MockOutcome::MaxTrials => (false, 1030, "Maximum trials reached", vec![ocr]),
    };

    json!({
        "success": true,
        "errorCode": 0,
        "data": {
            "success": success,
            "errorMessage": error_message,
            "errorCode": error_code,
            "metadata": "",
            "case_id": 1,
            "flow_id": session.flow_id,
            "flow_name": "B-Trust mock flow",
            "resultsList": results_list,
            "externalParams": { "map": {} }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(outcome: MockOutcome) -> MockSession {
        MockSession {
            flow_id: 7,
            id_number: Some("ID-1".to_string()),
            redirect_url: None,
            country: None,
            outcome,
        }
    }

    #[test]
    fn outcome_round_trips_through_strings() {
        for outcome in ALL_OUTCOMES {
            assert_eq!(outcome.to_string().parse::<MockOutcome>(), Ok(outcome));
        }
        assert!("nope".parse::<MockOutcome>().is_err());
    }

    #[test]
    fn successful_results_echo_the_id_number() {
        let results = mock_results("proc", &session(MockOutcome::Success), None);
        assert_eq!(results["data"]["success"], true);
        assert_eq!(results["data"]["errorCode"], 0);
        assert_eq!(results["data"]["resultsList"][0]["idNumber"], "ID-1");
        assert_eq!(results["data"]["resultsList"][0]["dob"], "01.01.1990");
        assert_eq!(
            results["data"]["resultsList"][0]["dateOfBirth"],
            "1990-01-01T00:00:00.000+0000"
        );
    }

    #[test]
    fn user_data_is_used_when_available() {
        let user = User {
            id: "1".to_string(),
            first_name: "MARIA".to_string(),
            last_name: "SANTOS".to_string(),
            middle_name: "".to_string(),
            country: "Spain".to_string(),
            id_card_number: "X9".to_string(),
            id_card_type: "philippinePassport".to_string(),
            date_of_birth: "03/07/2004".to_string(),
            embassy: "".to_string(),
        };
        let results = mock_results("proc", &session(MockOutcome::Success), Some(user));
        assert_eq!(results["data"]["resultsList"][0]["firstName"], "MARIA");
        assert_eq!(results["data"]["resultsList"][0]["idNumber"], "X9");
        assert_eq!(results["data"]["resultsList"][0]["dob"], "03.07.2004");
    }

    #[test]
    fn failure_outcomes_use_btrust_error_codes() {
        let cases = [
            (MockOutcome::LivenessFailed, -1),
            (MockOutcome::DocumentAuthenticationFailed, 1026),
            (MockOutcome::MaxTrials, 1030),
        ];
        for (outcome, code) in cases {
            let results = mock_results("proc", &session(outcome), None);
            assert_eq!(results["success"], true);
            assert_eq!(results["data"]["success"], false);
            assert_eq!(results["data"]["errorCode"], code);
        }
    }

    #[test]
    fn low_biometric_score_still_completes_the_flow() {
        let results = mock_results("proc", &session(MockOutcome::LowBiometricScore), None);
        assert_eq!(results["data"]["success"], true);
        assert_eq!(results["data"]["resultsList"][2]["score"], 0.21);
    }

    #[test]
    fn unparseable_dates_are_passed_through() {
        assert_eq!(
            scanovate_dates("1990-01-01"),
            ("1990-01-01".to_string(), "1990-01-01".to_string())
        );
    }
}
