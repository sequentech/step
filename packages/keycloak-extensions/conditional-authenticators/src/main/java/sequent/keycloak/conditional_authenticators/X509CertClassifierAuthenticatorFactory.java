// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import java.util.List;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

/** Factory for {@link X509CertClassifierAuthenticator}. */
@AutoService(AuthenticatorFactory.class)
public class X509CertClassifierAuthenticatorFactory implements AuthenticatorFactory {

  public static final String PROVIDER_ID = "x509-cert-classifier";

  /**
   * Authenticator config key for the HTTP header name that carries the forwarded client certificate
   * (PEM-encoded, URL-encoded). Defaults to {@code ssl-client-cert} (nginx default). Use {@code
   * Cf-Tls-Client-Cert} for Cloudflare or {@code Client-Cert} for RFC 9440 proxies.
   */
  public static final String CONF_CERT_HEADER_NAME = "cert-header-name";

  private static final Requirement[] REQUIREMENT_CHOICES = {
    Requirement.REQUIRED, Requirement.DISABLED
  };

  @Override
  public void init(Scope config) {
    // no-op
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    // no-op
  }

  @Override
  public void close() {
    // no-op
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "X509 Certificate Classifier";
  }

  @Override
  public String getReferenceCategory() {
    return "x509";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Reads the issuer CN from the client certificate forwarded by a reverse proxy and sets the"
        + " auth note 'cert-type' to the CN, or 'not-allowed' if no certificate is present."
        + " Configure 'cert-header-name' to match the header used by your proxy"
        + " (e.g. ssl-client-cert for nginx, Cf-Tls-Client-Cert for Cloudflare,"
        + " Client-Cert for RFC 9440).";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONF_CERT_HEADER_NAME,
            "Client Certificate Header Name",
            "HTTP header name that carries the URL-encoded PEM client certificate forwarded by the"
                + " reverse proxy. Use 'ssl-client-cert' for nginx (default),"
                + " 'Cf-Tls-Client-Cert' for Cloudflare, or 'Client-Cert' for RFC 9440 proxies.",
            ProviderConfigProperty.STRING_TYPE,
            X509CertClassifierAuthenticator.DEFAULT_CERT_HEADER));
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return new X509CertClassifierAuthenticator();
  }
}
