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

  private final HostnameVerificationPolicy policy;
  private final SSLSocketFactory sslSocketFactory;
  private final KeyStore truststore;
  private final Map<X500Principal, List<X509Certificate>> rootCertificates;
  private final Map<X500Principal, List<X509Certificate>> intermediateCertificates;

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
