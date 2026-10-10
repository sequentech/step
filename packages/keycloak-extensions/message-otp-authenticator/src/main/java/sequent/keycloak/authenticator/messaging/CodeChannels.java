// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.List;
import java.util.Optional;
import lombok.experimental.UtilityClass;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * The one channel a code goes to. At sign-in it is the voter's channel ({@link VoterChannel});
 * while enrolling, or when an authenticated voter changes their contact, it is the channel and
 * contact just entered. There is never a choice among channels and never another channel: when the
 * one channel can't be used, no code is sent and the refusal names why.
 */
@UtilityClass
public class CodeChannels {

  /**
   * Why no code can be sent. Each refusal has one message key for the page, and one reason ({@link
   * MessageReason}) for the event and the log, the same string harvest uses.
   */
  public enum Refusal {
    /** The record has no destination for any channel. */
    NO_CONTACT("messageOtp.auth.noChannel", MessageReason.NO_CONTACT_CHANNEL),
    /** The event no longer offers the voter's channel for codes. */
    NOT_OFFERED("messageOtp.auth.channelNotOffered", MessageReason.CHANNEL_NOT_OFFERED_FOR_CODES),
    /** Nothing in this Keycloak delivers the voter's channel (no message sender for it). */
    NOT_DELIVERED("messageOtp.auth.channelUnavailable", MessageReason.NO_SENDER_FOR_CHANNEL);

    private final String messageKey;
    private final MessageReason reason;

    Refusal(String messageKey, MessageReason reason) {
      this.messageKey = messageKey;
      this.reason = reason;
    }

    public String messageKey() {
      return messageKey;
    }

    public String reason() {
      return reason.reason();
    }
  }

  /**
   * The channel the code goes to, or why there is none.
   *
   * @param channel the channel and destination; for Messenger while enrolling the destination is
   *     empty until the voter's account is confirmed
   */
  public record Resolution(Optional<VoterChannel> channel, Optional<Refusal> refusal) {
    static Resolution of(VoterChannel channel) {
      return new Resolution(Optional.of(channel), Optional.empty());
    }

    static Resolution refused(Refusal refusal) {
      return new Resolution(Optional.empty(), Optional.of(refusal));
    }
  }

  /** At sign-in: the voter's one channel, if the event offers it and something delivers it. */
  public Resolution forSignIn(KeycloakSession session, RealmModel realm, UserModel user) {
    Optional<VoterChannel> channel = user == null ? Optional.empty() : VoterChannel.of(user);
    if (channel.isEmpty()) {
      return Resolution.refused(Refusal.NO_CONTACT);
    }
    MessageChannel name = channel.get().channel();
    if (!VoterChannels.deliverable(VoterChannels.sender(session), name)) {
      return Resolution.refused(Refusal.NOT_DELIVERED);
    }
    List<MessageChannel> offered =
        VoterChannels.offeredForOtp(
            PublicMessagingChannels.fromRealm(realm), VoterChannels.elections(user));
    if (!offered.contains(name)) {
      return Resolution.refused(Refusal.NOT_OFFERED);
    }
    return Resolution.of(channel.get());
  }

  /**
   * While enrolling or changing the contact: the channel entered, with the contact entered for it.
   * With no channel entered (a form that does not ask), the enrollment's email, else its mobile
   * number: the rule for voters without a messaging channel.
   */
  public Resolution forEnrollment(
      KeycloakSession session,
      RealmModel realm,
      AuthenticationSessionModel authSession,
      String mobileAttribute) {
    Optional<MessageChannel> chosen =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL));
    String email = blankToNull(authSession.getAuthNote("email"));
    String mobile = blankToNull(authSession.getAuthNote(mobileAttribute));
    if (chosen.isEmpty()) {
      if (email != null) {
        return Resolution.of(legacy(MessageChannel.EMAIL, email));
      }
      if (mobile != null) {
        return Resolution.of(legacy(MessageChannel.SMS, mobile));
      }
      return Resolution.refused(Refusal.NO_CONTACT);
    }
    MessageChannel channel = chosen.get();
    if (!VoterChannels.deliverable(VoterChannels.sender(session), channel)) {
      return Resolution.refused(Refusal.NOT_DELIVERED);
    }
    // The form that asked for the channel records what it offered (enrollment, the contact
    // change). A channel without that list didn't come from such a form: refused (R7-09).
    String offered = authSession.getAuthNote(MessagingAttributes.NOTE_OFFERED_CHANNELS);
    if (offered == null || !VoterChannels.parseList(offered).contains(channel)) {
      return Resolution.refused(Refusal.NOT_OFFERED);
    }
    String destination =
        switch (channel) {
          case EMAIL -> email;
          case SMS -> mobile;
          case WHATSAPP ->
              blankToNull(authSession.getAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER));
          case VIBER -> blankToNull(authSession.getAuthNote(MessagingAttributes.FORM_VIBER_NUMBER));
          case MESSENGER -> "";
        };
    if (destination == null) {
      return Resolution.refused(Refusal.NO_CONTACT);
    }
    Optional<String> page =
        channel == MessageChannel.MESSENGER
            ? PublicMessagingChannels.fromRealm(realm)
                .flatMap(PublicMessagingChannels::messengerPage)
                .map(PublicMessagingChannels.MessengerPage::pageId)
            : Optional.empty();
    return Resolution.of(
        new VoterChannel(channel, destination.trim(), page, VoterChannel.Source.RECORDED));
  }

  private VoterChannel legacy(MessageChannel channel, String destination) {
    return new VoterChannel(
        channel, destination.trim(), Optional.empty(), VoterChannel.Source.LEGACY);
  }

  private String blankToNull(String value) {
    return value == null || value.isBlank() ? null : value;
  }
}
