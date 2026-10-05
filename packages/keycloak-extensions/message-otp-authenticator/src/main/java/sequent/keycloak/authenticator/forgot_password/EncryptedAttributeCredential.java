// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.Base64;
import java.util.HexFormat;
import java.util.List;
import java.util.Optional;
import java.util.function.BiPredicate;
import javax.crypto.Cipher;
import javax.crypto.Mac;
import javax.crypto.spec.IvParameterSpec;
import javax.crypto.spec.SecretKeySpec;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import sequent.keycloak.realm.RealmNames;

/** Verifies Windmill's scoped seqenc:v1 envelopes; never falls back to a password. */
final class EncryptedAttributeCredential {
  static final String POLICY = "credentialPolicy";
  static final String ATTRIBUTE = "credentialSecretAttribute";
  static final String PREFIX = "seqenc:v1:";
  static final int MAX_VALUE_BYTES = 150;
  // Bound work even for a corrupt/imported account with an excessive number of values.
  static final int MAX_VALUES = 100;

  enum Policy {
    PASSWORD,
    SECRET_ATTRIBUTE
  }

  private EncryptedAttributeCredential() {}

  static ProviderConfigProperty policyProperty() {
    var property =
        new ProviderConfigProperty(
            POLICY,
            "Credential verification policy",
            "PASSWORD preserves normal Keycloak password verification. SECRET_ATTRIBUTE verifies the"
                + " same input against an encrypted voter attribute; requires MASTER_SECRET and"
                + " REJECT_AMBIGUOUS. It never falls back to a Keycloak password.",
            ProviderConfigProperty.LIST_TYPE,
            Policy.PASSWORD.name());
    property.setOptions(List.of(Policy.PASSWORD.name(), Policy.SECRET_ATTRIBUTE.name()));
    return property;
  }

  static ProviderConfigProperty attributeProperty() {
    return new ProviderConfigProperty(
        ATTRIBUTE,
        "Encrypted credential attribute",
        "User Profile attribute annotated sequent.secret=true. Do not put this attribute in"
            + " matchAttributes. Used only with SECRET_ATTRIBUTE.",
        ProviderConfigProperty.STRING_TYPE,
        "");
  }

  static boolean usesPassword(AuthenticatorConfigModel config) {
    return Policy.PASSWORD.name().equals(Utils.getString(config, POLICY, Policy.PASSWORD.name()));
  }

  static Optional<BiPredicate<UserModel, String>> verifier(
      KeycloakSession session,
      RealmModel realm,
      AuthenticatorConfigModel config,
      List<String> matchAttributes,
      MultiAttributeCredentialResolver.MatchPolicy matchPolicy) {
    return verifier(
        session, realm, config, matchAttributes, matchPolicy, System.getenv("MASTER_SECRET"));
  }

  static Optional<BiPredicate<UserModel, String>> verifier(
      KeycloakSession session,
      RealmModel realm,
      AuthenticatorConfigModel config,
      List<String> matchAttributes,
      MultiAttributeCredentialResolver.MatchPolicy matchPolicy,
      String master) {
    if (usesPassword(config)) {
      return Optional.of(MultiAttributeCredentialResolver::isPasswordValid);
    }
    if (!Policy.SECRET_ATTRIBUTE.name().equals(Utils.getString(config, POLICY))
        || matchPolicy != MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS) {
      return Optional.empty();
    }
    String attribute = Utils.getString(config, ATTRIBUTE).trim();
    var scope = RealmNames.parseEventRealmName(realm.getName());
    var profile = Utils.getRealmUserProfileAttributes(session);
    boolean declaredSecret =
        profile.stream()
            .anyMatch(
                a ->
                    attribute.equals(a.getName())
                        && a.getAnnotations() != null
                        && "true"
                            .equalsIgnoreCase(
                                String.valueOf(a.getAnnotations().get("sequent.secret"))));
    boolean secretLookup =
        matchAttributes != null
            && profile.stream()
                .anyMatch(
                    a ->
                        matchAttributes.contains(a.getName())
                            && a.getAnnotations() != null
                            && "true"
                                .equalsIgnoreCase(
                                    String.valueOf(a.getAnnotations().get("sequent.secret"))));
    // Scope comes from the trusted realm, never from submitted or user-editable attributes.
    if (attribute.isEmpty()
        || !declaredSecret
        || secretLookup
        || scope.isEmpty()
        || master == null
        || !master.matches("[0-9a-fA-F]{64}")) {
      return Optional.empty();
    }
    var event = scope.get();
    return Optional.of(
        (user, submitted) ->
            matches(
                master,
                event.tenantId(),
                event.electionEventId(),
                user.getId(),
                attribute,
                user.getAttributeStream(attribute).limit(MAX_VALUES + 1L).toList(),
                submitted));
  }

