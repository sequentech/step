// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Keep generated task implementations valid at the task's module scope.
#![deny(non_local_definitions)]

use celery::prelude::*;
use celery::protocol::Message;
use serde_json::json;

#[celery::task(name = "alc.contract.sync", max_retries = 2, time_limit = 10)]
fn contract_sync(label: String, count: u32) -> TaskResult<String> {
    Ok(format!("{label}:{count}"))
}

#[celery::task(name = "alc.contract.async", max_retries = 0, expires = 30)]
async fn contract_async(value: u32) -> TaskResult<u32> {
    tokio::task::yield_now().await;
    if value == 0 {
        return Err(TaskError::UnexpectedError("synthetic task error".into()));
    }
    Ok(value + 1)
}

#[test]
fn task_metadata_and_serialized_signature_remain_compatible() {
    assert_eq!(contract_sync::NAME, "alc.contract.sync");
    assert_eq!(contract_sync::ARGS, &["label", "count"]);
    assert_eq!(contract_sync::DEFAULTS.max_retries, Some(2));
    assert_eq!(contract_sync::DEFAULTS.time_limit, Some(10));
    let message = Message::try_from(contract_sync::new("synthetic".into(), 7)).unwrap();
    assert_eq!(message.headers.task, contract_sync::NAME);
    let body: serde_json::Value = serde_json::from_slice(&message.raw_body).unwrap();
    assert_eq!(body[0], json!([]));
    assert_eq!(body[1], json!({"label": "synthetic", "count": 7}));
    let (params, _) = message.body::<contract_sync>().unwrap().parts();
    assert_eq!(serde_json::to_value(params).unwrap(), body[1]);
    assert_eq!(
        contract_sync::_run("synthetic".into(), 7).unwrap(),
        "synthetic:7"
    );
}

#[tokio::test]
async fn generated_async_body_preserves_success_and_failure() {
    assert_eq!(contract_async::NAME, "alc.contract.async");
    assert_eq!(contract_async::ARGS, &["value"]);
    assert_eq!(contract_async::DEFAULTS.max_retries, Some(0));
    assert_eq!(
        contract_async::DEFAULTS.expires,
        Some(std::time::Duration::from_secs(30))
    );
    let message = Message::try_from(contract_async::new(4)).unwrap();
    let (params, _) = message.body::<contract_async>().unwrap().parts();
    assert_eq!(serde_json::to_value(params).unwrap(), json!({"value": 4}));
    assert_eq!(contract_async::_run(4).await.unwrap(), 5);
    assert!(
        matches!(contract_async::_run(0).await, Err(TaskError::UnexpectedError(message)) if message == "synthetic task error")
    );
}
