// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.Set;
import lombok.experimental.UtilityClass;

/** User attributes, form fields and authentication notes of the messaging channels. */
@UtilityClass
public class MessagingAttributes {
  // User attributes
  public final String MESSAGE_CHANNEL = "sequent.read-only.message-channel";
  public final String MESSAGE_NUMBER = "sequent.read-only.message-number";
  public final String WHATSAPP_NUMBER = "sequent.read-only.whatsapp-number";
  public final String VIBER_NUMBER = "sequent.read-only.viber-number";
  public final String MESSENGER_ID = "sequent.read-only.messenger-id";
  public final String MESSENGER_PAGE = "sequent.read-only.messenger-page";
  public final String VERIFIED_CHANNELS = "sequent.read-only.verified-channels";
  public final String MESSAGE_CONSENT = "sequent.read-only.message-consent";
  public final String AUTHORIZED_ELECTIONS = "authorized-election-ids";

  // Enrollment form fields, kept as authentication notes until the voter is saved
  public final String FORM_OTP_CHANNEL = "sequent.otp-channel";
  public final String FORM_WHATSAPP_NUMBER = "sequent.whatsapp-number";
  public final String FORM_VIBER_NUMBER = "sequent.viber-number";
  public final String FORM_MESSAGE_CONSENT = "sequent.message-consent";

  // Authentication notes only Keycloak sets
  public final String NOTE_OFFERED_CHANNELS = "sequent.offered-otp-channels";
  public final String NOTE_VERIFIED_CHANNEL = "sequent.verified-channel";
  public final String NOTE_VERIFIED_MESSENGER_ID = "sequent.verified-messenger-id";
  public final String NOTE_VERIFIED_MESSENGER_PAGE = "sequent.verified-messenger-page";

  /** The number or email address a correct code proved; Messenger uses the notes above. */
  public final String NOTE_VERIFIED_DESTINATION = "sequent.verified-destination";

  /** The consent the form showed and the voter accepted, once Keycloak validated it. */
  public final String NOTE_VERIFIED_CONSENT = "sequent.verified-consent";

  /** The channel and destination the live code was sent to, set when it is sent. */
  public final String NOTE_CODE_CHANNEL = "sequent.otp-code-channel";

  public final String NOTE_CODE_DESTINATION = "sequent.otp-code-destination";
  public final String NOTE_CODE_ID = "sequent.otp-code-id";
  public final String NOTE_DELIVERY_STATE = "sequent.otp-delivery-state";
  public final String NOTE_MESSENGER_REFERENCE = "sequent.messenger-reference";
  public final String NOTE_MESSENGER_LINK = "sequent.messenger-link";

  /** Upstream's Messenger linking word: never set any more, only cleared. */
  public final String NOTE_MESSENGER_WORD = "sequent.messenger-word";

  public final String NOTE_MESSENGER_STATE = "sequent.messenger-state";

  /**
   * Set while a sign-in reconnects Messenger after a Page change: the event's Page the live link
   * opens. Its confirmation saves the new account and Page (R7-10).
   */
  public final String NOTE_MESSENGER_RECONNECT = "sequent.messenger-reconnect";

  /** The channel an authenticated voter is changing their contact to, while its code is pending. */
  public final String NOTE_RESET_CHANNEL = "sequent.reset-channel";

  /**
   * Set when the contact change takes over the authentication session, so the sign-in's code, timer
   * and delivery state are forgotten once: the contact change has its own.
   */
  public final String NOTE_CONTACT_CHANGE_STARTED = "sequent.contact-change-started";

  /** The note holding the number entered for an SMS contact change (its configured attribute). */
  public final String NOTE_RESET_NUMBER_NOTE = "sequent.reset-number-note";

  /** Notes a submitted form must never set. */
  public final Set<String> KEYCLOAK_NOTES =
      Set.of(
          NOTE_OFFERED_CHANNELS,
          NOTE_VERIFIED_CHANNEL,
          NOTE_VERIFIED_MESSENGER_ID,
          NOTE_VERIFIED_MESSENGER_PAGE,
          NOTE_VERIFIED_DESTINATION,
          NOTE_VERIFIED_CONSENT,
          NOTE_CODE_CHANNEL,
          NOTE_CODE_DESTINATION,
          NOTE_CODE_ID,
          NOTE_DELIVERY_STATE,
          NOTE_MESSENGER_REFERENCE,
          NOTE_MESSENGER_LINK,
          NOTE_MESSENGER_WORD,
          NOTE_MESSENGER_STATE,
          NOTE_MESSENGER_RECONNECT,
          NOTE_RESET_CHANNEL,
          NOTE_CONTACT_CHANGE_STARTED,
          NOTE_RESET_NUMBER_NOTE);

  /** Realm attribute with the version of the consent wording shown at enrollment. */
  public final String CONSENT_VERSION_REALM_ATTRIBUTE = "sequent.message-consent-version";

  public final String CONSENT_VERSION_DEFAULT = "1";

  /** The messaging attributes, kept out of event details and logs: they hold contacts. */
  public final Set<String> CONTACT_ATTRIBUTES =
      Set.of(
          MESSAGE_CHANNEL,
          MESSAGE_NUMBER,
          WHATSAPP_NUMBER,
          VIBER_NUMBER,
          MESSENGER_ID,
          MESSENGER_PAGE,
          VERIFIED_CHANNELS,
          MESSAGE_CONSENT);
}
