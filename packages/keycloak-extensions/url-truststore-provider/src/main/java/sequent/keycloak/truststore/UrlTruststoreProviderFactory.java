// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.truststore;

import com.google.auto.service.AutoService;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URI;
import java.net.URLConnection;
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
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.function.BiFunction;
import java.util.function.Supplier;
import javax.net.ssl.TrustManager;
import javax.net.ssl.TrustManagerFactory;
import javax.net.ssl.X509TrustManager;
import javax.security.auth.x500.X500Principal;
import org.jboss.logging.Logger;
import org.keycloak.Config;
import org.keycloak.common.enums.HostnameVerificationPolicy;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.truststore.TruststoreProvider;
import org.keycloak.truststore.TruststoreProviderFactory;

/**
 * TruststoreProviderFactory that loads CA certificates from a remote URL (HTTP/HTTPS, S3 pre-signed
 * URLs). Supports optional background refresh at a configurable interval.
 *
 * <p>Per-realm CA certificates are fetched from the harvest service using the {@code
 * HARVEST_DOMAIN} environment variable. The election event id is extracted from the realm name
 * (realms follow the pattern {@code tenant-<tenantId>-event-<electionEventId>}) and the URL is
 * constructed as: {@code
 * http://<HARVEST_DOMAIN>/election-event/<electionEventId>/certificate-authorities/pem}
 *
 * <p>{@code --spi-truststore-url-url} is optional. When omitted, the JVM default truststore
 * (cacerts) is used as the global fallback for sessions without a matching realm CA.
 *
 * <p>Activate via:
 *
 * <pre>
 *   --spi-truststore-provider=url
 *   --spi-truststore-url-refresh-interval-seconds=3600   (optional)
 *   --spi-truststore-url-url=https://example.com/global-ca.pem  (optional)
 *   HARVEST_DOMAIN=harvest:8000
 * </pre>
 */
@AutoService(TruststoreProviderFactory.class)
public class UrlTruststoreProviderFactory implements TruststoreProviderFactory {

  private static final Logger log = Logger.getLogger(UrlTruststoreProviderFactory.class);
  private static final String PROVIDER_ID = "url";
  private static final String CFG_URL = "url";
  private static final String CFG_REFRESH_INTERVAL = "refresh-interval-seconds";
  private static final String CFG_HOSTNAME_VERIFICATION_POLICY = "hostname-verification-policy";
  private static final String MASTER_REALM_NAME = "master";
  private static final String EVENT_REALM_INFIX = "-event-";

  /**
   * Environment variable containing the harvest service domain (host[:port]). Used to construct
   * per-election-event CA certificate URLs as: {@code
   * http://<HARVEST_DOMAIN>/election-event/<electionEventId>/certificate-authorities/pem}
   */
  static final String ENV_HARVEST_DOMAIN = "HARVEST_DOMAIN";

  // package-private for testing — overridden in unit tests to avoid reading real env vars
  Supplier<String> harvestDomainSupplier = () -> System.getenv(ENV_HARVEST_DOMAIN);

  // package-private for testing — overridden to redirect URL construction to local test resources
  BiFunction<String, String, String> realmUrlBuilder =
      (domain, electionEventId) ->
          "http://"
              + domain
              + "/election-event/"
              + electionEventId
              + "/certificate-authorities/pem";

  private record RealmTruststoreEntry(String url, UrlTruststoreProvider provider) {}

  private volatile UrlTruststoreProvider provider;
  private final ConcurrentHashMap<String, RealmTruststoreEntry> realmCache =
      new ConcurrentHashMap<>();
  private ScheduledExecutorService scheduler;
  private String certUrl;
  private HostnameVerificationPolicy policy;

