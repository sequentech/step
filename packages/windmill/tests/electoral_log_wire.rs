// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use lapin::{
    message::{BasicGetMessage, Delivery},
    BasicProperties,
};
use windmill::{
    services::celery_app::durable_electoral_log_consumer_queues,
    tasks::electoral_log::{decode_electoral_log_delivery, LogMessageType},
};

const INPUT: &str = r#"{"tenant_id":"tenant","election_event_id":"event","message_type":"LOGIN","user_id":"voter","username":null,"body":"null"}"#;

fn delivery(body: &[u8], id: Option<&str>) -> BasicGetMessage {
    let mut properties = BasicProperties::default();
    if let Some(id) = id {
        properties = properties.with_correlation_id(id.into());
    }
    BasicGetMessage {
        message_count: 0,
        delivery: Delivery {
            delivery_tag: 1,
            exchange: "".into(),
            routing_key: "audit".into(),
            redelivered: false,
            properties,
            data: body.to_vec(),
            acker: Default::default(),
        },
    }
}

#[test]
fn current_wire_identity_uses_the_producer_id_and_exact_original_input() {
    let body = format!("[[],{{\"input\":{INPUT}}},{{\"callbacks\":null}}]");
    let mut delivery = delivery(body.as_bytes(), Some("current-id"));
    let first = decode_electoral_log_delivery(&delivery, false).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].payload_hash,
        "187681c2492a9e5dd19a09335f95461c8bed86bfb43574d3d6b345d1859b3dc7"
    );
    assert_eq!(
        first[0].delivery_id,
        "cbeb44da08682b8edf9bb80da04f2794830f526edba206fe85b059c793a3147a"
    );
    assert_eq!(
        first[0].input.message_type,
        LogMessageType::KeycloakEvent("LOGIN".into())
    );
    assert_eq!(first[0].input.user_id.as_deref(), Some("voter"));
    assert!(first[0].input.username.is_none());
    delivery.delivery_tag = 999;
    delivery.redelivered = true;
    let retry = decode_electoral_log_delivery(&delivery, false).unwrap();
    assert_eq!(retry[0].delivery_id, first[0].delivery_id);
    assert_eq!(retry[0].payload_hash, first[0].payload_hash);
    delivery.data = body.replace("voter", "different-voter").into_bytes();
    let reused = decode_electoral_log_delivery(&delivery, false).unwrap();
    assert_eq!(reused[0].delivery_id, first[0].delivery_id);
    assert_ne!(
        reused[0].payload_hash, first[0].payload_hash,
        "reused producer ID must not silently drop changed input"
    );
}

#[test]
fn legacy_batch_keeps_each_event_distinct_even_when_their_inputs_match() {
    let body = format!("[[],{{\"events\":[{INPUT},{INPUT}]}},{{}}]");
    let inputs =
        decode_electoral_log_delivery(&delivery(body.as_bytes(), Some("legacy-id")), true).unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(
        inputs[0].delivery_id,
        "8952fcfe6ebc5cdac63f6476623fa9ccb1bf601611de26597beb98b8e58ad6a4"
    );
    assert_eq!(
        inputs[1].delivery_id,
        "04ff4b3e546c93ddd21f79dca472c7ef186043d2bda99213cdca0395da0d54d7"
    );
    assert_ne!(inputs[0].delivery_id, inputs[1].delivery_id);
    assert_eq!(inputs[0].payload_hash, inputs[1].payload_hash);
    let empty = decode_electoral_log_delivery(
        &delivery(br#"[[],{"events":[]},{}]"#, Some("empty-batch")),
        true,
    )
    .unwrap();
    assert!(empty.is_empty(), "legacy no-op batches remain compatible");
}

#[test]
fn missing_stable_identity_or_invalid_celery_envelopes_are_rejected() {
    let valid = format!("[[],{{\"input\":{INPUT}}},{{}}]");
    assert_eq!(
        decode_electoral_log_delivery(&delivery(valid.as_bytes(), Some("id")), false)
            .unwrap()
            .len(),
        1
    );
    for id in [None, Some("")] {
        let error =
            decode_electoral_log_delivery(&delivery(valid.as_bytes(), id), false).unwrap_err();
        assert_eq!(
            error.to_string(),
            "electoral log delivery has no stable correlation ID"
        );
    }
    for (body, legacy, expected) in [
        ("not-json", false, "Error parsing Celery message as JSON"),
        (
            "{}",
            false,
            "Invalid Celery message: expected arguments array",
        ),
        (
            "[]",
            false,
            "Invalid Celery message: expected arguments array",
        ),
        ("[[],{},{}]", false, "Missing input in electoral log event"),
        (
            "[[],{},{}]",
            true,
            "Missing events in legacy electoral log batch",
        ),
    ] {
        let error = decode_electoral_log_delivery(&delivery(body.as_bytes(), Some("id")), legacy)
            .unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn existing_audit_queue_arguments_select_the_durable_dispatcher_only() {
    assert_eq!(
        durable_electoral_log_consumer_queues(
            vec![
                "test_short_queue".into(),
                "test_electoral_log_batch_queue".into(),
                "test_electoral_log_beat_queue".into(),
                "test_electoral_log_event_queue".into(),
                "another_electoral_log_batch_queue".into(),
            ],
            "test"
        ),
        [
            "test_short_queue",
            "test_electoral_log_beat_queue",
            "another_electoral_log_batch_queue"
        ]
    );
}
