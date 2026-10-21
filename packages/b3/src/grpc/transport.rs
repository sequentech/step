// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Transport security of the bulletin board gRPC service.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use config::{Config, Environment};
use serde::Deserialize;
use strum::Display;
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint, Identity};
#[cfg(feature = "server")]
use tonic::transport::{Server, ServerTlsConfig};

/// Environment variable prefix of the b3 server transport settings.
pub const SERVER_ENV_PREFIX: &str = "B3";
/// Environment variable prefix of the b3 client transport settings.
pub const CLIENT_ENV_PREFIX: &str = "B3_CLIENT";

const HTTPS_SCHEME: &str = "https";
const TLS_CERT_PATH: &str = "tls_cert_path";
const TLS_KEY_PATH: &str = "tls_key_path";
const TLS_CA_PATH: &str = "tls_ca_path";

/// How the bulletin board gRPC connection is protected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum B3TransportSecurity {
    /// Unencrypted HTTP/2 without client authentication.
    #[default]
    Plaintext,
    /// TLS, the server presents a certificate that clients verify.
    Tls,
    /// TLS, and the server also requires a client certificate issued by
    /// its configured CA.
    MutualTls,
}

/// Transport settings of a b3 server or client.
///
/// The server reads them from `B3_TRANSPORT_SECURITY`, `B3_TLS_CERT_PATH`,
/// `B3_TLS_KEY_PATH` and `B3_TLS_CA_PATH`, and the client from the same
/// names with the `B3_CLIENT_` prefix.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TransportConfig {
    pub transport_security: B3TransportSecurity,
    /// PEM certificate chain this side presents.
    pub tls_cert_path: Option<PathBuf>,
    /// PEM private key of `tls_cert_path`.
    pub tls_key_path: Option<PathBuf>,
    /// PEM CA certificate that issues the peer certificates: the client
    /// certificates on the server, the server certificate on the client.
    pub tls_ca_path: Option<PathBuf>,
}

impl TransportConfig {
    /// Reads the settings from the environment variables with the given prefix.
    pub fn from_env(prefix: &str) -> Result<Self> {
        Self::from_environment(Environment::default().prefix(prefix))
    }

    fn from_environment(environment: Environment) -> Result<Self> {
        Ok(Config::builder()
            .add_source(environment.ignore_empty(true))
            .build()?
            .try_deserialize()?)
    }

    /// Returns a gRPC server builder that applies these settings.
    #[cfg(feature = "server")]
    pub fn server_builder(&self) -> Result<Server> {
        let client_ca = match self.transport_security {
            B3TransportSecurity::Plaintext => return Ok(Server::builder()),
            B3TransportSecurity::Tls => None,
            B3TransportSecurity::MutualTls => Some(self.ca_certificate()?),
        };

        let mut tls = ServerTlsConfig::new().identity(self.identity()?);
        if let Some(client_ca) = client_ca {
            tls = tls.client_ca_root(client_ca);
        }

        // tonic builds the rustls server configuration with the process
        // default crypto provider, which is ambiguous when several are enabled.
        let _ = rustls::crypto::ring::default_provider().install_default();

        Ok(Server::builder().tls_config(tls)?)
    }

    /// Returns an endpoint to the given url that applies these settings.
    pub fn endpoint(&self, url: &str) -> Result<Endpoint> {
        let endpoint = Endpoint::from_shared(url.to_string())?;
        let identity = match self.transport_security {
            B3TransportSecurity::Plaintext => return Ok(endpoint),
            B3TransportSecurity::Tls => None,
            B3TransportSecurity::MutualTls => Some(self.identity()?),
        };

        if endpoint.uri().scheme_str() != Some(HTTPS_SCHEME) {
            return Err(anyhow!(
                "transport security {} requires an {HTTPS_SCHEME} url, got {url}",
                self.transport_security
            ));
        }

        let mut tls = ClientTlsConfig::new().ca_certificate(self.ca_certificate()?);
        if let Some(identity) = identity {
            tls = tls.identity(identity);
        }

        Ok(endpoint.tls_config(tls)?)
    }

    fn identity(&self) -> Result<Identity> {
        let cert = read_pem(
            self.tls_cert_path.as_deref(),
            TLS_CERT_PATH,
            self.transport_security,
        )?;
        let key = read_pem(
            self.tls_key_path.as_deref(),
            TLS_KEY_PATH,
            self.transport_security,
        )?;

        Ok(Identity::from_pem(cert, key))
    }

    fn ca_certificate(&self) -> Result<Certificate> {
        let ca = read_pem(
            self.tls_ca_path.as_deref(),
            TLS_CA_PATH,
            self.transport_security,
        )?;

        Ok(Certificate::from_pem(ca))
    }
}

