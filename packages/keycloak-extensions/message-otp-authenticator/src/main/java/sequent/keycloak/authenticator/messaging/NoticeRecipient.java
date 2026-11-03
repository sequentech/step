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
    Optional<VoterChannel> channel,
    Set<String> electionIds,
    String voterId,
    VoterChannel.Source shape) {

  /**
   * A recipient whose record shape is its channel's, or LEGACY without one.
   *
   * @param shape which of the three cases the voter's record is ({@link VoterChannel#shape}),
   *     whether or not its channel can be used: only a LEGACY voter may get a realm's configured
   *     courier (R7-01)
   */
  public NoticeRecipient(Optional<VoterChannel> channel, Set<String> electionIds, String voterId) {
    this(
        channel,
        electionIds,
        voterId,
        channel.map(VoterChannel::source).orElse(VoterChannel.Source.LEGACY));
  }

  /** How a notice is delivered, or why it is not. */
  public enum Route {
    /**
     * Handed to the message sender (harvest), which applies the event's channels and records the
     * outcome, refusals included. The channel is empty for a voter without one: harvest records
     * that too.
     */
    SENDER,
    /** Keycloak's own email or SMS provider delivers it: the sender doesn't carry the channel. */
    KEYCLOAK,
    /** Not sent, and nothing can record it elsewhere; the reason says why. */
    NOT_SENT
  }

  /** The route of a notice, with the reason when it is not sent. */
  public record Delivery(Route route, Optional<MessageReason> refusal) {}

  public static NoticeRecipient none() {
    return new NoticeRecipient(Optional.empty(), Set.of(), null);
  }

  /** A saved voter: their one channel ({@link VoterChannel}). */
  public static NoticeRecipient fromUser(UserModel user, String mobileAttribute) {
    if (user == null) {
      return none();
    }
    return new NoticeRecipient(
        VoterChannel.of(user),
        VoterChannels.elections(user),
        user.getId(),
        VoterChannel.shape(user));
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
   * How a notice reaches the voter's channel. Keycloak never decides whether the event uses the
   * channel for notices: a channel the sender carries is always handed to it, and harvest refuses
   * with its reason (and records it). Keycloak's email and SMS providers carry email and SMS only
   * when the sender doesn't. A voter with no channel is handed to a sender that records refusals;
   * without one, the notice is not sent and the reason is logged.
   */
  public Delivery delivery(MessageSenderProvider sender) {
    if (channel.isEmpty()) {
      return sender.getChannels().isEmpty()
          ? new Delivery(Route.NOT_SENT, Optional.of(MessageReason.NO_CONTACT_CHANNEL))
          : new Delivery(Route.SENDER, Optional.empty());
    }
    MessageChannel name = channel.get().channel();
    if (sender.delivers(name)) {
      return new Delivery(Route.SENDER, Optional.empty());
    }
    return name == MessageChannel.EMAIL || name == MessageChannel.SMS
        ? new Delivery(Route.KEYCLOAK, Optional.empty())
        : new Delivery(Route.NOT_SENT, Optional.of(MessageReason.NO_SENDER_FOR_CHANNEL));
  }

  private static String present(String value) {
    return value == null || value.isBlank() ? null : value.trim();
  }
}
