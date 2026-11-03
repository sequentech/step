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
  NO_SENDER_FOR_CHANNEL("No sender delivers the channel", false),
  /**
   * Keycloak only: a message OTP step of the realm sends to a configured courier (BOTH, EMAIL, SMS)
   * rather than the voter's channel, so a new channel would not get the sign-in codes. The contact
   * change is refused until every code step that can run is CHOSEN or NONE (R7-01).
   */
  CODE_STEP_NOT_ON_VOTER_CHANNEL("A code step doesn't send to the voter's channel", false),
  /**
   * Keycloak only: the realm's SMS provider sends only codes (Twilio Verify), so a temporary
   * password, a reset link or a notice by SMS is not sent and nothing depends on it (R7-08).
   */
  SMS_PROVIDER_SENDS_ONLY_CODES("The SMS provider sends only codes", false),
  /**
   * The voter's Messenger conversation belongs to another Page than the event's Messenger account
   * now has. Shared with harvest (R7-02); at sign-in Keycloak offers the reconnect (R7-10).
   */
  MESSENGER_PAGE_CHANGED("Not sent: the voter is connected to another Messenger Page", true),
  /**
   * Keycloak only: the Messenger reconnect after a Page change happens only at a sign-in step after
   * a validated password, never on a reset or enrollment page (R7-10).
   */
  MESSENGER_RECONNECT_NEEDS_PASSWORD("Messenger reconnect needs a signed-in password", false),
  /**
   * Keycloak only: a temporary password was handed over, but storing it failed, so the voter holds
   * a password that doesn't work and must ask again (R7-16).
   */
  PASSWORD_NOT_SET("The temporary password was sent but could not be set", false),
  /**
   * Keycloak only: the sign-in link (SmartLink) goes only by email, so a voter whose one channel is
   * another doesn't get it (R7-15).
   */
  SMART_LINK_ONLY_BY_EMAIL("The sign-in link is sent only by email", false);

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
