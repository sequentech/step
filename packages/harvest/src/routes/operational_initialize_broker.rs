// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Private typed broker client for the unmounted Initialize-voting route.
//! Endpoint, trust anchor and client identity are deployment provisioning,
//! not normal UI configuration. When a reply may be lost after device use,
//! do not retry the HTTP call automatically.

use super::operational_initialize::{
    BrokerFailure, BrokerFuture, BrokerRedirect, BrokerScope,
    OperationalInitializeBroker, Settled,
};
use reqwest::{Certificate, Client, Identity, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

const MAX_REPLY: usize = 16 * 1024;

pub(crate) struct PrivateInitializeBroker {
    tenant: Uuid,
    client: Client,
    start: Url,
    eligibility: Url,
    finish: Url,
    authorize: Url,
    oidc_client: String,
}

#[derive(Serialize)]
struct Return<'a> {
    scope: &'a BrokerScope,
    attempt: Uuid,
    state: &'a str,
    code: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Eligibility {
    scope: BrokerScope,
    challenge_available: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Started {
    scope: BrokerScope,
    attempt: Uuid,
    redirect: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Completed {
    scope: BrokerScope,
    approval_challenge: Uuid,
}

fn fixed_https(value: &str) -> Result<Url, BrokerFailure> {
    let url = Url::parse(value).map_err(|_| BrokerFailure::Unavailable)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(BrokerFailure::Unavailable);
    }
    Ok(url)
}

impl PrivateInitializeBroker {
    /// One separate per-tenant mTLS certificate (not the signer HSM key) and
    /// explicit private trusted root(s). Reject trust from the platform, DNS
    /// redirects and proxy env variables. Do not log or store the PEM here.
    pub(crate) fn new(
        tenant: Uuid,
        broker_origin: &str,
        keycloak_base: &str,
        oidc_client: &str,
        roots: &[Certificate],
        tls_id: Identity,
    ) -> Result<Self, BrokerFailure> {
        let keycloak = fixed_https(keycloak_base)?;
        let authorize = fixed_https(&format!(
            "{}/realms/tenant-{tenant}/protocol/openid-connect/auth",
            keycloak.as_str().trim_end_matches('/')
        ))?;
        Self::with_authority(
            tenant,
            broker_origin,
            authorize,
            oidc_client,
            roots,
            tls_id,
            None,
        )
    }

    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    pub(crate) fn for_local_initialize(
        portal: &crate::services::operational_portal_auth::OperationalPortalAuth,
        registered_authorization: &str,
        registered_callback: &str,
        oidc_client: &str,
        broker_origin: &str,
        broker_peer: std::net::SocketAddrV4,
        roots: &[Certificate],
        tls_id: Identity,
    ) -> Result<Self, BrokerFailure> {
        let (tenant, authorize) = portal
            .local_approval_endpoint(
                registered_authorization,
                registered_callback,
                oidc_client,
            )
            .ok_or(BrokerFailure::Unavailable)?;
        let origin = fixed_https(broker_origin)?;
        let host = origin.host_str().ok_or(BrokerFailure::Unavailable)?;
        if !broker_peer.ip().is_private()
            || broker_peer.port() == 0
            || host.is_empty()
            || host.starts_with('-')
            || host.ends_with('-')
            || !host.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
            })
            || broker_origin != format!("https://{host}:{}", broker_peer.port())
        {
            return Err(BrokerFailure::Unavailable);
        }
        Self::with_authority(
            tenant,
            broker_origin,
            authorize,
            oidc_client,
            roots,
            tls_id,
            Some(broker_peer),
        )
    }

    fn with_authority(
        tenant: Uuid,
        broker_origin: &str,
        authorize: Url,
        oidc_client: &str,
        roots: &[Certificate],
        tls_id: Identity,
        peer: Option<std::net::SocketAddrV4>,
    ) -> Result<Self, BrokerFailure> {
        if tenant.is_nil()
            || roots.is_empty()
            || oidc_client.is_empty()
            || oidc_client.len() > 150
            || !oidc_client
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(BrokerFailure::Unavailable);
        }
        let origin = fixed_https(broker_origin)?;
        if origin.path() != "/" {
            return Err(BrokerFailure::Unavailable);
        }
        let route = |name| -> Result<Url, BrokerFailure> {
            origin.join(name).map_err(|_| BrokerFailure::Unavailable)
        };
        let mut builder = Client::builder()
            .use_native_tls()
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .tls_built_in_root_certs(false)
            .identity(tls_id)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .https_only(true)
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(20));
        if let Some(peer) = peer {
            builder = builder.resolve(
                origin.host_str().ok_or(BrokerFailure::Unavailable)?,
                peer.into(),
            );
        }
        for root in roots {
            builder = builder.add_root_certificate(root.clone());
        }
        let client = builder.build().map_err(|_| BrokerFailure::Unavailable)?;
        Ok(Self {
            tenant,
            client,
            start: route("/private/v1/operational-signing/initialize/start")?,
            eligibility: route(
                "/private/v1/operational-signing/initialize/eligibility",
            )?,
            finish: route("/private/v1/operational-signing/initialize/finish")?,
            authorize,
            oidc_client: oidc_client.into(),
        })
    }

    fn authorize_scope(
        &self,
        scope: &BrokerScope,
    ) -> Result<(), BrokerFailure> {
        if scope.tenant != self.tenant
            || [scope.event, scope.request, scope.official]
                .iter()
                .any(Uuid::is_nil)
            || scope.payload_sha256.len() != 64
            || !scope
                .payload_sha256
                .bytes()
                .all(|x| x.is_ascii_digit() || (b'a'..=b'f').contains(&x))
        {
            return Err(BrokerFailure::Refused);
        }
        Ok(())
    }
}

