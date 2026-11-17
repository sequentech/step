// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Accepts mTLS connections and serves the API on each, as the caller its
//! certificate names. A certificate the policy doesn't name gets no API.

use std::sync::Arc;

use anyhow::Result;
use axum::Extension;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::api::{router, CallerName};
use crate::service::Service;
use crate::tls::spki_sha256;

pub async fn serve(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    service: Arc<Service>,
) -> Result<()> {
    let acceptor = TlsAcceptor::from(tls);
    let app = router(service.clone());
    loop {
        let (stream, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let app = app.clone();
        let service = service.clone();
        tokio::spawn(async move {
            let tls = match acceptor.accept(stream).await {
                Ok(tls) => tls,
                Err(error) => {
                    tracing::warn!(%peer, %error, "TLS handshake refused");
                    return;
                }
            };
            let Some(certificate) = tls
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|c| c.first())
                .cloned()
            else {
                return;
            };
            let Ok(spki) = spki_sha256(&certificate) else {
                return;
            };
            let Some(caller) = service
                .policy()
                .caller_by_spki(&spki)
                .map(|c| c.name.clone())
            else {
                tracing::warn!(%peer, "a certificate the policy doesn't name");
                let _ = service.record_unknown_caller(&spki);
                return;
            };
            let app = app.layer(Extension(CallerName(caller)));
            let connection = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(tls), TowerToHyperService::new(app));
            if let Err(error) = connection.await {
                tracing::debug!(%peer, %error, "connection closed");
            }
        });
    }
}
