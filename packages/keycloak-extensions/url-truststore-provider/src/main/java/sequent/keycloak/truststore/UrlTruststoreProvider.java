// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.truststore;

import java.security.KeyStore;
import java.security.cert.X509Certificate;
import java.util.List;
import java.util.Map;
import javax.net.ssl.SSLSocketFactory;
import javax.security.auth.x500.X500Principal;
import org.keycloak.common.enums.HostnameVerificationPolicy;
import org.keycloak.truststore.JSSETruststoreConfigurator;
import org.keycloak.truststore.TruststoreProvider;

/** TruststoreProvider backed by CA certificates fetched from a URL. */
public class UrlTruststoreProvider implements TruststoreProvider {

  /** Hostname matching policy used for connections through this truststore. */
  private final HostnameVerificationPolicy policy;

  /** Socket factory derived from these certificates, or the JVM default if unavailable. */
  private final SSLSocketFactory sslSocketFactory;

  /** In-memory CA entries supplied to Keycloak. */
  private final KeyStore truststore;

  /** Self-signed roots indexed by subject principal. */
  private final Map<X500Principal, List<X509Certificate>> rootCertificates;

  /** Intermediate issuers indexed by subject principal. */
  private final Map<X500Principal, List<X509Certificate>> intermediateCertificates;

  /**
   * Creates a provider from a fully built truststore and its classified certificates.
   *
   * @param truststore the CA entries to trust
   * @param policy hostname matching policy
   * @param rootCertificates immutable root certificate index
   * @param intermediateCertificates immutable intermediate certificate index
   */
  public UrlTruststoreProvider(
      KeyStore truststore,
      HostnameVerificationPolicy policy,
      Map<X500Principal, List<X509Certificate>> rootCertificates,
      Map<X500Principal, List<X509Certificate>> intermediateCertificates) {
    this.policy = policy;
    this.truststore = truststore;
    this.rootCertificates = rootCertificates;
    this.intermediateCertificates = intermediateCertificates;

    SSLSocketFactory jsseFactory = new JSSETruststoreConfigurator(this).getSSLSocketFactory();
    this.sslSocketFactory =
        jsseFactory != null ? jsseFactory : (SSLSocketFactory) SSLSocketFactory.getDefault();
  }

  @Override
  public HostnameVerificationPolicy getPolicy() {
    return policy;
  }

  @Override
  public SSLSocketFactory getSSLSocketFactory() {
    return sslSocketFactory;
  }

  @Override
  public KeyStore getTruststore() {
    return truststore;
  }

  @Override
  public Map<X500Principal, List<X509Certificate>> getRootCertificates() {
    return rootCertificates;
  }

  @Override
  public Map<X500Principal, List<X509Certificate>> getIntermediateCertificates() {
    return intermediateCertificates;
  }

  @Override
  public void close() {}
}
