// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::aws_secret_manager::AwsSecretManager;
use super::env_var_master_secret::EnvVarMasterSecret;
use super::hashicorp_vault::HashiCorpVault;
use super::Vault;
use std::env;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::subscriber::DefaultGuard;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

const SECRET_KEY: &str = "master_secret";
const SECRET_VALUE: &str = "5ec7e75ec7e75ec7e75ec7e75ec7e75ec7e75ec7e75ec7e75ec7e75ec7e75ec7";

static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Default)]
struct LogCapture(Arc<Mutex<Vec<u8>>>);

impl LogCapture {
    fn install(&self) -> DefaultGuard {
        let writer = self.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::set_default(subscriber)
    }

    fn contents(&self) -> String {
        let bytes = self.0.lock().expect("log capture lock").clone();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Write for LogCapture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log capture lock poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct EnvVars(Vec<(&'static str, Option<String>)>);

impl EnvVars {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let previous = vars
            .iter()
            .map(|(name, value)| {
                let previous = env::var(name).ok();
                match value {
                    Some(value) => env::set_var(name, value),
                    None => env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        EnvVars(previous)
    }
}

impl Drop for EnvVars {
    fn drop(&mut self) {
        for (name, previous) in &self.0 {
            match previous {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
    }
}

async fn serve_one_response(listener: TcpListener, body: String) {
    let (mut stream, _) = listener.accept().await.expect("accept");
    let mut request = Vec::new();
    let mut chunk = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut chunk).await.expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
    }
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .expect("write response");
}

#[tokio::test]
async fn env_var_backend_save_secret_omits_value_from_logs() {
    let logs = LogCapture::default();
    let _subscriber = logs.install();

    let result = EnvVarMasterSecret
        .save_secret(SECRET_KEY.to_string(), SECRET_VALUE.to_string())
        .await;

    assert!(result.is_err());
    let output = logs.contents();
    assert!(output.contains("MASTER_SECRET"), "{output}");
    assert!(!output.contains(SECRET_VALUE), "{output}");
}

#[tokio::test]
async fn aws_backend_save_secret_omits_value_from_logs() {
    let _env_lock = ENV_LOCK.lock().await;
    let _vars = EnvVars::set(&[("AWS_REGION", None), ("AWS_SM_KEY_PREFIX", None)]);
    let logs = LogCapture::default();
    let _subscriber = logs.install();

    let result = AwsSecretManager
        .save_secret(SECRET_KEY.to_string(), SECRET_VALUE.to_string())
        .await;

    assert!(result.is_err());
    let output = logs.contents();
    assert!(output.contains("save_secret"), "{output}");
    assert!(!output.contains(SECRET_VALUE), "{output}");
}

#[tokio::test]
async fn hashicorp_backend_read_secret_omits_value_from_logs() {
    let _env_lock = ENV_LOCK.lock().await;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let body = serde_json::json!({
        "auth": null,
        "data": { "data": SECRET_VALUE, "value": null },
        "lease_duration": 0,
        "lease_id": "",
        "renewable": false
    })
    .to_string();
    let server = tokio::spawn(serve_one_response(listener, body));
    let _vars = EnvVars::set(&[
        ("VAULT_SERVER_URL", Some(server_url.as_str())),
        ("VAULT_TOKEN", Some("test-token")),
    ]);
    let logs = LogCapture::default();
    let _subscriber = logs.install();

    let value = HashiCorpVault
        .read_secret(SECRET_KEY.to_string())
        .await
        .expect("read_secret");
    server.await.expect("server task");

    assert_eq!(value.as_deref(), Some(SECRET_VALUE));
    let output = logs.contents();
    assert!(output.contains("read_secret"), "{output}");
    assert!(!output.contains(SECRET_VALUE), "{output}");
}
