use rand::Rng;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{http::Status, response::status::Custom, State};
use tokio::time::sleep;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::services::user::random_user_by_country;

#[derive(Debug, Deserialize)]
struct TransactionNewRequest {
    doc_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct TransactionNewResponse {
    response: TransactionResponseData,
}

#[derive(Debug, Serialize)]
struct TransactionResponseData {
    user_id: String,
    token_dob: String,
}

// This will be our "in-memory" store to keep track of userID -> tokenDob or docID -> user data
#[derive(Default)]
pub struct MockInetumState {
    // userId -> tokenDob
    pub user_tokens: HashMap<String, String>,
    // userId -> docID (so we can link them to further results if we want)
    pub user_docs: HashMap<String, String>,
    // userId -> "verificationOK" or "verificationKO"
    pub user_status: HashMap<String, String>,
}

// We’ll store this in Rocket’s managed state
#[post("/transaction/new")]
pub async fn transaction_new(
) -> Result<Json<TransactionNewResponse>, Custom<String>> {
    let user_id = Uuid::new_v4().to_string();
    let token_dob = Uuid::new_v4().to_string();

    let resp = TransactionNewResponse {
        response: TransactionResponseData {
            user_id: user_id,
            token_dob: token_dob,
        },
    };

    Ok(Json(resp))
}

#[get("/transaction/<user_id>/status?<t>")]
pub async fn transaction_status_simple(user_id: String, t: String) -> Json<serde_json::Value> {
    // Return success immediately
    Json(serde_json::json!({
        "code": 0,
        "response": {
            "idStatus": "verificationOK"
        }
    }))
}

#[get("/transaction/results?<country>")]
pub async fn transaction_results(                 
    country: Option<String>,         
) -> Result<Json<serde_json::Value>, Custom<String>> {

    let delay_ms = rand::thread_rng().gen_range(50..501);
    sleep(Duration::from_millis(delay_ms)).await;

    let chosen_country = country.unwrap_or_else(|| "Philippines".to_string());

    let user = random_user_by_country(&chosen_country)
        .map_err(|e| Custom(Status::InternalServerError, format!("DB error: {e}")))?;


    // 5) Convert from "YYYY-MM-DD" to "dd/MM/yyyy"
    let parsed = chrono::NaiveDate::parse_from_str(user, "%Y-%m-%d");
    let inetum_dob = match parsed {
        Ok(date) => date.format("%d/%m/%Y").to_string(),
        Err(_) => "12/03/1980".to_string(),
    };


    let response_json = serde_json::json!({
        "code": 0,
        "response": {
            "docVerification": {
                "documentIdentification": [
                    { "type": "Identity Card" }
                ]
            },
            "ocr": {
                "issuing_state_code": "PHL",
                "given_names": dbuser.first_name,
                "middle_name": dbuser.middle_name,
                "surname": dbuser.last_name,
                "personal_number": format!("{}-OCR-123", &dbuser.country[..2].to_uppercase()),
                "date_of_birth": inetum_dob
            },
            "mrz": {
                "issuing_state_code": "PHL",
                "given_names": dbuser.first_name,
                "surname": dbuser.last_name,
                "personal_number": format!("{}-MRZ-456", &dbuser.country[..2].to_uppercase()),
                "document_number": format!("DOC{}", rand::thread_rng().gen_range(1000..9999)),
                "date_of_birth": inetum_dob
            },
            "resultData": {
                "scoreDocumental": 75,
                "scoreFacial": 80,
                "scoreValCamposCriticos": 60
            },
            "idStatus": "verificationOK"
        }
    });

    Ok(Json(response_json))
}