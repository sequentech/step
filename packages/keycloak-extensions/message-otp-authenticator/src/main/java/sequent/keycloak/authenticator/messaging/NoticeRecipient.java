// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.Optional;
import java.util.Set;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Where a voter's notices go: the voter's one channel, and nowhere else. When that channel can't be
 * used, the notice is not sent, and the reason is logged; it never falls back to another channel.
 *
 * @param channel the voter's channel and destination; empty when the record has none
 */
public record NoticeRecipient(
    Optional<VoterChannel> channel, Set<String> electionIds, String voterId) {

  /** How a notice is delivered, or why it is not. */
  public enum Route {
    /** The message sender delivers the channel. */
    SENDER,
    /** Keycloak's own email or SMS provider delivers it. */
    KEYCLOAK,
    /** Not sent; the reason says why. */
    NOT_SENT
  }

  /** Why a notice is not sent. One reason string each, shared by the log and the event. */
  public enum Refusal {
    NO_CONTACT("No contact channel"),
    NOT_OFFERED("Channel not offered for notices"),
    NOT_DELIVERED("No sender delivers the channel");

    private final String reason;

    Refusal(String reason) {
      this.reason = reason;
    }

    public String reason() {
      return reason;
    }
  }

  /** The route of a notice, with the reason when it is not sent. */
  public record Delivery(Route route, Optional<Refusal> refusal) {}

  public static NoticeRecipient none() {
    return new NoticeRecipient(Optional.empty(), Set.of(), null);
  }

  /** A saved voter: their one channel ({@link VoterChannel}). */
  public static NoticeRecipient fromUser(UserModel user, String mobileAttribute) {
    if (user == null) {
      return none();
    }
    return new NoticeRecipient(VoterChannel.of(user), VoterChannels.elections(user), user.getId());
  }

  /**
   * A voter still enrolling: the channel and destination a code verified in this session, read from
   * notes only Keycloak sets. Without a verified channel (a flow that sends no code), the
   * enrollment's email, else its mobile number.
   */
  public static NoticeRecipient fromEnrollment(
      AuthenticationSessionModel authSession, String mobileAttribute) {
    String voterId = authSession.getAuthNote("userId");
    return new NoticeRecipient(enrollmentChannel(authSession, mobileAttribute), Set.of(), voterId);
  }

  /** The enrolling voter's channel, as {@link #fromEnrollment} reads it. */
  public static Optional<VoterChannel> enrollmentChannel(
      AuthenticationSessionModel authSession, String mobileAttribute) {
    Optional<MessageChannel> verified =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    String email = present(authSession.getAuthNote("email"));
    String mobile = present(authSession.getAuthNote(mobileAttribute));
    if (verified.isPresent()) {
      MessageChannel channel = verified.get();
      if (channel == MessageChannel.MESSENGER) {
        String id =
            present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID));
        String page =
            present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE));
        return id == null || page == null
            ? Optional.empty()
            : Optional.of(
                new VoterChannel(channel, id, Optional.of(page), VoterChannel.Source.RECORDED));
      }
      String destination =
          present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_DESTINATION));
      if (destination == null) {
        // Email and SMS codes sent by a configured courier prove the enrollment's own contact.
        destination =
            switch (channel) {
              case EMAIL -> email;
              case SMS -> mobile;
              default -> null;
            };
      }
      return destination == null
          ? Optional.empty()
          : Optional.of(
              new VoterChannel(
                  channel, destination, Optional.empty(), VoterChannel.Source.RECORDED));
    }
    if (email != null) {
      return Optional.of(
          new VoterChannel(
              MessageChannel.EMAIL, email, Optional.empty(), VoterChannel.Source.LEGACY));
    }
    if (mobile != null) {
      return Optional.of(
          new VoterChannel(
              MessageChannel.SMS, mobile, Optional.empty(), VoterChannel.Source.LEGACY));
    }
    return Optional.empty();
  }

  /**
   * How a notice reaches the voter's channel. The message sender carries the channels it delivers,
   * when the event uses the channel for notices; Keycloak's email and SMS providers carry email and
   * SMS otherwise. Anything else is not sent.
   */
  public Delivery delivery(
      MessageSenderProvider sender, Optional<PublicMessagingChannels> projection) {
    if (channel.isEmpty()) {
      return new Delivery(Route.NOT_SENT, Optional.of(Refusal.NO_CONTACT));
    }
    MessageChannel name = channel.get().channel();
    boolean keycloakChannel = name == MessageChannel.EMAIL || name == MessageChannel.SMS;
    if (sender.delivers(name)) {
      boolean offered =
          projection
              .map(p -> p.channelsFor(MessagePurpose.NOTICE, electionIds).contains(name))
              .orElse(false);
      if (offered) {
        return new Delivery(Route.SENDER, Optional.empty());
      }
      return keycloakChannel
          ? new Delivery(Route.KEYCLOAK, Optional.empty())
          : new Delivery(Route.NOT_SENT, Optional.of(Refusal.NOT_OFFERED));
    }
    return keycloakChannel
        ? new Delivery(Route.KEYCLOAK, Optional.empty())
        : new Delivery(Route.NOT_SENT, Optional.of(Refusal.NOT_DELIVERED));
  }

  private static String present(String value) {
    return value == null || value.isBlank() ? null : value.trim();
  }
}