  static boolean matches(
      String masterHex,
      String tenant,
      String event,
      String user,
      String attribute,
      List<String> envelopes,
      String submitted) {
    if (submitted == null
        || submitted.isBlank()
        || submitted.length() > MAX_VALUE_BYTES
        || envelopes == null
        || envelopes.isEmpty()
        || envelopes.size() > MAX_VALUES) {
      return false;
    }
    byte[] input = submitted.getBytes(StandardCharsets.UTF_8);
    byte[] master = null;
    byte[] key = null;
    byte[] expected = null;
    try {
      if (input.length > MAX_VALUE_BYTES
          || masterHex == null
          || !masterHex.matches("[0-9a-fA-F]{64}")) {
        return false;
      }
      master = HexFormat.of().parseHex(masterHex);
      key = deriveKey(master, tenant, event, user, attribute);
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      expected = digest.digest(input);
      boolean matched = false;
      boolean invalid = false;
      for (String envelope : envelopes) {
        byte[] plaintext = null;
        byte[] actual = null;
        try {
          plaintext = decrypt(key, envelope);
          actual = digest.digest(plaintext);
          // Both digests have exactly 32 bytes. Check every stored value, without short-circuiting.
          matched |= MessageDigest.isEqual(expected, actual);
        } catch (GeneralSecurityException | IllegalArgumentException error) {
          invalid = true;
        } finally {
          wipe(plaintext);
          wipe(actual);
        }
      }
      return matched & !invalid;
    } catch (GeneralSecurityException | IllegalArgumentException error) {
      // Do not log key material, ciphertext, plaintext or exception payloads.
      return false;
    } finally {
      wipe(input);
      wipe(master);
      wipe(key);
      wipe(expected);
    }
  }

  private static byte[] deriveKey(
      byte[] master, String tenant, String event, String user, String attribute)
      throws GeneralSecurityException {
    Mac mac = Mac.getInstance("HmacSHA256");
    mac.init(
        new SecretKeySpec(
            "sequent-voter-secret-attribute-v1".getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
    byte[] prk = mac.doFinal(master);
    try {
      mac.init(new SecretKeySpec(prk, "HmacSHA256"));
      mac.update(
          ("tenant=" + tenant + ";event=" + event + ";user=" + user + ";attribute=" + attribute)
              .getBytes(StandardCharsets.UTF_8));
      return mac.doFinal(new byte[] {1});
    } finally {
      wipe(prk);
    }
  }

  private static byte[] decrypt(byte[] key, String envelope) throws GeneralSecurityException {
    if (envelope == null || envelope.length() > 255 || !envelope.startsWith(PREFIX)) {
      throw new IllegalArgumentException("Invalid credential envelope");
    }
    String encoded = envelope.substring(PREFIX.length());
    if (!encoded.matches("[A-Za-z0-9_-]+")) {
      throw new IllegalArgumentException("Invalid credential encoding");
    }
    byte[] payload = Base64.getUrlDecoder().decode(encoded);
    if (payload.length < 32
        || !Base64.getUrlEncoder().withoutPadding().encodeToString(payload).equals(encoded)) {
      throw new IllegalArgumentException("Invalid credential payload");
    }
    int size = ByteBuffer.wrap(payload).order(ByteOrder.LITTLE_ENDIAN).getInt();
    if (size < 16 || size > MAX_VALUE_BYTES + 16 || payload.length != 4 + size + 12) {
      throw new IllegalArgumentException("Invalid credential length");
    }
    Cipher cipher = Cipher.getInstance("ChaCha20-Poly1305");
    cipher.init(
        Cipher.DECRYPT_MODE,
        new SecretKeySpec(key, "ChaCha20"),
        new IvParameterSpec(payload, 4 + size, 12));
    return cipher.doFinal(payload, 4, size);
  }

  private static void wipe(byte[] bytes) {
    if (bytes != null) Arrays.fill(bytes, (byte) 0);
  }
}