impl OperationalInitializeBroker for PrivateInitializeBroker {
    fn can_offer<'a>(
        &'a self,
        scope: &'a BrokerScope,
    ) -> BrokerFuture<'a, bool> {
        Box::pin(async move {
            self.authorize_scope(scope)?;
            let response = self
                .client
                .post(self.eligibility.clone())
                .json(scope)
                .send()
                .await
                .map_err(|_| BrokerFailure::Unavailable)?;
            let reply: Eligibility = exact_body(response, false).await?;
            if reply.scope != *scope {
                return Err(BrokerFailure::Refused);
            }
            Ok(reply.challenge_available)
        })
    }

    fn start<'a>(
        &'a self,
        scope: &'a BrokerScope,
    ) -> BrokerFuture<'a, BrokerRedirect> {
        Box::pin(async move {
            self.authorize_scope(scope)?;
            let response = self
                .client
                .post(self.start.clone())
                .json(scope)
                .send()
                .await
                .map_err(|_| BrokerFailure::Unavailable)?;
            let started: Started = exact_body(response, false).await?;
            if started.scope != *scope || started.attempt.is_nil() {
                return Err(BrokerFailure::Refused);
            }
            let location = fixed_par(&started.redirect)?;
            if location.as_str() != started.redirect
                || !super::operational_initialize::matches_pushed_redirect(
                    &location,
                    &self.authorize,
                    &self.oidc_client,
                )
            {
                return Err(BrokerFailure::Refused);
            }
            Ok(BrokerRedirect {
                attempt: started.attempt,
                location,
            })
        })
    }

    fn finish<'a>(
        &'a self,
        scope: &'a BrokerScope,
        attempt: Uuid,
        state: &'a str,
        code: &'a str,
    ) -> BrokerFuture<'a, Settled> {
        Box::pin(async move {
            self.authorize_scope(scope)?;
            let response = self
                .client
                .post(self.finish.clone())
                .json(&Return {
                    scope,
                    attempt,
                    state,
                    code,
                })
                .send()
                .await
                .map_err(|_| BrokerFailure::Reconcile)?;
            let completed: Completed = exact_body(response, true).await?;
            if completed.scope != *scope
                || completed.approval_challenge.is_nil()
            {
                return Err(BrokerFailure::Reconcile);
            }
            Ok(Settled {
                scope: completed.scope,
                approval_challenge: completed.approval_challenge,
            })
        })
    }

    fn authorization_endpoint(&self) -> &Url {
        &self.authorize
    }

    fn oidc_client_id(&self) -> &str {
        &self.oidc_client
    }
}

