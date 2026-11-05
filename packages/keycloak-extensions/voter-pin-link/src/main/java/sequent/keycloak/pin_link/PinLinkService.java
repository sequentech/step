// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import sequent.keycloak.authenticator.forgot_password.ScopedAttributeDecryptor;

/**
 * Decides one POST, inside the caller's database transaction. The reason is only for the event log:
 * the voter sees one generic message for all of them but {@link Reason#REVEALED}.
 */
final class PinLinkService {

  enum Reason {
    REVEALED,
    /** Unknown, already used, or deleted while this request waited for the row lock. */
    NOT_AVAILABLE,
    LOCKED,
    WRONG_USERNAME,
    /** The envelope doesn't decrypt or doesn't produce this token: moved, or the PIN changed. */
    INVALID_ENVELOPE
  }

  /** A successful outcome carries the PIN, which the caller wipes. */
  record Outcome(Reason reason, byte[] pin) {
    static Outcome of(Reason reason) {
      return new Outcome(reason, null);
    }
  }

  interface RevealAudit {
    /** Must return only once the entry is durably accepted, and throw otherwise. */
    void record() throws Exception;
  }

  /** Rolls back the transaction: the link stays alive and nothing is shown. */
  static final class AuditFailedException extends RuntimeException {
    AuditFailedException() {
      super("The reveal could not be recorded");
    }
  }

  /** Who this request is about: the voter comes from the token, never from the submission. */
  record Voter(String tenantId, String electionEventId, String id, String username) {}

  private final PinLinkStore store;
  private final PinLinkConfig config;
  private final String masterHex;

  PinLinkService(PinLinkStore store, PinLinkConfig config, String masterHex) {
    this.store = store;
    this.config = config;
    this.masterHex = masterHex;
  }

  Outcome retrieve(
      Connection connection,
      Voter voter,
      String canonicalToken,
      String submittedUsername,
      RevealAudit audit)
      throws SQLException {
    String hash = PinLinkToken.lookupHash(canonicalToken);
    // Every POST for one link queues here, whatever replica it reached.
    if (!store.lockHash(connection, voter.id(), config.tokenHashAttribute(), hash)) {
      return Outcome.of(Reason.NOT_AVAILABLE);
    }
    int attempts = readAttempts(connection, voter.id());
    if (attempts >= config.maxAttempts()) {
      return Outcome.of(Reason.LOCKED);
    }
    if (!sameUsername(voter.username(), submittedUsername)) {
      store.setAttempts(connection, voter.id(), config.attemptsAttribute(), attempts + 1);
      return Outcome.of(Reason.WRONG_USERNAME);
    }
    byte[] pin = decrypt(connection, voter, canonicalToken);
    if (pin == null) {
      return Outcome.of(Reason.INVALID_ENVELOPE);
    }
    boolean revealed = false;
    try {
      if (!store.deleteHash(connection, voter.id(), config.tokenHashAttribute(), hash)) {
        return Outcome.of(Reason.NOT_AVAILABLE);
      }
      store.deleteAttribute(connection, voter.id(), config.attemptsAttribute());
      // After the delete, so its row lock still holds; before the commit, so a failure undoes it.
      try {
        audit.record();
      } catch (Exception error) {
        throw new AuditFailedException();
      }
      revealed = true;
      return new Outcome(Reason.REVEALED, pin);
    } finally {
      if (!revealed) {
        Arrays.fill(pin, (byte) 0);
      }
    }
  }

  private int readAttempts(Connection connection, String voterId) throws SQLException {
    List<String> values = store.values(connection, voterId, config.attemptsAttribute());
    if (values.isEmpty()) {
      return 0;
    }
    try {
      return Integer.parseInt(values.get(0));
    } catch (NumberFormatException error) {
      // Corrupt counter: lock rather than start counting again from zero.
      return config.maxAttempts();
    }
  }

  private byte[] decrypt(Connection connection, Voter voter, String canonicalToken)
      throws SQLException {
    List<String> envelopes = store.values(connection, voter.id(), config.pinAttribute());
    if (envelopes.size() != 1) {
      return null;
    }
    byte[] pin = null;
    try {
      pin =
          ScopedAttributeDecryptor.decrypt(
              masterHex,
              voter.tenantId(),
              voter.electionEventId(),
              voter.id(),
              config.pinAttribute(),
              envelopes.get(0));
      String expected =
          PinLinkToken.derive(
              masterHex, voter.tenantId(), voter.electionEventId(), voter.id(), pin);
      if (PinLinkToken.sameToken(expected, canonicalToken)) {
        return pin;
      }
    } catch (GeneralSecurityException error) {
      // Key material, ciphertext and exception payloads stay out of the logs.
    }
    if (pin != null) {
      Arrays.fill(pin, (byte) 0);
    }
    return null;
  }

  static boolean sameUsername(String expected, String submitted) {
    if (expected == null || submitted == null) {
      return false;
    }
    return MessageDigest.isEqual(
        expected.toLowerCase(Locale.ROOT).getBytes(StandardCharsets.UTF_8),
        submitted.trim().toLowerCase(Locale.ROOT).getBytes(StandardCharsets.UTF_8));
  }
}
