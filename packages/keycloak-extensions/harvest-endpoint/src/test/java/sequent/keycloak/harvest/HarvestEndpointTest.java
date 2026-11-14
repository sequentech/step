// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.harvest;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class HarvestEndpointTest {

  private static final String PATH = "/verify-application";

  private static Optional<HarvestEndpoint> resolve(Map<String, String> environment) {
    return HarvestEndpoint.fromEnvironment(environment::get);
  }

  @Test
  void usesHarvestUrlWhenSet() {
    Optional<HarvestEndpoint> endpoint =
        resolve(
            Map.of(
                HarvestEndpoint.ENV_HARVEST_URL, "https://harvest.internal:8443/",
                HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400"));

    assertEquals("https://harvest.internal:8443" + PATH, endpoint.orElseThrow().url(PATH));
  }

  @Test
  void fallsBackToPlainHttpHarvestDomain() {
    Optional<HarvestEndpoint> endpoint =
        resolve(Map.of(HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400"));

    assertEquals("http://harvest:8400" + PATH, endpoint.orElseThrow().url(PATH));
  }

  @Test
  void isEmptyWhenNeitherUrlNorDomainIsSet() {
    assertTrue(resolve(Map.of()).isEmpty());
    assertTrue(
        resolve(
                Map.of(
                    HarvestEndpoint.ENV_HARVEST_URL, " ",
                    HarvestEndpoint.ENV_HARVEST_DOMAIN, ""))
            .isEmpty());
  }

  @Test
  void requireTlsAcceptsHttpsHarvestUrl() {
    Optional<HarvestEndpoint> endpoint =
        resolve(
            Map.of(
                HarvestEndpoint.ENV_HARVEST_URL,
                "https://harvest:8400",
                HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
                HarvestTlsPolicy.REQUIRE_TLS.name()));

    assertEquals("https://harvest:8400" + PATH, endpoint.orElseThrow().url(PATH));
  }

  @Test
  void requireTlsRejectsPlainHttpHarvestUrl() {
    assertThrows(
        IllegalStateException.class,
        () ->
            resolve(
                Map.of(
                    HarvestEndpoint.ENV_HARVEST_URL,
                    "http://harvest:8400",
                    HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
                    HarvestTlsPolicy.REQUIRE_TLS.name())));
  }

  @Test
  void requireTlsRejectsHarvestDomainFallback() {
    assertThrows(
        IllegalStateException.class,
        () ->
            resolve(
                Map.of(
                    HarvestEndpoint.ENV_HARVEST_DOMAIN,
                    "harvest:8400",
                    HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
                    HarvestTlsPolicy.REQUIRE_TLS.name())));
  }

  @Test
  void plaintextAllowedAcceptsPlainHttpHarvestUrl() {
    Optional<HarvestEndpoint> endpoint =
        resolve(
            Map.of(
                HarvestEndpoint.ENV_HARVEST_URL,
                "http://harvest:8400",
                HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
                HarvestTlsPolicy.PLAINTEXT_ALLOWED.name()));

    assertEquals("http://harvest:8400" + PATH, endpoint.orElseThrow().url(PATH));
  }

  @Test
  void rejectsUnknownTlsPolicy() {
    assertThrows(
        IllegalStateException.class,
        () ->
            resolve(
                Map.of(
                    HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400",
                    HarvestEndpoint.ENV_HARVEST_TLS_POLICY, "require-tls")));
  }

  @Test
  void rejectsHarvestUrlWithUnsupportedScheme() {
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "ftp://harvest:8400")));
  }

  /** Paths are appended to the base URL, so a query or fragment there would swallow them. */
  @Test
  void rejectsHarvestUrlWithQueryOrFragment() {
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "https://harvest:8400?target=x")));
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "https://harvest:8400#section")));
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "https://harvest:8400/api?")));
  }

  /** A base URL with a path prefix keeps it, without a trailing slash, before the appended path. */
  @Test
  void keepsHarvestUrlBasePath() {
    Optional<HarvestEndpoint> endpoint =
        resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "https://gateway:8443/harvest/"));

    assertEquals("https://gateway:8443/harvest" + PATH, endpoint.orElseThrow().url(PATH));
  }

  @Test
  void rejectsHarvestUrlWithoutHost() {
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "harvest:8400")));
    assertThrows(
        IllegalStateException.class,
        () -> resolve(Map.of(HarvestEndpoint.ENV_HARVEST_URL, "https:///election-event")));
  }
}
