// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
import sequent.keycloak.authenticator.messaging.VoterChannels;

/** The enrollment's choice of channel for codes, and saving the contact it verified. */
@UtilityClass
public class EnrollmentChannels {

  /** Whether the registration form asks the voter how to get codes. */
  public enum ChannelChoicePolicy {
    NONE,
    OTP_CHANNELS
  }

  /**
   * The channels the voter's Post offers codes on that this Keycloak can deliver. The Post is the
   * ID or a label of its election; one that matches no election only gets the channels every
   * restricted election offers.
   */
  public List<MessageChannel> offered(
      KeycloakSession session, RealmModel realm, Collection<String> posts) {
    MessageSenderProvider sender = VoterChannels.sender(session);
    return VoterChannels.offeredForOtp(
            PublicMessagingChannels.fromRealm(realm),
            posts,
            PublicMessagingChannels.UnmatchedPostPolicy.COMMON_CHANNELS)
        .stream()
        .filter(channel -> VoterChannels.deliverable(sender, channel))
        .collect(Collectors.toList());
  }

  public String consentVersion(RealmModel realm) {
    return VerifiedContacts.consentVersion(realm);
  }

  /** What the consent box submits: the wording it showed and the channel it named. */
  public String consentValue(String consentVersion, MessageChannel channel) {
    return VerifiedContacts.consentValue(consentVersion, channel);
  }

  /** Errors of the submitted channel choice, its contact and the consent it needs. */
  public List<FormMessage> validate(
      List<MessageChannel> offered,
      MultivaluedMap<String, String> formData,
      String mobileAttribute,
      String consentVersion) {
    List<FormMessage> errors = new ArrayList<>();
    String submitted = formData.getFirst(MessagingAttributes.FORM_OTP_CHANNEL);
    Optional<MessageChannel> channel = MessageChannel.parse(submitted);
    if (channel.isEmpty()) {
      errors.add(
          new FormMessage(
              MessagingAttributes.FORM_OTP_CHANNEL, "messaging.channelChoice.required"));
      return errors;
    }
    if (!offered.contains(channel.get())) {
      errors.add(
          new FormMessage(
              MessagingAttributes.FORM_OTP_CHANNEL, "messaging.channelChoice.notAvailable"));
      return errors;
    }
    String numberField =
        switch (channel.get()) {
          case SMS -> mobileAttribute;
          case WHATSAPP -> MessagingAttributes.FORM_WHATSAPP_NUMBER;
          case VIBER -> MessagingAttributes.FORM_VIBER_NUMBER;
          case EMAIL, MESSENGER -> null;
        };
    if (numberField != null && !VoterChannels.isE164(trimmed(formData.getFirst(numberField)))) {
      errors.add(new FormMessage(numberField, "messaging.number.invalid"));
    }
    if (channel.get() == MessageChannel.EMAIL && trimmed(formData.getFirst("email")) == null) {
      errors.add(new FormMessage("email", Messages.MISSING_EMAIL));
    }
    if (VerifiedContacts.MESSAGING_APPS.contains(channel.get())
        && !consentValue(consentVersion, channel.get())
            .equals(formData.getFirst(MessagingAttributes.FORM_MESSAGE_CONSENT))) {
      errors.add(
          new FormMessage(MessagingAttributes.FORM_MESSAGE_CONSENT, "messaging.consent.required"));
    }
    return errors;
  }

  private String trimmed(String value) {
    return value == null || value.isBlank() ? null : value.trim();
  }

  /**
   * Saves the contact the enrollment code verified. The notice channel only changes when the voter
   * explicitly asked for notices on the verified channel; consent is recorded with its wording.
   */
  public void persist(
      UserModel user,
      AuthenticationSessionModel authSession,
      String mobileAttribute,
      String consentVersion,
      Instant now) {
    VerifiedContacts.persist(user, authSession, mobileAttribute, consentVersion, now);
  }
}
