// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Asset-bearing templates use a fresh, offline Chromium process. Only exact
//! attached files are fulfilled; no user path is ever resolved on the host.
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use headless_chrome::{
    browser::tab::RequestPausedDecision,
    protocol::cdp::{types::Event, Emulation, Fetch, Network, Page, Runtime},
    types::PrintToPdfOptions,
    Browser, LaunchOptionsBuilder,
};
use sequent_template_renderer::assets::{self, TemplateAssets};
use serde_json::json;
use std::{
    ffi::OsStr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

pub fn render(
    html: &str,
    files: &TemplateAssets,
    options: Option<PrintToPdfOptions>,
) -> Result<Vec<u8>> {
    if html.len() > 8_000_000 {
        bail!("Rendered HTML exceeds 8 MB");
    }
    let files = assets::decode(files).map_err(|e| anyhow!(e))?;
    let options = options.unwrap_or_else(|| PrintToPdfOptions {
        print_background: Some(true),
        prefer_css_page_size: Some(true),
        ..Default::default()
    });
    for (name, value, min, max) in [
        ("scale", options.scale, 0.1, 2.0),
        ("paper width", options.paper_width, 0.25, 100.0),
        ("paper height", options.paper_height, 0.25, 100.0),
        ("top margin", options.margin_top, 0.0, 10.0),
        ("bottom margin", options.margin_bottom, 0.0, 10.0),
        ("left margin", options.margin_left, 0.0, 10.0),
        ("right margin", options.margin_right, 0.0, 10.0),
    ] {
        if value.is_some_and(|value| {
            !value.is_finite() || value < min || value > max
        }) {
            bail!("Invalid PDF {name}: expected {min}–{max}");
        }
    }
    if options
        .header_template
        .as_ref()
        .is_some_and(|text| text.len() > 300_000)
        || options
            .footer_template
            .as_ref()
            .is_some_and(|text| text.len() > 300_000)
    {
        bail!("PDF header or footer exceeds 300 KB");
    }

    let launch = LaunchOptionsBuilder::default()
        .headless(true)
        // Existing report deployments run Chromium in a container as root.
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(20))
        .args(vec![
            OsStr::new("--disable-background-networking"),
            OsStr::new("--disable-quic"),
            OsStr::new("--js-flags=--max-old-space-size=128"),
            OsStr::new("--host-resolver-rules=MAP * ~NOTFOUND"),
            OsStr::new("--proxy-server=http://127.0.0.1:9"),
            OsStr::new("--proxy-bypass-list=<-loopback>"),
            OsStr::new(
                "--force-webrtc-ip-handling-policy=disable_non_proxied_udp",
            ),
        ])
        .build()?;
    let browser = Browser::new(launch)?;
    // A deadline outside the renderer also stops an infinite template script.
    // This renderer runs on the same Unix hosts as the existing PDF service.
    let pid = browser
        .get_process_id()
        .context("Missing renderer process")?;
    let (done, deadline) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        if matches!(
            deadline.recv_timeout(Duration::from_secs(20)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    });
    let result = (|| {
        let context = browser.new_context()?;
        let tab = context.new_tab()?;
        tab.set_default_timeout(Duration::from_secs(12));
        let script_failed = Arc::new(AtomicBool::new(false));
        let failure = Arc::clone(&script_failed);
        tab.add_event_listener(Arc::new(move |event: &Event| {
            if matches!(event, Event::RuntimeExceptionThrown(_)) {
                failure.store(true, Ordering::SeqCst);
            }
        }))?;
        tab.call_method(Runtime::Enable(None))?;
        let frame = tab
            .call_method(Page::GetFrameTree(None))?
            .frame_tree
            .frame
            .id;
        tab.call_method(Network::SetBypassServiceWorker { bypass: true })?;
        tab.call_method(serde_json::from_value::<Network::EmulateNetworkConditions>(json!({
            "offline": true, "latency": 0, "downloadThroughput": -1, "uploadThroughput": -1,
            "packetLoss": 100
        }))?)?;
        let html = html.as_bytes().to_vec();
        let loaded = AtomicBool::new(false);
        let requests = AtomicUsize::new(0);
        tab.enable_request_interception(Arc::new(
            move |_transport,
                  _session,
                  event: Fetch::events::RequestPausedEvent| {
                let request = &event.params;
                let is_document = serde_json::to_value(&request.resource_Type)
                    .ok()
                    == Some(json!("Document"));
                let content = if requests.fetch_add(1, Ordering::Relaxed) >= 500
                    || request.request.method != "GET"
                    || request.frame_id != frame
                {
                    None
                } else if is_document {
                    if request.request.url == assets::DOCUMENT_URL
                        && !loaded.swap(true, Ordering::SeqCst)
                    {
                        Some(("text/html".to_string(), html.clone()))
                    } else {
                        None
                    }
                } else {
                    assets::request_path(&request.request.url)
                        .and_then(|path| files.get(&path))
                        .map(|file| (file.mime.clone(), file.bytes.clone()))
                };
                match content {
                    Some((mime, body)) => {
                        RequestPausedDecision::Fulfill(Fetch::FulfillRequest {
                            request_id: request.request_id.clone(),
                            response_code: 200,
                            response_headers: Some(vec![
                                Fetch::HeaderEntry {
                                    name: "Content-Type".into(),
                                    value: mime,
                                },
                                Fetch::HeaderEntry {
                                    name: "Content-Security-Policy".into(),
                                    value: assets::RENDER_CSP.into(),
                                },
                                Fetch::HeaderEntry {
                                    name: "Access-Control-Allow-Origin".into(),
                                    value: "*".into(),
                                },
                                Fetch::HeaderEntry {
                                    name: "X-Content-Type-Options".into(),
                                    value: "nosniff".into(),
                                },
                                Fetch::HeaderEntry {
                                    name: "Cache-Control".into(),
                                    value: "no-store".into(),
                                },
                            ]),
                            binary_response_headers: None,
                            body: Some(STANDARD.encode(body)),
                            response_phrase: None,
                        })
                    }
                    None => RequestPausedDecision::Fail(Fetch::FailRequest {
                        request_id: request.request_id.clone(),
                        error_reason: Network::ErrorReason::BlockedByClient,
                    }),
                }
            },
        ))?;
        tab.enable_fetch(None, Some(false))?;
        tab.navigate_to(assets::DOCUMENT_URL)?
            .wait_until_navigated()?;
        let ready = tab.call_method(serde_json::from_value::<Runtime::Evaluate>(json!({
            "expression": "(async () => { await window.sequentTemplateReady; await document.fonts.ready; await Promise.all([...document.images].map(i => i.decode().catch(() => {}))); })()",
            "awaitPromise": true, "returnByValue": true, "timeout": 10000
        }))?)?;
        if ready.exception_details.is_some()
            || script_failed.load(Ordering::SeqCst)
        {
            bail!("Template readiness script failed");
        }
        tab.call_method(Emulation::SetScriptExecutionDisabled { value: true })?;
        let bytes = tab.print_to_pdf(Some(options))?;
        if bytes.len() > 8_000_000 {
            bail!("PDF output exceeds 8 MB");
        }
        Ok(bytes)
    })();
    // Wake the watchdog before dropping the browser on either success or error.
    let _ = done.send(());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_host_paths_before_launching_a_browser() {
        let files = [(
            "../secret".into(),
            assets::TemplateAsset {
                mime: "text/plain".into(),
                base64: "".into(),
            },
        )]
        .into();
        assert!(render("<h1>Test</h1>", &files, None)
            .unwrap_err()
            .to_string()
            .contains("Invalid relative"));
    }
    #[test]
    #[ignore = "requires a local Chromium executable; run in the Studio container"]
    fn attached_script_runs_while_network_and_host_files_are_blocked() {
        use std::{io::ErrorKind, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/secret", listener.local_addr().unwrap());
        let script = format!("document.title='Attached static files';document.body.append('Script worked');fetch('{url}').catch(()=>{{}});fetch('file:///etc/passwd').catch(()=>{{}});");
        let files = [(
            "scripts/main.js".into(),
            assets::TemplateAsset {
                mime: "text/javascript".into(),
                base64: STANDARD.encode(script),
            },
        )]
        .into();
        let pdf = render("<html><head><title>Before script</title></head><body><script src='/scripts/main.js'></script></body></html>", &files, None).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf
            .windows(b"Attached static files".len())
            .any(|s| s == b"Attached static files"));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            ErrorKind::WouldBlock
        );
    }
    #[test]
    #[ignore = "requires Chromium and exercises the 20-second process deadline"]
    fn looping_script_is_stopped_within_the_render_deadline() {
        let files = [(
            "loop.js".into(),
            assets::TemplateAsset {
                mime: "text/javascript".into(),
                base64: STANDARD.encode("while (true) {}"),
            },
        )]
        .into();
        let started = std::time::Instant::now();
        assert!(
            render("<script src='/loop.js'></script>", &files, None).is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(30));
    }
    #[test]
    fn oversized_paper_is_rejected_before_launching_a_browser() {
        let options = PrintToPdfOptions {
            paper_height: Some(1_000_000.0),
            ..Default::default()
        };
        assert!(
            render("<h1>Test</h1>", &TemplateAssets::new(), Some(options))
                .unwrap_err()
                .to_string()
                .contains("paper height")
        );
    }
}
