// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.time.Instant;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Saves the contact a code verified, the same way when the voter enrolls and when an authenticated
 * voter adds or replaces a contact later.
 */
@UtilityClass
public class VerifiedContacts {
  public final Set<MessageChannel> MESSAGING_APPS =
      Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER);

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

  /**
   * Saves the contact the code verified, read from the authentication notes. The notice channel
   * only changes when the voter explicitly asked for notices on the verified channel; consent is
   * recorded with its wording.
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
