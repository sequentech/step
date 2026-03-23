// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.truststore;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.net.URL;
import java.security.cert.X509Certificate;
import java.util.List;
import java.util.Map;
import javax.security.auth.x500.X500Principal;
import org.junit.jupiter.api.Test;
import org.keycloak.Config;
import org.keycloak.common.enums.HostnameVerificationPolicy;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.truststore.TruststoreProvider;

class UrlTruststoreProviderFactoryTest {

  private static String certUrl(String filename) {
    URL resource =
        UrlTruststoreProviderFactoryTest.class.getClassLoader().getResource("certs/" + filename);
    assertNotNull(resource, "Test resource not found: certs/" + filename);
    return resource.toString();
  }

  /** Creates and initialises a factory using the given PEM URL as the global truststore. */
  private static UrlTruststoreProviderFactory initFactory(String globalUrl) {
    Config.Scope config = mock(Config.Scope.class);
    when(config.get("url")).thenReturn(globalUrl);
    when(config.get("hostname-verification-policy", "DEFAULT")).thenReturn("DEFAULT");
    when(config.getLong("refresh-interval-seconds", 0L)).thenReturn(0L);
    UrlTruststoreProviderFactory factory = new UrlTruststoreProviderFactory();
    factory.init(config);
    return factory;
  }

  /** Creates a session mock whose realm returns the given truststore URL attribute (may be null). */
  private static KeycloakSession sessionWithRealm(String realmId, String truststoreUrl) {
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakContext ctx = mock(KeycloakContext.class);
    RealmModel realm = mock(RealmModel.class);
    when(session.getContext()).thenReturn(ctx);
    when(ctx.getRealm()).thenReturn(realm);
    when(realm.getId()).thenReturn(realmId);
    when(realm.getAttribute(UrlTruststoreProviderFactory.REALM_ATTR_TRUSTSTORE_URL))
        .thenReturn(truststoreUrl);
    return session;
  }

  /** Creates a session mock with no active realm (outbound HTTPS context). */
  private static KeycloakSession sessionWithNoRealm() {
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakContext ctx = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(ctx);
    when(ctx.getRealm()).thenReturn(null);
    return session;
  }

  @Test
  void rootCertIsClassifiedAsRoot() {
    UrlTruststoreProvider provider =
        UrlTruststoreProviderFactory.fetchAndBuild(
            certUrl("root-ca.pem"), HostnameVerificationPolicy.DEFAULT);

    Map<X500Principal, List<X509Certificate>> roots = provider.getRootCertificates();
    Map<X500Principal, List<X509Certificate>> intermediates =
        provider.getIntermediateCertificates();

    assertFalse(roots.isEmpty(), "Expected at least one root CA");
    assertTrue(intermediates.isEmpty(), "Expected no intermediate CAs for a self-signed cert");
    assertTrue(provider.getTruststore() != null);
  }

  @Test
  void intermediateCertIsClassifiedAsIntermediate() {
    UrlTruststoreProvider provider =
        UrlTruststoreProviderFactory.fetchAndBuild(
            certUrl("intermediate-ca.pem"), HostnameVerificationPolicy.DEFAULT);

    Map<X500Principal, List<X509Certificate>> roots = provider.getRootCertificates();
    Map<X500Principal, List<X509Certificate>> intermediates =
        provider.getIntermediateCertificates();

    assertTrue(roots.isEmpty(), "Expected no root CAs for a CA-signed cert");
    assertFalse(intermediates.isEmpty(), "Expected at least one intermediate CA");
  }

  @Test
  void chainPemClassifiesBothRootAndIntermediate() {
    UrlTruststoreProvider provider =
        UrlTruststoreProviderFactory.fetchAndBuild(
            certUrl("chain.pem"), HostnameVerificationPolicy.DEFAULT);

    Map<X500Principal, List<X509Certificate>> roots = provider.getRootCertificates();
    Map<X500Principal, List<X509Certificate>> intermediates =
        provider.getIntermediateCertificates();

    assertFalse(roots.isEmpty(), "Expected root CA from chain.pem");
    assertFalse(intermediates.isEmpty(), "Expected intermediate CA from chain.pem");
  }