  @Override
  public void init(Config.Scope config) {
    certUrl = config.get(CFG_URL);

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

    if (certUrl != null && !certUrl.isBlank()) {
      provider = fetchAndBuild(certUrl, policy);
      log.infof("URL TruststoreProvider global truststore initialized from: %s", certUrl);
    } else {
      provider = buildFromJvmTruststore(policy);
      log.infof("URL TruststoreProvider global truststore initialized from JVM default (cacerts)");
    }

    long refreshIntervalSeconds = config.getLong(CFG_REFRESH_INTERVAL, 0L);
    if (refreshIntervalSeconds > 0) {
      scheduler =
          Executors.newSingleThreadScheduledExecutor(
              r -> {
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
    KeycloakContext ctx = session.getContext();
    if (ctx != null) {
      RealmModel realm = ctx.getRealm();
      if (realm != null) {
        String harvestDomain = harvestDomainSupplier.get();
        log.debugf("ENV %s = %s", ENV_HARVEST_DOMAIN, harvestDomain);
        if (harvestDomain != null && !harvestDomain.isBlank()) {
          String realmName = realm.getName();
          if (MASTER_REALM_NAME.equals(realmName)) {
            log.debugf(
                "Realm '%s' detected in create() — skipping per-realm CA lookup",
                MASTER_REALM_NAME);
            return provider;
          }
          int eventIdx = realmName.indexOf(EVENT_REALM_INFIX);
          if (eventIdx < 0) {
            log.debugf(
                "Realm '%s' is not an event realm — skipping per-realm CA lookup", realmName);
            return provider;
          }
          String electionEventId = realmName.substring(eventIdx + EVENT_REALM_INFIX.length());
          String realmUrl = realmUrlBuilder.apply(harvestDomain, electionEventId);
          log.debugf(
              "Constructed realm-specific truststore URL for realm '%s'"
                  + " (election event '%s'): %s",
              realmName, electionEventId, realmUrl);
          return realmProviderFor(realm.getId(), realmName, realmUrl);
        }
      }
    }
    log.debugf("Using global truststore for session (no realm-specific CA)");
    return provider;
  }

  /**
   * Returns a cached realm-specific provider, re-fetching if the constructed URL differs from what
   * was last loaded. Concurrent refreshes for the same realm are serialized.
   */
  private UrlTruststoreProvider realmProviderFor(
      String realmId, String realmName, String realmUrl) {
    if (MASTER_REALM_NAME.equals(realmName)) {
      log.debugf("Realm 'master' detected in realmProviderFor() — returning global truststore");
      return provider;
    }
    RealmTruststoreEntry cached = realmCache.get(realmId);
    if (cached != null && cached.url().equals(realmUrl)) {
      // null provider is a sentinel meaning "no cert found, use global"
      log.infof("Using cached realm-specific truststore for realm %s from: %s", realmId, realmUrl);
      return cached.provider() != null ? cached.provider() : provider;
    }
    // Stale or first load — serialize concurrent refreshes for this realm.
    RealmTruststoreEntry[] result = {null};
    realmCache.compute(
        realmId,
        (id, existing) -> {
          if (existing != null && existing.url().equals(realmUrl)) {
            log.infof(
                "Realm-specific truststore for realm %s already loaded from: %s", id, realmUrl);
            result[0] = existing;
            return existing;
          }
          log.infof("Loading realm-specific truststore for realm %s from: %s", id, realmUrl);
          UrlTruststoreProvider fresh;
          try {
            fresh = fetchAndBuild(realmUrl, policy);
          } catch (RuntimeException e) {
            if (e.getCause() instanceof FileNotFoundException) {
              // 404: no CA certificate exists for this realm. Cache a sentinel to avoid
              // hammering harvest on every request. The refresh cycle will retry periodically.
              log.warnf(
                  "No realm-specific CA found for realm %s at %s — caching absent, falling back"
                      + " to global truststore.",
                  id, realmUrl);
              RealmTruststoreEntry sentinel = new RealmTruststoreEntry(realmUrl, null);
              result[0] = sentinel;
              return sentinel;
            }
            // Transient error (timeout, network issue, etc.) — do not cache so the next
            // request or refresh cycle can retry.
            log.warnf(
                "Transient error fetching realm CA for realm %s from %s — using global truststore"
                    + " for this request. Cause: %s",
                id, realmUrl, e.getMessage());
            result[0] = existing; // keep existing entry if present
            return existing;
          }
          RealmTruststoreEntry entry = new RealmTruststoreEntry(realmUrl, fresh);
          result[0] = entry;
          return entry;
        });
    // result[0] is null when a transient error occurred and no previous entry existed.
    return result[0] != null && result[0].provider() != null ? result[0].provider() : provider;
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {
    if (scheduler != null) {
      scheduler.shutdownNow();
    }
    realmCache.clear();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  private void refresh() {
    if (certUrl != null && !certUrl.isBlank()) {
      try {
        provider = fetchAndBuild(certUrl, policy);
        log.infof("URL TruststoreProvider refreshed from: %s", certUrl);
      } catch (Exception e) {
        log.errorf(e, "Failed to refresh URL TruststoreProvider from: %s", certUrl);
      }
    }
    for (Map.Entry<String, RealmTruststoreEntry> entry : realmCache.entrySet()) {
      String realmId = entry.getKey();
      RealmTruststoreEntry current = entry.getValue();
      try {
        UrlTruststoreProvider fresh = fetchAndBuild(current.url(), policy);
        realmCache.put(realmId, new RealmTruststoreEntry(current.url(), fresh));
        if (current.provider() == null) {
          log.infof("Realm CA now available for realm %s from: %s", realmId, current.url());
        } else {
          log.infof("Realm truststore refreshed for realm %s from: %s", realmId, current.url());
        }
      } catch (RuntimeException e) {
        if (e.getCause() instanceof FileNotFoundException) {
          // Still absent — keep sentinel and wait for next refresh cycle.
          log.debugf(
              "Realm CA still absent for realm %s at: %s — keeping sentinel.",
              realmId, current.url());
        } else {
          // Transient error — keep current entry and retry next cycle.
          log.errorf(
              e,
              "Failed to refresh realm truststore for realm %s from: %s",
              realmId,
              current.url());
        }
      }
    }
  }

  static UrlTruststoreProvider buildFromJvmTruststore(HostnameVerificationPolicy policy) {
    try {
      TrustManagerFactory tmf =
          TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
      tmf.init((KeyStore) null); // null = use JVM default truststore (cacerts)

      KeyStore keyStore = KeyStore.getInstance("PKCS12");
      keyStore.load(null, null);
      int index = 0;
      for (TrustManager tm : tmf.getTrustManagers()) {
        if (tm instanceof X509TrustManager) {
          for (X509Certificate cert : ((X509TrustManager) tm).getAcceptedIssuers()) {
            keyStore.setCertificateEntry("jvm-" + index++, cert);
          }
        }
      }
      log.debugf("Loaded %d certificate(s) from JVM default truststore", index);

      Map<X500Principal, List<X509Certificate>> rootCerts = new HashMap<>();
      Map<X500Principal, List<X509Certificate>> intermediateCerts = new HashMap<>();
      classifyCertificates(keyStore, rootCerts, intermediateCerts);

      return new UrlTruststoreProvider(
          keyStore,
          policy,
          Collections.unmodifiableMap(rootCerts),
          Collections.unmodifiableMap(intermediateCerts));
    } catch (Exception e) {
      throw new RuntimeException("Failed to load JVM default truststore", e);
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

  private static final int FETCH_CONNECT_TIMEOUT_MS = 5_000;
  private static final int FETCH_READ_TIMEOUT_MS = 10_000;

  private static Collection<? extends Certificate> fetchCertificates(String url) {
    URLConnection connection;
    try {
      connection = URI.create(url).toURL().openConnection();
      connection.setConnectTimeout(FETCH_CONNECT_TIMEOUT_MS);
      connection.setReadTimeout(FETCH_READ_TIMEOUT_MS);
      connection.connect();
    } catch (IOException e) {
      throw new RuntimeException("Failed to connect to certificate URL: " + url, e);
    }
    try (InputStream stream = connection.getInputStream()) {
      CertificateFactory cf = CertificateFactory.getInstance("X.509");
      Collection<? extends Certificate> certs = cf.generateCertificates(stream);
      if (certs.isEmpty()) {
        log.warnf("No certificates found at URL: %s", url);
        return Collections.emptyList();
      }
      log.debugf("Fetched %d certificate(s) from: %s", certs.size(), url);
      return certs;
    } catch (IOException | CertificateException e) {
      throw new RuntimeException("Failed to fetch certificates from URL: " + url, e);
    } finally {
      if (connection instanceof HttpURLConnection http) {
        http.disconnect();
      }
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
      log.warnf(
          "Could not verify certificate signature for %s: %s",
          cert.getSubjectX500Principal(), e.getMessage());
      return false;
    }
  }
}
