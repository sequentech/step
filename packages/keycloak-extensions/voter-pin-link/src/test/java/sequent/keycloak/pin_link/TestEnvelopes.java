// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.HexFormat;
import javax.crypto.Cipher;
import javax.crypto.Mac;
import javax.crypto.spec.IvParameterSpec;
import javax.crypto.spec.SecretKeySpec;

/** Writes seqenc:v1 envelopes the way the Windmill importer does; only tests encrypt in Java. */
final class TestEnvelopes {
  private TestEnvelopes() {}

  static String encrypt(
      String masterHex, String tenant, String event, String user, String attribute, String value)
      throws GeneralSecurityException {
    Mac mac = Mac.getInstance("HmacSHA256");
    mac.init(
        new SecretKeySpec(
            "sequent-voter-secret-attribute-v1".getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
    byte[] prk = mac.doFinal(HexFormat.of().parseHex(masterHex));
    mac.init(new SecretKeySpec(prk, "HmacSHA256"));
    mac.update(
        ("tenant=" + tenant + ";event=" + event + ";user=" + user + ";attribute=" + attribute)
            .getBytes(StandardCharsets.UTF_8));
    byte[] key = mac.doFinal(new byte[] {1});
    byte[] nonce = new byte[12];
    new SecureRandom().nextBytes(nonce);
    Cipher cipher = Cipher.getInstance("ChaCha20-Poly1305");
    cipher.init(
        Cipher.ENCRYPT_MODE, new SecretKeySpec(key, "ChaCha20"), new IvParameterSpec(nonce));
    byte[] sealed = cipher.doFinal(value.getBytes(StandardCharsets.UTF_8));
    ByteBuffer payload =
        ByteBuffer.allocate(4 + sealed.length + nonce.length).order(ByteOrder.LITTLE_ENDIAN);
    payload.putInt(sealed.length).put(sealed).put(nonce);
    return "seqenc:v1:" + Base64.getUrlEncoder().withoutPadding().encodeToString(payload.array());
  }
}
