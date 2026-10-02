// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;
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
import sequent.keycloak.authenticator.messaging.VoterChannels;

/** The enrollment's choice of channel for codes, and saving the contact it verified. */
@UtilityClass
public class EnrollmentChannels {
  private final Set<MessageChannel> MESSAGING_APPS =
      Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER);

  /** Whether the registration form asks the voter how to get codes. */
  public enum ChannelChoicePolicy {
    NONE,
    OTP_CHANNELS
  }

  /** The channels the voter's Post offers codes on that this Keycloak can deliver. */
  public List<MessageChannel> offered(
      KeycloakSession session, RealmModel realm, Collection<String> electionIds) {
    MessageSenderProvider sender = VoterChannels.sender(session);
    return VoterChannels.offeredForOtp(PublicMessagingChannels.fromRealm(realm), electionIds)
        .stream()
        .filter(channel -> VoterChannels.deliverable(sender, channel))
        .collect(Collectors.toList());
  }

  public String consentVersion(RealmModel realm) {
    String version = realm.getAttribute(MessagingAttributes.CONSENT_VERSION_REALM_ATTRIBUTE);
    return version == null || version.isBlank()
        ? MessagingAttributes.CONSENT_VERSION_DEFAULT
        : version;
  }

  /** What the consent box submits: the wording it showed and the channel it named. */
  public String consentValue(String consentVersion, MessageChannel channel) {
    return consentVersion + ":" + channel.name();
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
    if (MESSAGING_APPS.contains(channel.get())
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
    Optional<MessageChannel> verified =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    if (verified.isEmpty()) {
      return;
    }
    MessageChannel channel = verified.get();
    switch (channel) {
      case SMS -> setIfPresent(user, mobileAttribute, authSession.getAuthNote(mobileAttribute));
      case WHATSAPP ->
          setIfPresent(
              user,
              MessagingAttributes.WHATSAPP_NUMBER,
              authSession.getAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER));
      case VIBER ->
          setIfPresent(
              user,
              MessagingAttributes.VIBER_NUMBER,
              authSession.getAuthNote(MessagingAttributes.FORM_VIBER_NUMBER));
      case MESSENGER -> {
        setIfPresent(
            user,
            MessagingAttributes.MESSENGER_ID,
            authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID));
        setIfPresent(
            user,
            MessagingAttributes.MESSENGER_PAGE,
            authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE));
      }
      case EMAIL -> {}
    }

    Set<String> channels =
        user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS)
            .collect(Collectors.toCollection(LinkedHashSet::new));
    channels.add(channel.name());
    user.setAttribute(MessagingAttributes.VERIFIED_CHANNELS, List.copyOf(channels));

    if (channel.name().equals(authSession.getAuthNote(MessagingAttributes.FORM_NOTICE_CHANNEL))) {
      user.setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, channel.name());
    }
    if (MESSAGING_APPS.contains(channel)
        && consentValue(consentVersion, channel)
            .equals(authSession.getAuthNote(MessagingAttributes.FORM_MESSAGE_CONSENT))) {
      ObjectNode consent = new ObjectMapper().createObjectNode();
      consent.put("time", now.toString());
      consent.put("wording_version", consentVersion);
      consent.put("channel", channel.name());
      user.setSingleAttribute(MessagingAttributes.MESSAGE_CONSENT, consent.toString());
    }
  }

  private void setIfPresent(UserModel user, String attribute, String value) {
    if (value != null && !value.isBlank()) {
      user.setSingleAttribute(attribute, value.trim());
    }
  }
}
