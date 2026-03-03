// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.truststore;

import static org.junit.jupiter.api.Assertions.*;

import java.net.URL;
import java.security.cert.X509Certificate;
import java.util.List;
import java.util.Map;
import javax.security.auth.x500.X500Principal;
import org.junit.jupiter.api.Test;
import org.keycloak.common.enums.HostnameVerificationPolicy;

class UrlTruststoreProviderFactoryTest {

  private static String certUrl(String filename) {
    URL resource =
        UrlTruststoreProviderFactoryTest.class.getClassLoader().getResource("certs/" + filename);
    assertNotNull(resource, "Test resource not found: certs/" + filename);
    return resource.toString();
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
}
