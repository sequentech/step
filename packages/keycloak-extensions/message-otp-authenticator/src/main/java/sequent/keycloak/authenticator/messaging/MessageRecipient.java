// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

/**
 * Whose channel a notice goes to (sequent-core's {@code NoticeRecipient}, sent as {@code
 * recipient}). Absent means {@link #VOTER_CHANNEL}.
 */
public enum MessageRecipient {
  /** The voter's recorded channel; harvest refuses any other recorded channel. */
  VOTER_CHANNEL,
  /**
   * The channel the voter just replaced: only for the notice that their contact was changed, sent
   * after Keycloak saved the new channel.
   */
  PREVIOUS_CHANNEL
}
