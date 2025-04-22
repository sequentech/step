// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sms_sender::{SmsConfig, SmsSender};
use warp::Filter;

async fn handle_send_sms(sms_config: SmsConfig) -> Result<impl warp::Reply, warp::Rejection> {
    let sender = SmsSender::new();
    match sender.send_sms(sms_config).await {
        Ok(response) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "response": response
        }))),
        Err(e) => {
            eprintln!("Error sending SMS: {}", e);
            Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "message": e.to_string()
            })))
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let send_sms_route = warp::post()
        .and(warp::path("send_sms"))
        .and(warp::body::json())
        .and_then(handle_send_sms);

    //TODO: replace with actual values
    println!("Server running on http://127.0.0.1:4000");
    warp::serve(send_sms_route)
        .run(([127, 0, 0, 1], 4000))
        .await;
}
