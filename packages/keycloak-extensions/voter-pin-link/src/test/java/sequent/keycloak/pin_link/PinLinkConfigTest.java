// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.time.Instant;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class PinLinkConfigTest {
  private static Map<String, String> enabled() {
    Map<String, String> attributes = new HashMap<>();
    attributes.put(PinLinkConfig.POLICY, "ONE_TIME_LINK");
    attributes.put(PinLinkConfig.EXPIRES_AT, "2030-01-01T00:00:00Z");
    return attributes;
  }

  /**
   * The fixture lists the expiry values Windmill's tests also accept and refuse. If the two read
   * one differently, voters could be mailed links that never work.
   */
  @Test
  void readsTheExpiryLikeWindmill() throws Exception {
    JsonNode vectors;
    try (var stream = PinLinkConfigTest.class.getResourceAsStream("/voter-pin-link-v1.json")) {
      vectors = new ObjectMapper().readTree(stream).get("expires_at");
    }
    for (JsonNode value : vectors.get("valid")) {
      Map<String, String> attributes = enabled();
      attributes.put(PinLinkConfig.EXPIRES_AT, value.asText());
      assertThat(PinLinkConfig.from(attributes)).as(value.asText()).isPresent();
    }
    for (JsonNode value : vectors.get("invalid")) {
      Map<String, String> attributes = enabled();
      attributes.put(PinLinkConfig.EXPIRES_AT, value.asText());
      assertThat(PinLinkConfig.from(attributes)).as(value.asText()).isEmpty();
    }
  }

  @Test
  void allowsNoOtherHostByDefault() {
    assertThat(PinLinkConfig.assetOrigins(Map.<String, String>of()::get)).isEmpty();
    assertThat(PinLinkConfig.assetOrigins(Map.of(PinLinkConfig.ASSET_ORIGINS, " \n")::get))
        .isEmpty();
  }

  @Test
  void readsAssetOriginsSeparatedBySpacesOrCommas() {
    Map<String, String> attributes =
        Map.of(
            PinLinkConfig.ASSET_ORIGINS,
            "https://a.example.org, https://b.example.org:8443/\nhttps://a.example.org");
    assertThat(PinLinkConfig.assetOrigins(attributes::get))
        .containsExactly("https://a.example.org", "https://b.example.org:8443");
  }

  @Test
  void leavesOutWhatIsNotAnHttpsOrigin() {
    Map<String, String> attributes =
        Map.of(
            PinLinkConfig.ASSET_ORIGINS,
            "* http://a.example.org https://*.example.org https://a.example.org/logo.png"
                + " https://user@a.example.org 'self' data: https://b.example.org;script-src"
                + " https://c.example.org");
    assertThat(PinLinkConfig.assetOrigins(attributes::get))
        .containsExactly("https://c.example.org");
  }

  /**
   * Windmill doesn't read this attribute, so voters may already have their links when it is wrong.
   */
  @Test
  void aBadAssetOriginLeavesTheLinkOn() {
    Map<String, String> attributes = enabled();
    attributes.put(PinLinkConfig.ASSET_ORIGINS, "not an origin");
    assertThat(PinLinkConfig.from(attributes)).isPresent();
  }

  @Test
  void isOffByDefault() {
    assertThat(PinLinkConfig.from(Map.of())).isEmpty();
  }

  /**
   * Only the exact value {@code ONE_TIME_LINK} turns the link on; a different case or {@code true}
   * leaves it off.
   */
  @Test
  void isOffForAnyOtherPolicyValue() {
    for (String value : new String[] {"DISABLED", "one_time_link", "true", ""}) {
      Map<String, String> attributes = enabled();
      attributes.put(PinLinkConfig.POLICY, value);
      assertThat(PinLinkConfig.from(attributes)).isEmpty();
    }
  }

  @Test
  void usesTheDefaultsWhenOnlyPolicyAndExpiryAreSet() {
    PinLinkConfig config = PinLinkConfig.from(enabled()).orElseThrow();
    assertThat(config.pinAttribute()).isEqualTo("pin");
    assertThat(config.tokenHashAttribute()).isEqualTo("pin-link-token-hash");
    assertThat(config.attemptsAttribute()).isEqualTo("pin-link-attempts");
    assertThat(config.maxAttempts()).isEqualTo(5);
  }

  @Test
  void honoursConfiguredNamesAndLimits() {
    Map<String, String> attributes = enabled();
    attributes.put(PinLinkConfig.PIN_ATTRIBUTE, "member-pin");
    attributes.put(PinLinkConfig.TOKEN_HASH_ATTRIBUTE, "member-hash");
    attributes.put(PinLinkConfig.ATTEMPTS_ATTRIBUTE, "member-attempts");
    attributes.put(PinLinkConfig.MAX_ATTEMPTS, "3");
    PinLinkConfig config = PinLinkConfig.from(attributes).orElseThrow();
    assertThat(config.pinAttribute()).isEqualTo("member-pin");
    assertThat(config.tokenHashAttribute()).isEqualTo("member-hash");
    assertThat(config.attemptsAttribute()).isEqualTo("member-attempts");
    assertThat(config.maxAttempts()).isEqualTo(3);
  }

  /**
   * A missing expiry, or an invalid expiry, attempt limit or attribute name, turns the link off
   * instead of falling back to a default.
   */
  @Test
  void failsClosedOnInvalidValues() {
    String[][] invalid = {
      {PinLinkConfig.EXPIRES_AT, null},
      {PinLinkConfig.EXPIRES_AT, "tomorrow"},
      {PinLinkConfig.MAX_ATTEMPTS, "0"},
      {PinLinkConfig.MAX_ATTEMPTS, "-1"},
      {PinLinkConfig.MAX_ATTEMPTS, "many"},
      {PinLinkConfig.MAX_ATTEMPTS, "1000"},
      {PinLinkConfig.PIN_ATTRIBUTE, "bad name"},
      {PinLinkConfig.PIN_ATTRIBUTE, "pin-link-token-hash"},
      {PinLinkConfig.ATTEMPTS_ATTRIBUTE, "pin-link-token-hash"},
    };
    for (String[] entry : invalid) {
      Map<String, String> attributes = enabled();
      attributes.put(entry[0], entry[1]);
      assertThat(PinLinkConfig.from(attributes)).as(entry[0] + "=" + entry[1]).isEmpty();
    }
  }

  @Test
  void expiresExactlyAtTheConfiguredInstant() {
    PinLinkConfig config = PinLinkConfig.from(enabled()).orElseThrow();
    Instant expiry = Instant.parse("2030-01-01T00:00:00Z");
    assertThat(config.isExpired(expiry.minusMillis(1))).isFalse();
    assertThat(config.isExpired(expiry)).isTrue();
  }
}
