// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rand::Rng;
use rocket::serde::{json::Json, Serialize};
use rocket::{http::Status, response::status::Custom};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::services::user::random_user_by_country;

#[derive(Debug, Serialize)]
pub struct TransactionNewResponse {
    response: TransactionResponseData,
}

#[derive(Debug, Serialize)]
struct TransactionResponseData {
    user_id: String,
    token_dob: String,
}

// We’ll store this in Rocket’s managed state
#[post("/transaction/new")]
pub async fn transaction_new() -> Result<Json<TransactionNewResponse>, Custom<String>> {
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

#[get("/status")]
pub async fn transaction_status_simple() -> Json<serde_json::Value> {
    // Return success immediately
    Json(serde_json::json!({
        "code": 0,
        "response": {
            "idStatus": "verificationOK"
        }
    }))
}

#[get("/results?<country>")]
pub async fn transaction_results(
    country: Option<String>,
) -> Result<Json<serde_json::Value>, Custom<String>> {
    let delay_ms = rand::thread_rng().gen_range(50..501);
    sleep(Duration::from_millis(delay_ms)).await;

    let chosen_country = country.unwrap_or_else(|| "".to_string());

    let user = random_user_by_country(&chosen_country)
        .map_err(|e| Custom(Status::InternalServerError, format!("DB error: {e}")))?;

    let user =
        user.ok_or_else(|| Custom(Status::InternalServerError, "User not found".to_string()))?;

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
                "given_names": user.first_name,
                "middle_name": user.middle_name,
                "surname": user.last_name,
                "personal_number": user.id_card_number,
                "date_of_birth": user.date_of_birth,
            },
            "mrz": {
                "issuing_state_code": "PHL",
                "given_names": user.first_name,
                "surname": user.last_name,
                "personal_number": user.id_card_number,
                "document_number": user.id_card_number,
                "date_of_birth": user.date_of_birth,
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
