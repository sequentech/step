// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;

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

  @Test
  void isOffByDefault() {
    assertThat(PinLinkConfig.from(Map.of())).isEmpty();
  }

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
