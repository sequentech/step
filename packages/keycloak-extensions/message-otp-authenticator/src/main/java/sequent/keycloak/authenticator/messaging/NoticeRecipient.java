// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.EnumMap;
import java.util.EnumSet;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.Utils.MessageCourier;

/**
 * Where a voter's notices go: the notification channel the voter explicitly chose, if it is one of
 * their verified contacts.
 */
public record NoticeRecipient(
    Optional<MessageChannel> preferred,
    Map<MessageChannel, String> contacts,
    Set<String> electionIds,
    String voterId) {

  public static NoticeRecipient none() {
    return new NoticeRecipient(Optional.empty(), Map.of(), Set.of(), null);
  }

  public static NoticeRecipient fromUser(UserModel user, String mobileAttribute) {
    if (user == null) {
      return none();
    }
    Set<MessageChannel> verified = VoterChannels.verified(user, mobileAttribute);
    return new NoticeRecipient(
        MessageChannel.parse(user.getFirstAttribute(MessagingAttributes.MESSAGE_CHANNEL))
            .filter(verified::contains),
        VoterChannels.contacts(user, mobileAttribute),
        VoterChannels.elections(user),
        user.getId());
  }

  /** A voter still enrolling: only the channel verified in this session counts. */
  public static NoticeRecipient fromEnrollment(
      AuthenticationSessionModel authSession, String mobileAttribute) {
    Optional<MessageChannel> verified =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    Map<MessageChannel, String> contacts =
        new EnumMap<>(VoterChannels.enrollmentContacts(authSession, mobileAttribute));
    String messengerId = authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID);
    if (messengerId != null) {
      contacts.put(MessageChannel.MESSENGER, messengerId);
    }
    Optional<MessageChannel> preferred =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.FORM_NOTICE_CHANNEL))
            .filter(channel -> verified.equals(Optional.of(channel)));
    return new NoticeRecipient(preferred, contacts, Set.of(), authSession.getAuthNote("userId"));
  }

  /** The chosen channel when it is a messaging app that the event and the sender serve. */
  public Optional<MessageChannel> messagingApp(
      MessageSenderProvider sender, Optional<PublicMessagingChannels> projection) {
    return preferred
        .filter(channel -> channel != MessageChannel.EMAIL && channel != MessageChannel.SMS)
        .filter(sender::delivers)
        .filter(contacts::containsKey)
        .filter(
            channel ->
                projection
                    .map(p -> p.channelsFor(MessagePurpose.NOTICE, electionIds).contains(channel))
                    .orElse(false));
  }

  /**
   * The courier for Keycloak's email and SMS providers. A configured courier other than CHOSEN is
   * kept as it is; CHOSEN sends one notice, to the chosen channel or else to email or SMS.
   */
  public MessageCourier keycloakCourier(MessageCourier configured) {
    if (configured != MessageCourier.CHOSEN) {
      return configured;
    }
    Set<MessageChannel> keycloakChannels = EnumSet.of(MessageChannel.EMAIL, MessageChannel.SMS);
    Optional<MessageChannel> channel =
        preferred.filter(keycloakChannels::contains).filter(contacts::containsKey);
    if (channel.isEmpty()) {
      channel =
          contacts.containsKey(MessageChannel.EMAIL)
              ? Optional.of(MessageChannel.EMAIL)
              : Optional.of(MessageChannel.SMS);
    }
    return channel.get() == MessageChannel.EMAIL ? MessageCourier.EMAIL : MessageCourier.SMS;
  }

  /** The channel a notice is expected on, for pages that tell the voter. */
  public Optional<MessageChannel> expectedChannel(MessageCourier configured) {
    if (preferred.isPresent() && contacts.containsKey(preferred.get())) {
      return preferred;
    }
    return switch (keycloakCourier(configured)) {
      case EMAIL -> Optional.of(MessageChannel.EMAIL);
      case SMS -> Optional.of(MessageChannel.SMS);
      default ->
          contacts.containsKey(MessageChannel.EMAIL)
              ? Optional.of(MessageChannel.EMAIL)
              : Optional.empty();
    };
  }
}
