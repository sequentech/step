// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.HttpHeaders;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.Event;
import org.keycloak.events.EventBuilder;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.x509.X509ClientCertificateLookup;
import org.keycloak.sessions.AuthenticationSessionModel;

class X509UserResolutionAuthenticatorTest {

  private static final String PROXY_SECRET = "configured-proxy-secret";

  private final X509UserResolutionAuthenticator authenticator =
      new X509UserResolutionAuthenticator();
  private AuthenticationFlowContext context;
  private HttpRequest request;
  private HttpHeaders headers;
  private X509ClientCertificateLookup lookup;
  private Map<String, String> config;

  @BeforeEach
  void setUp() {
    context = mock(AuthenticationFlowContext.class);
    request = mock(HttpRequest.class);
    headers = mock(HttpHeaders.class);
    lookup = mock(X509ClientCertificateLookup.class);
    KeycloakSession session = mock(KeycloakSession.class);
    EventBuilder eventBuilder = mock(EventBuilder.class);
    config = new HashMap<>();
    AuthenticatorConfigModel configModel = new AuthenticatorConfigModel();
    configModel.setConfig(config);

    when(context.getSession()).thenReturn(session);
    when(session.getProvider(X509ClientCertificateLookup.class)).thenReturn(lookup);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getHttpHeaders()).thenReturn(headers);
    when(context.getAuthenticatorConfig()).thenReturn(configModel);
    when(context.getEvent()).thenReturn(eventBuilder);
    when(eventBuilder.getEvent()).thenReturn(new Event());
    when(context.getAuthenticationSession()).thenReturn(mock(AuthenticationSessionModel.class));
  }

  @Test
  void readsForwardedCertificateWhenNoTrustPolicyIsConfigured() throws Exception {
    authenticator.authenticate(context);

    verify(lookup).getCertificateChain(request);
  }

  @Test
  void readsForwardedCertificateWithMatchingProxySecret() throws Exception {
    config.put(
        X509CertHeaderTrust.CONF_TRUST_POLICY,
        X509CertHeaderTrust.Policy.REQUIRE_PROXY_SECRET.name());
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER)).thenReturn(PROXY_SECRET);

    authenticator.authenticate(context);

    verify(lookup).getCertificateChain(request);
  }

  @Test
  void doesNotReadForwardedCertificateWithoutProxySecret() throws Exception {
    config.put(
        X509CertHeaderTrust.CONF_TRUST_POLICY,
        X509CertHeaderTrust.Policy.REQUIRE_PROXY_SECRET.name());
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);

    authenticator.authenticate(context);

    verify(lookup, never()).getCertificateChain(any());
    verify(context).attempted();
    verify(context, never()).success();
  }

  @Test
  void doesNotReadForwardedCertificateWithWrongProxySecret() throws Exception {
    config.put(
        X509CertHeaderTrust.CONF_TRUST_POLICY,
        X509CertHeaderTrust.Policy.REQUIRE_PROXY_SECRET.name());
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER))
        .thenReturn("other-secret");

    authenticator.authenticate(context);

    verify(lookup, never()).getCertificateChain(any());
    verify(context).attempted();
    verify(context, never()).success();
  }

  @Test
  void doesNotReadForwardedCertificateWhenRequiredProxySecretIsNotConfigured() throws Exception {
    config.put(
        X509CertHeaderTrust.CONF_TRUST_POLICY,
        X509CertHeaderTrust.Policy.REQUIRE_PROXY_SECRET.name());
    when(headers.getHeaderString(X509CertHeaderTrust.PROXY_SECRET_HEADER)).thenReturn("");

    authenticator.authenticate(context);

    verify(lookup, never()).getCertificateChain(any());
    verify(context).attempted();
    verify(context, never()).success();
  }

  @Test
  void unknownTrustPolicyRequiresProxySecret() throws Exception {
    config.put(X509CertHeaderTrust.CONF_TRUST_POLICY, "NOT_A_POLICY");
    config.put(X509CertHeaderTrust.CONF_PROXY_SECRET, PROXY_SECRET);

    authenticator.authenticate(context);

    verify(lookup, never()).getCertificateChain(any());
    verify(context).attempted();
    verify(context, never()).success();
  }
}
