// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(all(
    feature = "default_features",
    feature = "areas",
    feature = "reports",
    feature = "probe",
    feature = "signatures",
    feature = "log"
))]

use sequent_core::services::area_tree::{TreeNode, TreeNodeArea};
use serde_json::json;
use std::time::Duration;

fn area(id: &str, parent: Option<&str>) -> TreeNodeArea {
    TreeNodeArea {
        id: id.into(),
        tenant_id: "tenant".into(),
        election_event_id: "event".into(),
        annotations: None,
        parent_id: parent.map(str::to_owned),
    }
}

#[test]
fn hierarchy_keeps_descendants_and_rejects_missing_parents() {
    let tree = TreeNode::<()>::from_areas(vec![
        area("child", Some("root")),
        area("root", None),
    ])
    .unwrap();
    let ids: Vec<_> =
        tree.get_all_children().into_iter().map(|a| a.id).collect();
    assert_eq!(ids, ["root", "child"]);
    assert_eq!(
        TreeNode::<()>::from_areas(vec![area("child", Some("missing"))])
            .unwrap_err()
            .to_string(),
        "Parent id missing not found in the tree structure"
    );
}

#[test]
fn hierarchy_rejects_cycles_even_when_an_independent_root_exists() {
    for nodes in [
        vec![area("self", Some("self"))],
        vec![
            area("root", None),
            area("a", Some("b")),
            area("b", Some("a")),
        ],
    ] {
        assert!(TreeNode::<()>::from_areas(nodes).is_err(),"a cyclic component must not silently disappear from the area export");
    }
}

#[tokio::test]
async fn retry_returns_the_success_value_or_last_error_with_a_bounded_attempt_count(
) {
    use sequent_core::util::retry::retry_with_exponential_backoff as retry;
    let mut attempts = 0;
    let value = retry(
        || {
            attempts += 1;
            std::future::ready(if attempts == 3 {
                Ok(17)
            } else {
                Err(attempts)
            })
        },
        2,
        Duration::ZERO,
    )
    .await
    .unwrap();
    assert_eq!((value, attempts), (17, 3));
    attempts = 0;
    assert_eq!(
        retry(
            || {
                attempts += 1;
                std::future::ready(Err::<(), _>(attempts))
            },
            2,
            Duration::ZERO
        )
        .await,
        Err(3)
    );
    assert_eq!(attempts, 3);
    attempts = 0;
    assert_eq!(
        retry(
            || {
                attempts += 1;
                std::future::ready(Err::<(), _>("terminal"))
            },
            0,
            Duration::ZERO
        )
        .await,
        Err("terminal")
    );
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn health_probe_tracks_live_and_ready_independently() {
    use sequent_core::services::probe::ProbeHandler;
    let reservation = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let probe = ProbeHandler::new("live", "ready", address);
    struct Stop(tokio::task::JoinHandle<()>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _server = Stop(tokio::spawn(probe.future()));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    async fn response(
        client: &reqwest::Client,
        address: std::net::SocketAddr,
        path: &str,
    ) -> (u16, String) {
        let response = client
            .get(format!("http://{address}/{path}"))
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }
    assert_eq!(
        response(&client, address, "live").await,
        (400, "Not live".into())
    );
    assert_eq!(
        response(&client, address, "ready").await,
        (400, "Not ready".into())
    );
    probe.set_live(|| Box::pin(async { true })).await;
    assert_eq!(
        response(&client, address, "live").await,
        (200, "Live".into())
    );
    assert_eq!(
        response(&client, address, "ready").await,
        (400, "Not ready".into())
    );
    probe.set_ready(|| Box::pin(async { true })).await;
    assert_eq!(
        response(&client, address, "ready").await,
        (200, "Ready".into())
    );
    probe.set_live(|| Box::pin(async { false })).await;
    assert_eq!(
        response(&client, address, "live").await,
        (400, "Not live".into())
    );
    assert_eq!(response(&client, address, "missing").await.0, 404);
}

#[test]
fn templates_escape_input_and_fail_on_missing_or_malformed_templates() {
    use sequent_core::services::reports::{
        render_template, render_template_text,
    };
    let values = json!({"name":"<Admin>"}).as_object().unwrap().clone();
    assert_eq!(
        render_template_text("Hello {{name}}", values.clone()).unwrap(),
        "Hello &lt;Admin&gt;"
    );
    let templates = [("main".into(), "<b>{{name}}</b>".into())]
        .into_iter()
        .collect();
    assert_eq!(
        render_template("main", templates, values.clone()).unwrap(),
        "<b>&lt;Admin&gt;</b>"
    );
    assert!(
        render_template("missing", Default::default(), values.clone()).is_err()
    );
    assert!(render_template_text("{{#if}}", values).is_err());
}

#[test]
fn temporary_report_files_preserve_bytes_and_are_removed_on_drop() {
    use sequent_core::temp_path;
    let bytes = vec![0, 255, 13, 10, 65];
    let (path, name, size) =
        temp_path::write_into_named_temp_file(&bytes, "report-", ".bin")
            .unwrap();
    assert_eq!(size, 5);
    assert_eq!(
        temp_path::read_temp_path(&path).unwrap(),
        [0, 255, 13, 10, 65]
    );
    assert_eq!(temp_path::get_file_size(&name).unwrap(), 5);
    drop(path);
    assert!(!std::path::Path::new(&name).exists());
    assert!(temp_path::get_file_size(&name).is_err());
    let mut file = temp_path::generate_temp_file("empty-", ".dat").unwrap();
    assert_eq!(
        temp_path::read_temp_file(&mut file).unwrap(),
        Vec::<u8>::new()
    );
}

#[test]
fn shell_adapter_preserves_output_and_rejects_failed_commands() {
    use sequent_core::signatures::shell::run_shell_command;
    assert_eq!(
        run_shell_command("printf 'synthetic\\n'").unwrap(),
        "synthetic\n"
    );
    assert!(run_shell_command("exit 7")
        .unwrap_err()
        .to_string()
        .contains("Shell command failed"));
    assert!(
        sequent_core::signatures::ecies_encrypt::ecies_sign_data_bulk(
            &sequent_core::signatures::ecies_encrypt::EciesKeyPair {
                private_key_pem: "synthetic".into(),
                public_key_pem: "synthetic".into()
            },
            &[]
        )
        .unwrap()
        .is_empty()
    );
}
