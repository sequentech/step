// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.util.JsonSerialization;

class EncryptedAttributeCredentialTest {
  @SuppressWarnings("unchecked")
  static Map<String, String> fixture() throws Exception {
    try (var stream =
        EncryptedAttributeCredentialTest.class.getResourceAsStream("/voter-secret-v1.json")) {
      return JsonSerialization.readValue(stream, Map.class);
    }
  }

  private boolean matches(Map<String, String> v, String input, List<String> values) {
    return EncryptedAttributeCredential.matches(
        v.get("master"),
        v.get("tenant"),
        v.get("event"),
        v.get("user"),
        v.get("attribute"),
        values,
        input);
  }

  @Test
  void verifiesTheSameV1FixtureAsRust() throws Exception {
    var v = fixture();
    assertTrue(matches(v, v.get("plaintext"), List.of(v.get("envelope"))));
    for (String wrong : List.of("", " ", "abç 0123", "Abc 0123", "Abç 0123 ", "x".repeat(151))) {
      assertFalse(matches(v, wrong, List.of(v.get("envelope"))));
    }
    assertFalse(matches(v, null, List.of(v.get("envelope"))));
  }

  @Test
  void failsClosedForEveryScopeAndKeyMismatch() throws Exception {
    for (String field : List.of("master", "tenant", "event", "user", "attribute")) {
      var v = fixture();
      v.put(field, field.equals("master") ? "00".repeat(32) : "different");
      assertFalse(matches(v, v.get("plaintext"), List.of(v.get("envelope"))), field);
    }
  }

  @Test
  void rejectsPlaintextMalformedAndTamperedValuesEvenAlongsideAValidValue() throws Exception {
    var v = fixture();
    String good = v.get("envelope");
    for (String bad :
        List.of(
            v.get("plaintext"),
            "seqenc:v2:" + good.substring(10),
            good + "=",
            good + "AAAA",
            "seqenc:v1:AAAA",
            good.substring(0, 25) + "_" + good.substring(26),
            "seqenc:v1:" + "a".repeat(256))) {
      assertFalse(matches(v, v.get("plaintext"), List.of(bad)), bad);
      assertFalse(matches(v, v.get("plaintext"), List.of(good, bad)));
    }
    assertFalse(matches(v, v.get("plaintext"), List.of()));
    assertTrue(matches(v, v.get("plaintext"), List.of(good, good)));
    assertTrue(matches(v, v.get("plaintext"), List.of(v.get("alternate_envelope"), good)));
    assertTrue(matches(v, "different", List.of(good, v.get("alternate_envelope"))));
    assertFalse(matches(v, v.get("plaintext"), java.util.Collections.nCopies(101, good)));
  }

  @Test
  void invalidConfigurationNeverFallsBackToPasswordVerification() {
    var session = mock(KeycloakSession.class);
    var realm = mock(RealmModel.class);
    var config = new AuthenticatorConfigModel();
    config.setConfig(Map.of(EncryptedAttributeCredential.POLICY, "INVALID"));
    assertTrue(
        EncryptedAttributeCredential.verifier(
                session,
                realm,
                config,
                List.of("username"),
                MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS)
            .isEmpty());
    config.setConfig(Map.of(EncryptedAttributeCredential.POLICY, "SECRET_ATTRIBUTE"));
    assertTrue(
        EncryptedAttributeCredential.verifier(
                session,
                realm,
                config,
                List.of("username"),
                MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH)
            .isEmpty());
    assertTrue(
        EncryptedAttributeCredential.verifier(
                session,
                realm,
                config,
                List.of("username"),
                MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS)
            .isEmpty());
  }

  @Test
  void passwordRemainsTheDefaultAndBothFactoriesExposeThePolicy() {
    assertTrue(EncryptedAttributeCredential.usesPassword(null));
    for (var properties :
        List.of(
            new MultiAttributePasswordAuthenticator().getConfigProperties(),
            new MultiAttributePasswordDirectGrantAuthenticator().getConfigProperties())) {
      assertTrue(
          properties.stream()
              .anyMatch(
                  p ->
                      p.getName().equals("credentialPolicy")
                          && p.getDefaultValue().equals("PASSWORD")));
      assertTrue(
          properties.stream().anyMatch(p -> p.getName().equals("credentialSecretAttribute")));
    }
  }
}
