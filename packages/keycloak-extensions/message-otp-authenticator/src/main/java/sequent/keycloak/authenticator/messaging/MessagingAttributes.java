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
  public final String FORM_NOTICE_CHANNEL = "sequent.notice-channel";
  public final String FORM_MESSAGE_CONSENT = "sequent.message-consent";

  // Authentication notes only Keycloak sets
  public final String NOTE_OFFERED_CHANNELS = "sequent.offered-otp-channels";
  public final String NOTE_VERIFIED_CHANNEL = "sequent.verified-channel";
  public final String NOTE_VERIFIED_MESSENGER_ID = "sequent.verified-messenger-id";
  public final String NOTE_VERIFIED_MESSENGER_PAGE = "sequent.verified-messenger-page";
  public final String NOTE_CODE_ID = "sequent.otp-code-id";
  public final String NOTE_DELIVERY_STATE = "sequent.otp-delivery-state";
  public final String NOTE_MESSENGER_REFERENCE = "sequent.messenger-reference";
  public final String NOTE_MESSENGER_LINK = "sequent.messenger-link";
  public final String NOTE_MESSENGER_WORD = "sequent.messenger-word";
  public final String NOTE_MESSENGER_STATE = "sequent.messenger-state";

  /** Notes a submitted form must never set. */
  public final Set<String> KEYCLOAK_NOTES =
      Set.of(
          NOTE_OFFERED_CHANNELS,
          NOTE_VERIFIED_CHANNEL,
          NOTE_VERIFIED_MESSENGER_ID,
          NOTE_VERIFIED_MESSENGER_PAGE,
          NOTE_CODE_ID,
          NOTE_DELIVERY_STATE,
          NOTE_MESSENGER_REFERENCE,
          NOTE_MESSENGER_LINK,
          NOTE_MESSENGER_WORD,
          NOTE_MESSENGER_STATE);

  /** Realm attribute with the version of the consent wording shown at enrollment. */
  public final String CONSENT_VERSION_REALM_ATTRIBUTE = "sequent.message-consent-version";

  public final String CONSENT_VERSION_DEFAULT = "1";
}