// The caller separately enforces exact origin/path/parameters before any
// browser receives the pushed authorization URL.
fn fixed_par(value: &str) -> Result<Url, BrokerFailure> {
    if value.len() > 2048 {
        return Err(BrokerFailure::Unavailable);
    }
    Url::parse(value).map_err(|_| BrokerFailure::Unavailable)
}

pub(super) async fn exact_body<T: DeserializeOwned>(
    mut response: Response,
    may_have_started_device: bool,
) -> Result<T, BrokerFailure> {
    let incomplete = || {
        if may_have_started_device {
            BrokerFailure::Reconcile
        } else {
            BrokerFailure::Unavailable
        }
    };
    match response.status() {
        StatusCode::OK => {}
        // A pre-device denied / unavailable response cannot be translated
        // by this client into permission to issue a new challenge.
        _ if may_have_started_device => return Err(incomplete()),
        StatusCode::UNAUTHORIZED
        | StatusCode::FORBIDDEN
        | StatusCode::NOT_FOUND => return Err(BrokerFailure::Refused),
        _ => return Err(incomplete()),
    }
    let mut data = Vec::<u8>::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| incomplete())? {
        if chunk.len() > MAX_REPLY.saturating_sub(data.len()) {
            return Err(incomplete());
        }
        data.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&data).map_err(|_| incomplete())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn reply(
        status: &str,
        body: &[u8],
        declared_length: Option<usize>,
    ) -> Response {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let message = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            declared_length.unwrap_or(body.len()),
        );
        let body = body.to_vec();
        let peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                let mut byte = [0];
                if stream.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            assert!(head.starts_with(b"GET /reply HTTP/1.1\r\n"));
            stream.write_all(message.as_bytes()).await.unwrap();
            stream.write_all(&body).await.unwrap();
            stream.shutdown().await.unwrap();
        });
        // Only the response decoder, not private transport/auth policy, is
        // under test. No broker credentials are sent to this HTTP stand-in.
        let response = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
            .get(format!("http://{address}/reply"))
            .send()
            .await
            .unwrap();
        peer.await.unwrap();
        response
    }

    #[tokio::test]
    async fn unknown_or_incomplete_post_device_reply_never_authorizes_a_retry()
    {
        let bytes = br#"{"eligible":true}"#;
        let result = exact_body::<serde_json::Value>(
            reply("200 OK", bytes, None).await,
            false,
        )
        .await;
        assert!(matches!(result, Ok(ref value) if value["eligible"] == true));

        for status in ["401 Unauthorized", "403 Forbidden", "404 Not Found"] {
            let response = reply(status, b"", None).await;
            assert!(matches!(
                exact_body::<serde_json::Value>(response, false).await,
                Err(BrokerFailure::Refused)
            ));
        }
        for (status, body, declared_length) in [
            ("503 Service Unavailable", b"".as_slice(), None),
            ("200 OK", b"{}".as_slice(), Some(16)),
            ("200 OK", b"not-json".as_slice(), None),
        ] {
            for may_have_signed in [false, true] {
                let response = reply(status, body, declared_length).await;
                let result =
                    exact_body::<serde_json::Value>(response, may_have_signed)
                        .await;
                assert!(match may_have_signed {
                    true => matches!(result, Err(BrokerFailure::Reconcile)),
                    false => matches!(result, Err(BrokerFailure::Unavailable)),
                });
            }
        }
        let oversized = vec![b' '; MAX_REPLY + 1];
        assert!(matches!(
            exact_body::<serde_json::Value>(
                reply("200 OK", &oversized, None).await,
                true
            )
            .await,
            Err(BrokerFailure::Reconcile)
        ));
        let may_have_signed = reply("403 Forbidden", b"", None).await;
        assert!(matches!(
            exact_body::<serde_json::Value>(may_have_signed, true).await,
            Err(BrokerFailure::Reconcile)
        ));
    }

    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    #[test]
    fn local_private_broker_only_offers_the_installed_public_par_location() {
        use crate::route_services::service_tokens::RealmSigner;
        use crate::services::operational_portal_auth::OperationalPortalAuth;
        use openssl::{
            asn1::Asn1Time,
            hash::MessageDigest,
            pkey::PKey,
            rsa::Rsa,
            x509::{X509NameBuilder, X509},
        };

        let tenant = Uuid::new_v4();
        let issuer = format!("http://localhost:18090/realms/tenant-{tenant}");
        let auth = OperationalPortalAuth::for_local_initialize(
            tenant,
            "http://localhost:3002",
            "http://localhost:18090/",
            "normal-admin",
            issuer.clone(),
            "normal-admin".into(),
            RealmSigner::shared().jwks(),
        )
        .unwrap();
        let endpoint = format!("{issuer}/protocol/openid-connect/auth");
        let callback = "http://localhost:3002/operational-signing/return";
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "synthetic-harvest")
            .unwrap();
        let name = name.build();
        let mut cert = X509::builder().unwrap();
        cert.set_version(2).unwrap();
        cert.set_subject_name(&name).unwrap();
        cert.set_issuer_name(&name).unwrap();
        cert.set_pubkey(&key).unwrap();
        cert.set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        cert.set_not_after(&Asn1Time::days_from_now(1).unwrap())
            .unwrap();
        cert.sign(&key, MessageDigest::sha256()).unwrap();
        let pem = cert.build().to_pem().unwrap();
        let cert = Certificate::from_pem(&pem).unwrap();
        let identity = Identity::from_pkcs8_pem(
            &pem,
            &key.private_key_to_pem_pkcs8().unwrap(),
        )
        .unwrap();

        let install = |endpoint: &str, callback: &str, broker: &str| {
            PrivateInitializeBroker::for_local_initialize(
                &auth,
                endpoint,
                callback,
                "synthetic-gold",
                broker,
                "10.0.0.54:8843".parse().unwrap(),
                std::slice::from_ref(&cert),
                identity.clone(),
            )
        };
        let origin = "https://private-broker:8843";
        let broker = install(&endpoint, callback, origin).unwrap();
        assert!(
            install(&endpoint.replace("18090", "8090"), callback, origin)
                .is_err()
        );
        assert!(install(&endpoint, &format!("{callback}?"), origin).is_err());
        assert!(
            install(&endpoint, callback, "http://private-broker:8843").is_err()
        );
        let mut redirect = Url::parse(&endpoint).unwrap();
        redirect
            .query_pairs_mut()
            .append_pair("client_id", "synthetic-gold")
            .append_pair(
                "request_uri",
                "urn:ietf:params:oauth:request_uri:opaque",
            );
        let allowed = |url: &Url| {
            super::super::operational_initialize::matches_pushed_redirect(
                url,
                broker.authorization_endpoint(),
                broker.oidc_client_id(),
            )
        };
        assert!(allowed(&redirect));
        for wrong in [
            redirect.as_str().replace("18090", "8090"),
            redirect.as_str().replace("synthetic-gold", "normal-admin"),
            redirect.as_str().replace("%3A", ":"),
            format!("{redirect}&next=x"),
        ] {
            assert!(!allowed(&Url::parse(&wrong).unwrap()));
        }
    }

    #[test]
    fn broker_return_is_parsed_only_with_bounded_authority() {
        assert!(fixed_par(&"x".repeat(2049)).is_err());
        assert!(fixed_par("bad input").is_err());
        assert!(fixed_par("https://valid.invalid/realms/tenant").is_ok());
        for url in [
            "http://valid.invalid/",
            "https://a:b@valid.invalid/",
            "https://valid.invalid/?to=elsewhere",
            "https://valid.invalid/#elsewhere",
            "file:///tmp/not-an-endpoint",
            "not a URL",
        ] {
            assert!(fixed_https(url).is_err(), "{url}");
        }
        assert!(fixed_https("https://valid.invalid/realms/tenant").is_ok());
    }
}
