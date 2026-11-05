// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import java.security.GeneralSecurityException;

/**
 * The one way out of a seqenc:v1 envelope for code outside this package. It exists for the voter
 * PIN link, which must show the stored value; every other caller compares through {@link
 * EncryptedAttributeCredential#matches}.
 */
public final class ScopedAttributeDecryptor {
  private ScopedAttributeDecryptor() {}

  /**
   * Decrypts one envelope under the scope it was encrypted for. A different tenant, event, voter or
   * attribute derives a different key, so a moved envelope fails authentication.
   *
   * @return the plaintext, which the caller must wipe
   * @throws GeneralSecurityException if the envelope is invalid or belongs to another scope
   */
  public static byte[] decrypt(
      String masterHex, String tenant, String event, String user, String attribute, String envelope)
      throws GeneralSecurityException {
    return EncryptedAttributeCredential.decryptScoped(
        masterHex, tenant, event, user, attribute, envelope);
  }
}
