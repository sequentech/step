# Local development certificates

Run `.devcontainer/scripts/initialize-command.sh` from the repository root before
building the development Compose services. It creates a local self-signed nginx
certificate and a separate SimpleSAML signing certificate with fresh private keys.
It also creates `vp-sso-signing.key`/`.crt` for the local SAML client. Only the public
certificate enters the Keycloak realm; a client that signs requests uses the local
private file. The copied private key and its old public trust entry are removed.
Python 3 and OpenSSL must be installed on the host. Generated files are ignored by Git.

Reopening the environment preserves existing credentials and certificates. To renew
a certificate, stop the affected local service, retain its existing pair if needed,
remove that pair, rerun initialization and rebuild the affected image. Reimport the
new IdP public certificate into local Keycloak when renewing the SimpleSAML pair.
These certificates are development inputs; use deployment-specific certificates
and controlled key storage for an installation.
