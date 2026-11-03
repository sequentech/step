// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;
import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.CERT_NOT_PROVIDED;

import jakarta.ws.rs.core.HttpHeaders;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.sessions.AuthenticationSessionModel;

class X509CertClassifierAuthenticatorTest {

  static final String REQUIRE_PROXY_SECRET = X509CertHeaderTrust.Policy.REQUIRE_PROXY_SECRET.name();
  static final String PROXY_SECRET = "configured-proxy-secret";

  private static final String CERT_PEM =
      "-----BEGIN CERTIFICATE-----\n"
          + "MIIBhjCCAS2gAwIBAgIUWYAxu67k82sj2zYNPIVwpNVq9+gwCgYIKoZIzj0EAwIw\n"
          + "GDEWMBQGA1UEAwwNVGVzdCBWb3RlciBDQTAgFw0yNjEwMDkxODAxMTlaGA8yMTI2\n"
          + "MDkxNTE4MDExOVowGDEWMBQGA1UEAwwNVGVzdCBWb3RlciBDQTBZMBMGByqGSM49\n"
          + "AgEGCCqGSM49AwEHA0IABH/3Krl32vVCzXaLbcpAj5FG5G5gqhKWKbTG58Mdml5Y\n"
          + "ALu2cfoKY4qfCW0si/61ijNJ/CjXeQOQ6/mpeGqtVcOjUzBRMB0GA1UdDgQWBBSx\n"
          + "RMi6wu2zM5hGI50PgAAXts0Y9TAfBgNVHSMEGDAWgBSxRMi6wu2zM5hGI50PgAAX\n"
          + "ts0Y9TAPBgNVHRMBAf8EBTADAQH/MAoGCCqGSM49BAMCA0cAMEQCICVwO+8OiUwq\n"
          + "A1l0tdGhyQDAsxfTqXYmzntX7vzx75qYAiA/tjDhPQyoVRcBgWSq1wH5+hG5YbHN\n"
          + "55QhhuM17EgFkQ==\n"
          + "-----END CERTIFICATE-----\n";
  private static final String CERT_ISSUER_CN = "Test Voter CA";

  private final X509CertClassifierAuthenticator authenticator =
      new X509CertClassifierAuthenticator();
  private AuthenticationFlowContext context;
  private HttpHeaders headers;
  private AuthenticationSessionModel authSession;
  private Map<String, String> config;

  @BeforeEach
  void setUp() {
    context = mock(AuthenticationFlowContext.class);
    HttpRequest request = mock(HttpRequest.class);
    headers = mock(HttpHeaders.class);
    authSession = mock(AuthenticationSessionModel.class);
    config = new HashMap<>();
    AuthenticatorConfigModel configModel = new AuthenticatorConfigModel();
    configModel.setConfig(config);

    when(context.getHttpRequest()).thenReturn(request);
    when(request.getHttpHeaders()).thenReturn(headers);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getEvent()).thenReturn(mock(EventBuilder.class));
    when(context.getAuthenticatorConfig()).thenReturn(configModel);
    when(headers.getHeaderString(X509CertClassifierAuthenticator.DEFAULT_CERT_HEADER))
        .thenReturn(URLEncoder.encode(CERT_PEM, StandardCharsets.UTF_8));
  }

  @Test
  void classifiesForwardedCertificateWhenNoTrustPolicyIsConfigured() {
    authenticator.authenticate(context);

    verify(authSession)
        .setAuthNote(X509CertClassifierAuthenticator.AUTH_NOTE_CERT_TYPE, CERT_ISSUER_CN);
    verify(context).success();
  }

  @Test
  void classifiesForwardedCertificateWithMatchingProxySecret() {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, REQUIRE_PROXY_SECRET);
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER)).thenReturn(PROXY_SECRET);

    authenticator.authenticate(context);

    verify(authSession)
        .setAuthNote(X509CertClassifierAuthenticator.AUTH_NOTE_CERT_TYPE, CERT_ISSUER_CN);
    verify(context).success();
  }

  @Test
  void ignoresForwardedCertificateWithoutProxySecret() {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, REQUIRE_PROXY_SECRET);
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);

    authenticator.authenticate(context);

    assertTreatedAsNoCertificate();
  }

  @Test
  void ignoresForwardedCertificateWithWrongProxySecret() {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, REQUIRE_PROXY_SECRET);
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER))
        .thenReturn("other-secret");

    authenticator.authenticate(context);

    assertTreatedAsNoCertificate();
  }

  @Test
  void ignoresForwardedCertificateWhenRequiredProxySecretIsNotConfigured() {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, REQUIRE_PROXY_SECRET);
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER)).thenReturn("");

    authenticator.authenticate(context);

    assertTreatedAsNoCertificate();
  }

  @Test
  void unknownTrustPolicyRequiresProxySecret() {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, "NOT_A_POLICY");
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);

    authenticator.authenticate(context);

    assertTreatedAsNoCertificate();
  }

  private void assertTreatedAsNoCertificate() {
    verify(authSession).setAuthNote(AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
    verify(authSession, never())
        .setAuthNote(eq(X509CertClassifierAuthenticator.AUTH_NOTE_CERT_TYPE), anyString());
    verify(context).attempted();
    verify(context, never()).success();
  }
}
