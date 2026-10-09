// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

/** Mirrors sequent-core's {@code MessengerLinkState}. */
public enum MessengerLinkState {
  PENDING,
  CODE_SENT,
  CONFIRMED,
  EXPIRED,
  REPLACED;

  /** Unrecognised states are treated as EXPIRED, so they can never confirm a contact. */
  public static MessengerLinkState parse(String value) {
    if (value != null) {
      for (MessengerLinkState state : values()) {
        if (state.name().equals(value)) {
          return state;
        }
      }
    }
    return EXPIRED;
  }
}
