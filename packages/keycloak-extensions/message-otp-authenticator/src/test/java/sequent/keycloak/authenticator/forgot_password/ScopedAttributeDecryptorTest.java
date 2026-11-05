// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

class ScopedAttributeDecryptorTest {
  private static JsonNode fixture;

  @BeforeAll
  static void readFixture() throws Exception {
    try (var stream =
        ScopedAttributeDecryptorTest.class.getResourceAsStream("/voter-secret-v1.json")) {
      fixture = new ObjectMapper().readTree(stream);
    }
  }

  private static byte[] decrypt(String tenant, String event, String user, String attribute)
      throws GeneralSecurityException {
    return ScopedAttributeDecryptor.decrypt(
        fixture.get("master").asText(),
        tenant,
        event,
        user,
        attribute,
        fixture.get("envelope").asText());
  }

  @Test
  void decryptsAnEnvelopeInItsOwnScope() throws Exception {
    assertArrayEquals(
        fixture.get("plaintext").asText().getBytes(StandardCharsets.UTF_8),
        decrypt("tenant", "event", "voter", "login-code"));
  }

  @Test
  void refusesAnEnvelopeMovedToAnotherScope() {
    assertThrows(
        GeneralSecurityException.class, () -> decrypt("x", "event", "voter", "login-code"));
    assertThrows(
        GeneralSecurityException.class, () -> decrypt("tenant", "x", "voter", "login-code"));
    assertThrows(
        GeneralSecurityException.class, () -> decrypt("tenant", "event", "x", "login-code"));
    assertThrows(GeneralSecurityException.class, () -> decrypt("tenant", "event", "voter", "x"));
  }

  /**
   * However the envelope is malformed, callers get a {@link GeneralSecurityException} and not a
   * runtime parsing exception.
   */
  @Test
  void refusesMalformedInputWithTheSameExceptionType() {
    String master = fixture.get("master").asText();
    for (String envelope : new String[] {null, "", "plain", "seqenc:v1:***", "seqenc:v1:AAAA"}) {
      assertThrows(
          GeneralSecurityException.class,
          () -> ScopedAttributeDecryptor.decrypt(master, "t", "e", "u", "a", envelope));
    }
  }

  /** A missing or malformed master secret is refused before any decryption is attempted. */
  @Test
  void refusesAnInvalidMasterSecret() {
    for (String master : new String[] {null, "", "abcd", "z".repeat(64)}) {
      assertThrows(
          GeneralSecurityException.class,
          () ->
              ScopedAttributeDecryptor.decrypt(
                  master, "tenant", "event", "voter", "login-code", "seqenc:v1:AAAA"));
    }
  }
}
