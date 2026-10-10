// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

/**
 * Why a message is not sent, with the one reason string the page, the event, the log and harvest
 * share. The strings of the shared reasons are sequent-core's {@code MessageReason} strings (a test
 * keeps them equal); harvest gives them for the sends it refuses, and Keycloak only uses them for
 * what it decides itself: which channel a code goes to, and email and SMS sent by its own
 * providers.
 */
public enum MessageReason {
  /** The voter's record has no channel with a destination. */
  NO_CONTACT_CHANNEL("No contact channel", true),
  /** The event, or the voter's Post, doesn't offer the voter's channel for codes. */
  CHANNEL_NOT_OFFERED_FOR_CODES("Channel not offered for codes", true),
  /** The event, or the voter's Post, doesn't offer the voter's channel for notices. */
  CHANNEL_NOT_OFFERED_FOR_NOTICES("Channel not offered for notices", true),
  /**
   * Keycloak only: this deployment has no message sender for the channel (no harvest address, or
   * the {@code KEYCLOAK_ONLY} policy), so nothing can carry it and harvest is never asked.
   */
  NO_SENDER_FOR_CHANNEL("No sender delivers the channel", false);

  private final String reason;
  private final boolean sharedWithHarvest;

  MessageReason(String reason, boolean sharedWithHarvest) {
    this.reason = reason;
    this.sharedWithHarvest = sharedWithHarvest;
  }

  public String reason() {
    return reason;
  }

  /** Whether harvest gives the same string (sequent-core's {@code MessageReason}). */
  public boolean sharedWithHarvest() {
    return sharedWithHarvest;
  }

  @Override
  public String toString() {
    return reason;
  }
}
