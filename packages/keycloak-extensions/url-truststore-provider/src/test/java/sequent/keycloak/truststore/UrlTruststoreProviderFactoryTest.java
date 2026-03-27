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

  /** Returns the base URL for the test certs directory (with trailing slash). */
  private static String certsBaseUrl() {
    String url = certUrl("root-ca.pem");
    return url.substring(0, url.lastIndexOf('/') + 1);
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

  /** Creates and initialises a factory with no global URL (falls back to JVM truststore). */
  private static UrlTruststoreProviderFactory initFactoryNoGlobalUrl() {
    Config.Scope config = mock(Config.Scope.class);
    when(config.get("url")).thenReturn(null);
    when(config.get("hostname-verification-policy", "DEFAULT")).thenReturn("DEFAULT");
    when(config.getLong("refresh-interval-seconds", 0L)).thenReturn(0L);
    UrlTruststoreProviderFactory factory = new UrlTruststoreProviderFactory();
    factory.init(config);
    return factory;
  }

  /**
   * Creates a session mock whose realm returns the given realm name via {@code realm.getName()}.
   * The factory constructs the truststore URL via {@code realmUrlBuilder}.
   */
  private static KeycloakSession sessionWithRealm(String realmId, String realmName) {
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakContext ctx = mock(KeycloakContext.class);
    RealmModel realm = mock(RealmModel.class);
    when(session.getContext()).thenReturn(ctx);
    when(ctx.getRealm()).thenReturn(realm);
    when(realm.getId()).thenReturn(realmId);
    when(realm.getName()).thenReturn(realmName);
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
  void noGlobalUrlFallsBackToJvmTruststore() {
    UrlTruststoreProviderFactory factory = initFactoryNoGlobalUrl();

    // JVM cacerts contains well-known public CAs — should not be empty.
    TruststoreProvider globalProvider = factory.create(sessionWithNoRealm());

    assertNotNull(globalProvider);
    assertFalse(
        globalProvider.getRootCertificates().isEmpty(),
        "JVM truststore should contain public root CAs");
  }

  @Test
  void noGlobalUrlRealmCertNotFoundFallsBackToJvmTruststore() {
    UrlTruststoreProviderFactory factory = initFactoryNoGlobalUrl();
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";
    // "master" exits early — should fall back to JVM truststore.
    KeycloakSession masterSession = sessionWithRealm("master-id", "master");

    TruststoreProvider result = factory.create(masterSession);

    assertNotNull(result);
    assertFalse(
        result.getRootCertificates().isEmpty(),
        "Fallback to JVM truststore should contain public root CAs");
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
                    "https://127.0.0.1:19999/nonexistent.pem", HostnameVerificationPolicy.DEFAULT));
    assertTrue(
        ex.getMessage().contains("Failed to connect to certificate URL")
            || ex.getMessage().contains("Failed to fetch certificates"),
        "Expected fetch error message, got: " + ex.getMessage());
  }

  // --- Realm-aware create() tests ---

  @Test
  void createReturnsRealmSpecificProviderForRealm() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    // election event id "realm-chain" resolves to client-ca-realm-chain.pem (root + intermediate).
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";
    KeycloakSession session = sessionWithRealm("realm-1", "tenant-test-event-realm-chain");

    TruststoreProvider realmProvider = factory.create(session);

    assertNotNull(realmProvider);
    assertFalse(
        realmProvider.getRootCertificates().isEmpty(), "Expected root CA from realm truststore");
    assertFalse(
        realmProvider.getIntermediateCertificates().isEmpty(),
        "Expected intermediate CA from realm truststore");
  }

  @Test
  void createFallsBackToGlobalWhenEnvVarNotSet() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> null; // env var not configured
    KeycloakSession sessionWithRealm = sessionWithRealm("realm-1", "realm-root");
    KeycloakSession sessionNoRealm = sessionWithNoRealm();

    TruststoreProvider p1 = factory.create(sessionWithRealm);
    TruststoreProvider p2 = factory.create(sessionNoRealm);

    assertSame(p1, p2, "Both should be the global provider when env var is not set");
  }

  @Test
  void createFallsBackToGlobalForNonEventRealm() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";
    // Tenant-only realm names (no "-event-" infix) should fall back to global.
    KeycloakSession tenantSession = sessionWithRealm("tenant-id", "tenant-test");

    TruststoreProvider result = factory.create(tenantSession);

    assertNotNull(result);
    assertFalse(result.getRootCertificates().isEmpty(), "Should have global root CAs");
    assertTrue(
        result.getIntermediateCertificates().isEmpty(), "Global provider has no intermediates");
  }

  @Test
  void createFallsBackToGlobalWhenNoRealmContext() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";

    TruststoreProvider provider = factory.create(sessionWithNoRealm());

    assertNotNull(provider);
    assertFalse(provider.getRootCertificates().isEmpty(), "Global provider should have root CAs");
  }

  @Test
  void createCachesRealmProvider() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";

    TruststoreProvider p1 =
        factory.create(sessionWithRealm("realm-1", "tenant-test-event-realm-root"));
    TruststoreProvider p2 =
        factory.create(sessionWithRealm("realm-1", "tenant-test-event-realm-root"));

    assertSame(p1, p2, "Same realm should return the cached provider instance");
  }

  @Test
  void createFallsBackToGlobalWhenRealmCertNotFound() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";
    // "master" exits early — should silently use global provider.
    KeycloakSession masterSession = sessionWithRealm("master-id", "master");

    TruststoreProvider result = factory.create(masterSession);

    assertNotNull(result);
    // Should be the global provider (built from root-ca.pem), not an error.
    assertFalse(result.getRootCertificates().isEmpty(), "Should have global root CAs");
    assertTrue(
        result.getIntermediateCertificates().isEmpty(), "Global provider has no intermediates");
  }

  @Test
  void createCachesNotFoundSentinelToAvoidRetry() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";
    // Use an event realm whose cert file does not exist — should cache a sentinel.
    KeycloakSession masterSession1 =
        sessionWithRealm("missing-id", "tenant-test-event-nonexistent");
    KeycloakSession masterSession2 =
        sessionWithRealm("missing-id", "tenant-test-event-nonexistent");

    TruststoreProvider p1 = factory.create(masterSession1);
    TruststoreProvider p2 = factory.create(masterSession2);

    // Both calls return the same global provider instance (sentinel is cached, no retry).
    assertSame(p1, p2, "Missing-cert realms should return the same cached global provider");
  }

  @Test
  void differentRealmsGetIndependentProviders() {
    UrlTruststoreProviderFactory factory = initFactory(certUrl("root-ca.pem"));
    factory.harvestDomainSupplier = () -> "test-harvest";
    factory.realmUrlBuilder =
        (domain, electionEventId) -> certsBaseUrl() + "client-ca-" + electionEventId + ".pem";

    // realm-a → client-ca-realm-root.pem (root only)
    // realm-b → client-ca-realm-chain.pem (root + intermediate)
    TruststoreProvider pA =
        factory.create(sessionWithRealm("realm-a", "tenant-test-event-realm-root"));
    TruststoreProvider pB =
        factory.create(sessionWithRealm("realm-b", "tenant-test-event-realm-chain"));

    assertNotSame(pA, pB, "Different realms should get independent provider instances");
    assertFalse(
        pB.getIntermediateCertificates().isEmpty(),
        "realm-b uses realm-chain so should have intermediate CAs");
    assertTrue(
        pA.getIntermediateCertificates().isEmpty(),
        "realm-a uses realm-root so should have no intermediate CAs");
  }
}
