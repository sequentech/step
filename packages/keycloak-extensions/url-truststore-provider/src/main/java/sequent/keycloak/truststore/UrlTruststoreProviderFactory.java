// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.truststore;

import com.google.auto.service.AutoService;
import java.io.IOException;
import java.io.InputStream;
import java.net.URI;
import java.security.InvalidKeyException;
import java.security.KeyStore;
import java.security.KeyStoreException;
import java.security.NoSuchAlgorithmException;
import java.security.NoSuchProviderException;
import java.security.PublicKey;
import java.security.SignatureException;
import java.security.cert.Certificate;
import java.security.cert.CertificateException;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import javax.security.auth.x500.X500Principal;
import org.jboss.logging.Logger;
import org.keycloak.Config;
import org.keycloak.common.enums.HostnameVerificationPolicy;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.truststore.TruststoreProvider;
import org.keycloak.truststore.TruststoreProviderFactory;

/**
 * TruststoreProviderFactory that loads CA certificates from a remote URL (HTTP/HTTPS, S3
 * pre-signed URLs). Supports optional background refresh at a configurable interval.
 *
 * <p>Activate via:
 *
 * <pre>
 *   --spi-truststore-provider=url
 *   --spi-truststore-url-url=https://example.com/client-ca.pem
 *   --spi-truststore-url-refresh-interval-seconds=3600
 * </pre>
 */
@AutoService(TruststoreProviderFactory.class)
public class UrlTruststoreProviderFactory implements TruststoreProviderFactory {

  private static final Logger log = Logger.getLogger(UrlTruststoreProviderFactory.class);
  private static final String PROVIDER_ID = "url";
  private static final String CFG_URL = "url";
  private static final String CFG_REFRESH_INTERVAL = "refresh-interval-seconds";
  private static final String CFG_HOSTNAME_VERIFICATION_POLICY = "hostname-verification-policy";

  private volatile UrlTruststoreProvider provider;
  private ScheduledExecutorService scheduler;
  private String certUrl;
  private HostnameVerificationPolicy policy;

  @Override
  public void init(Config.Scope config) {
    certUrl = config.get(CFG_URL);
    if (certUrl == null || certUrl.isBlank()) {
      throw new RuntimeException(
          "Missing required config 'url' for url TruststoreProvider"
              + " (--spi-truststore-url-url=<https://...>)");
    }

    String policyValue =
        config.get(CFG_HOSTNAME_VERIFICATION_POLICY, HostnameVerificationPolicy.DEFAULT.name());
    try {
      policy = HostnameVerificationPolicy.valueOf(policyValue);
    } catch (IllegalArgumentException e) {
      throw new RuntimeException(
          "Invalid value for '"
              + CFG_HOSTNAME_VERIFICATION_POLICY
              + "': "
              + policyValue
              + " (must be DEFAULT, ANY, or WILDCARD)");
    }

    provider = fetchAndBuild(certUrl, policy);
    log.infof("URL TruststoreProvider initialized from: %s", certUrl);

    long refreshIntervalSeconds = config.getLong(CFG_REFRESH_INTERVAL, 0L);
    if (refreshIntervalSeconds > 0) {
      scheduler = Executors.newSingleThreadScheduledExecutor(r -> {
        Thread t = new Thread(r, "url-truststore-refresh");
        t.setDaemon(true);
        return t;
      });
      scheduler.scheduleAtFixedRate(
          this::refresh, refreshIntervalSeconds, refreshIntervalSeconds, TimeUnit.SECONDS);
      log.infof(
          "URL TruststoreProvider refresh scheduled every %d seconds", refreshIntervalSeconds);
    }
  }

  @Override
  public TruststoreProvider create(KeycloakSession session) {
    return provider;
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {
    if (scheduler != null) {
      scheduler.shutdownNow();
    }
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  private void refresh() {
    try {
      provider = fetchAndBuild(certUrl, policy);
      log.infof("URL TruststoreProvider refreshed from: %s", certUrl);
    } catch (Exception e) {
      log.errorf(e, "Failed to refresh URL TruststoreProvider from: %s", certUrl);
    }
  }

  static UrlTruststoreProvider fetchAndBuild(String url, HostnameVerificationPolicy policy) {
    Collection<? extends Certificate> certs = fetchCertificates(url);

    KeyStore keyStore;
    try {
      keyStore = KeyStore.getInstance("PKCS12");
      keyStore.load(null, null);
    } catch (Exception e) {
      throw new RuntimeException("Failed to create in-memory KeyStore", e);
    }

    int index = 0;
    for (Certificate cert : certs) {
      try {
        keyStore.setCertificateEntry("cert-" + index++, cert);
      } catch (KeyStoreException e) {
        throw new RuntimeException("Failed to add certificate to KeyStore", e);
      }
    }

    Map<X500Principal, List<X509Certificate>> rootCerts = new HashMap<>();
    Map<X500Principal, List<X509Certificate>> intermediateCerts = new HashMap<>();
    classifyCertificates(keyStore, rootCerts, intermediateCerts);

    return new UrlTruststoreProvider(
        keyStore,
        policy,
        Collections.unmodifiableMap(rootCerts),
        Collections.unmodifiableMap(intermediateCerts));
  }

  private static Collection<? extends Certificate> fetchCertificates(String url) {
    try (InputStream stream = URI.create(url).toURL().openStream()) {
      CertificateFactory cf = CertificateFactory.getInstance("X.509");
      Collection<? extends Certificate> certs = cf.generateCertificates(stream);
      if (certs.isEmpty()) {
        throw new RuntimeException("No certificates found at URL: " + url);
      }
      log.debugf("Fetched %d certificate(s) from: %s", certs.size(), url);
      return certs;
    } catch (IOException | CertificateException e) {
      throw new RuntimeException("Failed to fetch certificates from URL: " + url, e);
    }
  }

  private static void classifyCertificates(
      KeyStore keyStore,
      Map<X500Principal, List<X509Certificate>> rootCerts,
      Map<X500Principal, List<X509Certificate>> intermediateCerts) {
    try {
      java.util.Enumeration<String> aliases = keyStore.aliases();
      while (aliases.hasMoreElements()) {
        String alias = aliases.nextElement();
        Certificate cert = keyStore.getCertificate(alias);
        if (!(cert instanceof X509Certificate)) {
          continue;
        }
        X509Certificate x509 = (X509Certificate) cert;
        X500Principal principal = x509.getSubjectX500Principal();
        if (isSelfSigned(x509)) {
          rootCerts.computeIfAbsent(principal, k -> new ArrayList<>()).add(x509);
          log.debugf("Root CA: %s", principal);
        } else {
          intermediateCerts.computeIfAbsent(principal, k -> new ArrayList<>()).add(x509);
          log.debugf("Intermediate CA: %s", principal);
        }
      }
    } catch (KeyStoreException e) {
      throw new RuntimeException("Failed to read KeyStore entries", e);
    }
  }

  private static boolean isSelfSigned(X509Certificate cert) {
    PublicKey key = cert.getPublicKey();
    try {
      cert.verify(key);
      return true;
    } catch (SignatureException | InvalidKeyException e) {
      return false;
    } catch (CertificateException | NoSuchAlgorithmException | NoSuchProviderException e) {
      log.warnf("Could not verify certificate signature for %s: %s",
          cert.getSubjectX500Principal(), e.getMessage());
      return false;
    }
  }
}