fn read_pem(
    path: Option<&Path>,
    setting: &str,
    transport_security: B3TransportSecurity,
) -> Result<Vec<u8>> {
    let path = path.ok_or_else(|| {
        anyhow!("{setting} is required with transport security {transport_security}")
    })?;

    std::fs::read(path).with_context(|| format!("Failed to read {setting} {}", path.display()))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::time::Duration;

    use rcgen::{
        BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    };
    use tempfile::TempDir;
    use tonic::transport::server::TcpIncoming;
    use tonic::{Request, Response, Status};

    use super::*;
    use crate::grpc::proto::b3_server::{B3Server, B3};
    use crate::grpc::{
        B3Client, GetBoardsReply, GetBoardsRequest, GetMessagesMultiReply, GetMessagesMultiRequest,
        GetMessagesReply, GetMessagesRequest, PutMessagesMultiReply, PutMessagesMultiRequest,
        PutMessagesReply, PutMessagesRequest,
    };

    const HOST: &str = "127.0.0.1";
    const TEST_BOARD: &str = "testboard";
    const TIMEOUT: Duration = Duration::from_secs(10);

    struct BoardsOnly;

    #[tonic::async_trait]
    impl B3 for BoardsOnly {
        async fn get_boards(
            &self,
            _request: Request<GetBoardsRequest>,
        ) -> Result<Response<GetBoardsReply>, Status> {
            Ok(Response::new(GetBoardsReply {
                boards: vec![TEST_BOARD.to_string()],
            }))
        }

        async fn get_messages(
            &self,
            _request: Request<GetMessagesRequest>,
        ) -> Result<Response<GetMessagesReply>, Status> {
            Ok(Response::new(GetMessagesReply::default()))
        }

        async fn put_messages(
            &self,
            _request: Request<PutMessagesRequest>,
        ) -> Result<Response<PutMessagesReply>, Status> {
            Ok(Response::new(PutMessagesReply::default()))
        }

        async fn get_messages_multi(
            &self,
            _request: Request<GetMessagesMultiRequest>,
        ) -> Result<Response<GetMessagesMultiReply>, Status> {
            Ok(Response::new(GetMessagesMultiReply::default()))
        }

        async fn put_messages_multi(
            &self,
            _request: Request<PutMessagesMultiRequest>,
        ) -> Result<Response<PutMessagesMultiReply>, Status> {
            Ok(Response::new(PutMessagesMultiReply::default()))
        }
    }

    struct TestCa {
        issuer: Issuer<'static, KeyPair>,
        cert_path: PathBuf,
    }

    impl TestCa {
        fn new(dir: &Path, name: &str) -> TestCa {
            let key = KeyPair::generate().unwrap();
            let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.distinguished_name.push(DnType::CommonName, name);
            let cert = params.self_signed(&key).unwrap();
            let cert_path = dir.join(format!("{name}.pem"));
            std::fs::write(&cert_path, cert.pem()).unwrap();

            TestCa {
                issuer: Issuer::new(params, key),
                cert_path,
            }
        }

        /// Issues a certificate for HOST and returns its certificate and key paths.
        fn issue(
            &self,
            dir: &Path,
            name: &str,
            usage: ExtendedKeyUsagePurpose,
        ) -> (PathBuf, PathBuf) {
            let key = KeyPair::generate().unwrap();
            let mut params = CertificateParams::new(vec![HOST.to_string()]).unwrap();
            params.distinguished_name.push(DnType::CommonName, name);
            params.extended_key_usages = vec![usage];
            let cert = params.signed_by(&key, &self.issuer).unwrap();

            let cert_path = dir.join(format!("{name}.pem"));
            let key_path = dir.join(format!("{name}.key"));
            std::fs::write(&cert_path, cert.pem()).unwrap();
            std::fs::write(&key_path, key.serialize_pem()).unwrap();

            (cert_path, key_path)
        }
    }

    /// A deployment CA that issues the server and trustee certificates,
    /// and an unrelated CA with a server and a client certificate of its own.
    struct TestPki {
        _dir: TempDir,
        ca_path: PathBuf,
        server: (PathBuf, PathBuf),
        trustee: (PathBuf, PathBuf),
        other_ca_server: (PathBuf, PathBuf),
        other_ca_client: (PathBuf, PathBuf),
    }

    impl TestPki {
        fn new() -> TestPki {
            let dir = TempDir::new().unwrap();
            let ca = TestCa::new(dir.path(), "ca");
            let other_ca = TestCa::new(dir.path(), "other_ca");

            TestPki {
                server: ca.issue(dir.path(), "server", ExtendedKeyUsagePurpose::ServerAuth),
                trustee: ca.issue(dir.path(), "trustee", ExtendedKeyUsagePurpose::ClientAuth),
                other_ca_server: other_ca.issue(
                    dir.path(),
                    "other_server",
                    ExtendedKeyUsagePurpose::ServerAuth,
                ),
                other_ca_client: other_ca.issue(
                    dir.path(),
                    "other_client",
                    ExtendedKeyUsagePurpose::ClientAuth,
                ),
                ca_path: ca.cert_path,
                _dir: dir,
            }
        }

        fn server_config(&self, transport_security: B3TransportSecurity) -> TransportConfig {
            TransportConfig {
                transport_security,
                tls_cert_path: Some(self.server.0.clone()),
                tls_key_path: Some(self.server.1.clone()),
                tls_ca_path: Some(self.ca_path.clone()),
            }
        }

        fn client_config(
            &self,
            transport_security: B3TransportSecurity,
            identity: Option<&(PathBuf, PathBuf)>,
        ) -> TransportConfig {
            TransportConfig {
                transport_security,
                tls_cert_path: identity.map(|(cert, _)| cert.clone()),
                tls_key_path: identity.map(|(_, key)| key.clone()),
                tls_ca_path: Some(self.ca_path.clone()),
            }
        }
    }

    async fn serve(config: &TransportConfig) -> SocketAddr {
        let incoming = TcpIncoming::bind(format!("{HOST}:0").parse().unwrap()).unwrap();
        let addr = incoming.local_addr().unwrap();
        let router = config
            .server_builder()
            .unwrap()
            .add_service(B3Server::new(BoardsOnly));
        tokio::spawn(router.serve_with_incoming(incoming));

        addr
    }

    async fn get_boards(config: &TransportConfig, url: &str) -> Result<Vec<String>> {
        let endpoint = config
            .endpoint(url)?
            .connect_timeout(TIMEOUT)
            .timeout(TIMEOUT);
        let mut client = B3Client::connect(endpoint).await?;
        let reply = client.get_boards(GetBoardsRequest {}).await?;

        Ok(reply.into_inner().boards)
    }

    fn https_url(addr: SocketAddr) -> String {
        format!("https://{addr}")
    }

    fn http_url(addr: SocketAddr) -> String {
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn mutual_tls_serves_client_certificate_from_configured_ca() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::MutualTls)).await;
        let client = pki.client_config(B3TransportSecurity::MutualTls, Some(&pki.trustee));

        let boards = get_boards(&client, &https_url(addr)).await.unwrap();

        assert_eq!(boards, vec![TEST_BOARD.to_string()]);
    }

    #[tokio::test]
    async fn mutual_tls_rejects_plaintext_client() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::MutualTls)).await;

        let result = get_boards(&TransportConfig::default(), &http_url(addr)).await;

        assert!(result.is_err(), "plaintext client was served: {result:?}");
    }

    #[tokio::test]
    async fn mutual_tls_rejects_client_without_certificate() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::MutualTls)).await;
        let client = pki.client_config(B3TransportSecurity::Tls, None);

        let result = get_boards(&client, &https_url(addr)).await;

        assert!(
            result.is_err(),
            "client without certificate was served: {result:?}"
        );
    }

    #[tokio::test]
    async fn mutual_tls_rejects_client_certificate_from_other_ca() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::MutualTls)).await;
        let client = pki.client_config(B3TransportSecurity::MutualTls, Some(&pki.other_ca_client));

        let result = get_boards(&client, &https_url(addr)).await;

        assert!(
            result.is_err(),
            "certificate from another CA was served: {result:?}"
        );
    }

    #[tokio::test]
    async fn tls_serves_client_that_trusts_server_ca() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::Tls)).await;
        let client = pki.client_config(B3TransportSecurity::Tls, None);

        let boards = get_boards(&client, &https_url(addr)).await.unwrap();

        assert_eq!(boards, vec![TEST_BOARD.to_string()]);
    }

    #[tokio::test]
    async fn tls_client_rejects_server_certificate_from_other_ca() {
        let pki = TestPki::new();
        let server = TransportConfig {
            tls_cert_path: Some(pki.other_ca_server.0.clone()),
            tls_key_path: Some(pki.other_ca_server.1.clone()),
            ..pki.server_config(B3TransportSecurity::Tls)
        };
        let addr = serve(&server).await;
        let client = pki.client_config(B3TransportSecurity::Tls, None);

        let result = get_boards(&client, &https_url(addr)).await;

        assert!(
            result.is_err(),
            "server certificate from another CA was accepted: {result:?}"
        );
    }

    #[tokio::test]
    async fn tls_rejects_plaintext_client() {
        let pki = TestPki::new();
        let addr = serve(&pki.server_config(B3TransportSecurity::Tls)).await;

        let result = get_boards(&TransportConfig::default(), &http_url(addr)).await;

        assert!(result.is_err(), "plaintext client was served: {result:?}");
    }

    #[tokio::test]
    async fn plaintext_serves_plaintext_client() {
        let addr = serve(&TransportConfig::default()).await;

        let boards = get_boards(&TransportConfig::default(), &http_url(addr))
            .await
            .unwrap();

        assert_eq!(boards, vec![TEST_BOARD.to_string()]);
    }

    #[test]
    fn tls_client_rejects_http_url() {
        let pki = TestPki::new();
        let client = pki.client_config(B3TransportSecurity::MutualTls, Some(&pki.trustee));

        assert!(client.endpoint("http://b3:50051").is_err());
    }

    #[test]
    fn mutual_tls_client_requires_certificate() {
        let pki = TestPki::new();
        let client = pki.client_config(B3TransportSecurity::MutualTls, None);

        assert!(client.endpoint("https://b3:50051").is_err());
    }

    #[test]
    fn mutual_tls_server_requires_client_ca() {
        let pki = TestPki::new();
        let server = TransportConfig {
            tls_ca_path: None,
            ..pki.server_config(B3TransportSecurity::MutualTls)
        };

        assert!(server.server_builder().is_err());
    }

    #[test]
    fn tls_server_requires_certificate() {
        let server = TransportConfig {
            transport_security: B3TransportSecurity::Tls,
            ..TransportConfig::default()
        };

        assert!(server.server_builder().is_err());
    }

    fn from_variables(prefix: &str, variables: &[(&str, &str)]) -> Result<TransportConfig> {
        let source: HashMap<String, String> = variables
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();

        TransportConfig::from_environment(
            Environment::default().prefix(prefix).source(Some(source)),
        )
    }

    #[test]
    fn reads_server_settings_from_environment() {
        let config = from_variables(
            SERVER_ENV_PREFIX,
            &[
                ("B3_TRANSPORT_SECURITY", "mutual_tls"),
                ("B3_TLS_CERT_PATH", "/certs/b3.pem"),
                ("B3_TLS_KEY_PATH", "/certs/b3.key"),
                ("B3_TLS_CA_PATH", "/certs/trustees_ca.pem"),
                ("B3_CLIENT_TLS_CA_PATH", "/certs/b3_ca.pem"),
                ("B3_BIND", "0.0.0.0:50051"),
            ],
        )
        .unwrap();

        assert_eq!(config.transport_security, B3TransportSecurity::MutualTls);
        assert_eq!(config.tls_cert_path, Some(PathBuf::from("/certs/b3.pem")));
        assert_eq!(config.tls_key_path, Some(PathBuf::from("/certs/b3.key")));
        assert_eq!(
            config.tls_ca_path,
            Some(PathBuf::from("/certs/trustees_ca.pem"))
        );
    }

    #[test]
    fn reads_client_settings_from_environment() {
        let config = from_variables(
            CLIENT_ENV_PREFIX,
            &[
                ("B3_TRANSPORT_SECURITY", "plaintext"),
                ("B3_TLS_CA_PATH", "/certs/trustees_ca.pem"),
                ("B3_CLIENT_TRANSPORT_SECURITY", "tls"),
                ("B3_CLIENT_TLS_CA_PATH", "/certs/b3_ca.pem"),
                ("B3_URL", "https://b3:50051"),
            ],
        )
        .unwrap();

        assert_eq!(config.transport_security, B3TransportSecurity::Tls);
        assert_eq!(config.tls_ca_path, Some(PathBuf::from("/certs/b3_ca.pem")));
        assert_eq!(config.tls_cert_path, None);
    }

    #[test]
    fn defaults_to_plaintext_when_unset_or_empty() {
        let unset = from_variables(SERVER_ENV_PREFIX, &[]).unwrap();
        let empty = from_variables(
            SERVER_ENV_PREFIX,
            &[("B3_TRANSPORT_SECURITY", ""), ("B3_TLS_CERT_PATH", "")],
        )
        .unwrap();

        assert_eq!(unset.transport_security, B3TransportSecurity::Plaintext);
        assert_eq!(empty.transport_security, B3TransportSecurity::Plaintext);
        assert_eq!(empty.tls_cert_path, None);
    }

    #[test]
    fn rejects_unknown_transport_security() {
        let result = from_variables(SERVER_ENV_PREFIX, &[("B3_TRANSPORT_SECURITY", "none")]);

        assert!(result.is_err());
    }
}
