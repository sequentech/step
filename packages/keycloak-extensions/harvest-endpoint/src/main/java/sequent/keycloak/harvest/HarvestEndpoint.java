// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.harvest;

import java.net.URI;
import java.net.URISyntaxException;
import java.util.Arrays;
import java.util.Locale;
import java.util.Optional;
import java.util.function.UnaryOperator;

/**
 * Base URL of the harvest service used by the Keycloak extensions.
 *
 * <p>{@code HARVEST_URL} (for example {@code https://harvest:8400}) takes precedence. When it is
 * not set, the base URL is {@code http://<HARVEST_DOMAIN>}. {@code HARVEST_TLS_POLICY} (a {@link
 * HarvestTlsPolicy}, {@code PLAINTEXT_ALLOWED} by default) decides whether a plain {@code http}
 * base URL is accepted.
 */
public final class HarvestEndpoint {

  public static final String ENV_HARVEST_URL = "HARVEST_URL";
  public static final String ENV_HARVEST_DOMAIN = "HARVEST_DOMAIN";
  public static final String ENV_HARVEST_TLS_POLICY = "HARVEST_TLS_POLICY";
  public static final HarvestTlsPolicy DEFAULT_TLS_POLICY = HarvestTlsPolicy.PLAINTEXT_ALLOWED;

  private static final String HTTP_SCHEME = "http";
  private static final String HTTPS_SCHEME = "https";
  private static final String SCHEME_SEPARATOR = "://";
  private static final String PATH_SEPARATOR = "/";

  private final String baseUrl;

  private HarvestEndpoint(String baseUrl) {
    this.baseUrl = baseUrl;
  }

  /**
   * Resolves the harvest base URL from the given environment lookup.
   *
   * @return empty when neither {@code HARVEST_URL} nor {@code HARVEST_DOMAIN} is set
   * @throws IllegalStateException when the policy or the URL is invalid, or the policy requires TLS
   *     and the URL is not {@code https}
   */
  public static Optional<HarvestEndpoint> fromEnvironment(UnaryOperator<String> environment) {
    HarvestTlsPolicy policy = parseTlsPolicy(environment.apply(ENV_HARVEST_TLS_POLICY));

    String baseUrl;
    String harvestUrl = environment.apply(ENV_HARVEST_URL);
    if (!isBlank(harvestUrl)) {
      baseUrl = harvestUrl.trim();
    } else {
      String harvestDomain = environment.apply(ENV_HARVEST_DOMAIN);
      if (isBlank(harvestDomain)) {
        return Optional.empty();
      }
      baseUrl = HTTP_SCHEME + SCHEME_SEPARATOR + harvestDomain.trim();
    }
    while (baseUrl.endsWith(PATH_SEPARATOR)) {
      baseUrl = baseUrl.substring(0, baseUrl.length() - 1);
    }

    String scheme = parseScheme(baseUrl);
    if (policy == HarvestTlsPolicy.REQUIRE_TLS && !HTTPS_SCHEME.equals(scheme)) {
      throw new IllegalStateException(
          ENV_HARVEST_TLS_POLICY
              + "="
              + policy.name()
              + " requires an "
              + HTTPS_SCHEME
              + " "
              + ENV_HARVEST_URL
              + ", got: "
              + baseUrl);
    }
    return Optional.of(new HarvestEndpoint(baseUrl));
  }

  /** Returns the absolute URL of the given harvest path, which must start with {@code /}. */
  public String url(String path) {
    return baseUrl + path;
  }

  private static HarvestTlsPolicy parseTlsPolicy(String value) {
    if (isBlank(value)) {
      return DEFAULT_TLS_POLICY;
    }
    try {
      return HarvestTlsPolicy.valueOf(value.trim());
    } catch (IllegalArgumentException e) {
      throw new IllegalStateException(
          "Invalid value for "
              + ENV_HARVEST_TLS_POLICY
              + ": "
              + value
              + " (must be one of "
              + Arrays.toString(HarvestTlsPolicy.values())
              + ")",
          e);
    }
  }

  private static String parseScheme(String baseUrl) {
    URI uri;
    try {
      uri = new URI(baseUrl);
    } catch (URISyntaxException e) {
      throw new IllegalStateException("Invalid harvest URL: " + baseUrl, e);
    }
    String scheme = uri.getScheme() == null ? "" : uri.getScheme().toLowerCase(Locale.ROOT);
    if (!HTTP_SCHEME.equals(scheme) && !HTTPS_SCHEME.equals(scheme)) {
      throw new IllegalStateException(
          "Harvest URL must use " + HTTP_SCHEME + " or " + HTTPS_SCHEME + ": " + baseUrl);
    }
    if (uri.getHost() == null) {
      throw new IllegalStateException("Harvest URL has no host: " + baseUrl);
    }
    return scheme;
  }

  private static boolean isBlank(String value) {
    return value == null || value.isBlank();
  }
}
