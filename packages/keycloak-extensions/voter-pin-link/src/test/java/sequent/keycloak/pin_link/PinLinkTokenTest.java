// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class PinLinkTokenTest {
  private static JsonNode fixture() throws Exception {
    try (var stream = PinLinkTokenTest.class.getResourceAsStream("/voter-pin-link-v1.json")) {
      return new ObjectMapper().readTree(stream);
    }
  }

  /**
   * Windmill generates the link and Keycloak checks it, so both must derive the same token and
   * lookup hash for the vectors in the shared fixture.
   */
  @Test
  void matchesTheVectorsSharedWithWindmill() throws Exception {
    JsonNode fixture = fixture();
    for (JsonNode vector : fixture.get("vectors")) {
      String token =
          PinLinkToken.derive(
              fixture.get("master_secret_hex").asText(),
              vector.get("tenant").asText(),
              vector.get("event").asText(),
              vector.get("user").asText(),
              vector.get("pin").asText().getBytes(StandardCharsets.UTF_8));
      assertThat(token).isEqualTo(vector.get("token").asText());
      assertThat(PinLinkToken.lookupHash(token)).isEqualTo(vector.get("lookup_hash").asText());
    }
  }

  @Test
  void sameInputKeepsTheSameTokenSoDuplicateMailsKeepTheSameLink() throws Exception {
    byte[] pin = "123456789012".getBytes(StandardCharsets.UTF_8);
    String master = fixture().get("master_secret_hex").asText();
    assertThat(PinLinkToken.derive(master, "t", "e", "u", pin))
        .isEqualTo(PinLinkToken.derive(master, "t", "e", "u", pin));
  }

  @Test
  void isScopedToTenantEventAndVoterAndKeyedByTheMasterSecret() throws Exception {
    byte[] pin = "123456789012".getBytes(StandardCharsets.UTF_8);
    String master = fixture().get("master_secret_hex").asText();
    String other = "ff" + master.substring(2);
    String token = PinLinkToken.derive(master, "t", "e", "u", pin);
    assertThat(PinLinkToken.derive(master, "x", "e", "u", pin)).isNotEqualTo(token);
    assertThat(PinLinkToken.derive(master, "t", "x", "u", pin)).isNotEqualTo(token);
    assertThat(PinLinkToken.derive(master, "t", "e", "x", pin)).isNotEqualTo(token);
    assertThat(PinLinkToken.derive(other, "t", "e", "u", pin)).isNotEqualTo(token);
  }

  @Test
  void hasAtLeast128BitsOfCrockfordBase32() throws Exception {
    String token =
        PinLinkToken.derive(
            fixture().get("master_secret_hex").asText(),
            "t",
            "e",
            "u",
            "123456789012".getBytes(StandardCharsets.UTF_8));
    assertThat(token).hasSize(PinLinkToken.LENGTH).matches("[0-9A-HJKMNP-TV-Z]+");
    assertThat(PinLinkToken.LENGTH * 5).isGreaterThanOrEqualTo(128);
  }

  @Test
  void acceptsATokenTypedInAnyCaseWithSeparatorsAndLookAlikes() {
    String token = "4G89H1GVW3JYW0PR9NGBAHCABP";
    assertThat(PinLinkToken.canonicalize(token)).contains(token);
    assertThat(PinLinkToken.canonicalize(token.toLowerCase())).contains(token);
    assertThat(PinLinkToken.canonicalize("4G89-H1GV-W3JY-W0PR-9NGB-AHCA-BP")).contains(token);
    assertThat(PinLinkToken.canonicalize("4G89HlGVW3JYWOPR9NGBAHCABP")).contains(token);
    assertThat(PinLinkToken.canonicalize("4G89HIGVW3JYWoPR9NGBAHCABP")).contains(token);
  }

  @Test
  void rejectsAnythingElse() {
    assertThat(PinLinkToken.canonicalize(null)).isEmpty();
    assertThat(PinLinkToken.canonicalize("")).isEmpty();
    assertThat(PinLinkToken.canonicalize("4G89H1GVW3JYW0PR9NGBAHCAB")).isEmpty();
    assertThat(PinLinkToken.canonicalize("4G89H1GVW3JYW0PR9NGBAHCABPP")).isEmpty();
    assertThat(PinLinkToken.canonicalize("4G89H1GVW3JYW0PR9NGBAHCABU")).isEmpty();
    assertThat(PinLinkToken.canonicalize("../../../../etc/passwd")).isEmpty();
    assertThat(PinLinkToken.canonicalize("A".repeat(10_000))).isEmpty();
  }
}
