// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

/**
 * Mirrors sequent-core's {@code MessageAttemptState}. ACCEPTED means the provider took the message,
 * not that the voter received it; UNKNOWN means it may have been sent and must not be resent
 * automatically.
 */
public enum MessageAttemptState {
  QUEUED,
  ACCEPTED,
  DELIVERED,
  FAILED,
  UNKNOWN;

  /** Unrecognised or missing states are UNKNOWN: never assume a message was or was not sent. */
  public static MessageAttemptState parse(String value) {
    if (value != null) {
      for (MessageAttemptState state : values()) {
        if (state.name().equals(value)) {
          return state;
        }
      }
    }
    return UNKNOWN;
  }
}