  @Test
  void missingUrlThrows() {
    RuntimeException ex =
        assertThrows(
            RuntimeException.class,
            () ->
                UrlTruststoreProviderFactory.fetchAndBuild(
                    null, HostnameVerificationPolicy.DEFAULT));
    assertNotNull(ex);
  }

  @Test
  void unreachableUrlThrows() {
    RuntimeException ex =
        assertThrows(
            RuntimeException.class,
            () ->
                UrlTruststoreProviderFactory.fetchAndBuild(
                    "https://127.0.0.1:19999/nonexistent.pem",
                    HostnameVerificationPolicy.DEFAULT));
    assertTrue(
        ex.getMessage().contains("Failed to fetch certificates"),
        "Expected fetch error message, got: " + ex.getMessage());
  }

  // --- Realm-aware create() tests ---

  @Test
  void createReturnsRealmSpecificProviderWhenAttributeIsSet() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    // chain.pem contains both root + intermediate; root-ca.pem has only a root.
    KeycloakSession session = sessionWithRealm("realm-1", certUrl("chain.pem"));

    TruststoreProvider realmProvider = factory.create(session);

    assertNotNull(realmProvider);
    // The realm provider was built from chain.pem — it should have both root and intermediate CAs.
    assertFalse(
        realmProvider.getRootCertificates().isEmpty(), "Expected root CA from realm truststore");
    assertFalse(
        realmProvider.getIntermediateCertificates().isEmpty(),
        "Expected intermediate CA from realm truststore");
  }

  @Test
  void createFallsBackToGlobalWhenNoRealmAttribute() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    KeycloakSession sessionWithAttr = sessionWithRealm("realm-1", null);
    KeycloakSession sessionNoRealm = sessionWithNoRealm();

    TruststoreProvider p1 = factory.create(sessionWithAttr);
    TruststoreProvider p2 = factory.create(sessionNoRealm);

    assertSame(p1, p2, "Both should be the global provider when no realm attribute is set");
  }

  @Test
  void createFallsBackToGlobalWhenNoRealmContext() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));

    TruststoreProvider provider = factory.create(sessionWithNoRealm());

    assertNotNull(provider);
    assertFalse(provider.getRootCertificates().isEmpty(), "Global provider should have root CAs");
  }

  @Test
  void createCachesRealmProviderForSameUrl() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    String realmUrl = certUrl("chain.pem");

    TruststoreProvider p1 = factory.create(sessionWithRealm("realm-1", realmUrl));
    TruststoreProvider p2 = factory.create(sessionWithRealm("realm-1", realmUrl));

    assertSame(p1, p2, "Same realm + URL should return the cached provider instance");
  }

  @Test
  void createRefetchesWhenRealmUrlChanges() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));

    TruststoreProvider p1 = factory.create(sessionWithRealm("realm-1", certUrl("root-ca.pem")));
    TruststoreProvider p2 = factory.create(sessionWithRealm("realm-1", certUrl("chain.pem")));

    assertNotSame(p1, p2, "Changed URL should cause a new provider to be loaded");
  }

  @Test
  void differentRealmsGetIndependentProviders() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));

    TruststoreProvider pA = factory.create(sessionWithRealm("realm-a", certUrl("root-ca.pem")));
    TruststoreProvider pB = factory.create(sessionWithRealm("realm-b", certUrl("chain.pem")));

    assertNotSame(pA, pB, "Different realms should get independent provider instances");
    assertTrue(
        pB.getIntermediateCertificates().isEmpty() == false,
        "realm-b uses chain.pem so should have intermediate CAs");
    assertTrue(
        pA.getIntermediateCertificates().isEmpty(),
        "realm-a uses root-ca.pem so should have no intermediate CAs");
  }
}
