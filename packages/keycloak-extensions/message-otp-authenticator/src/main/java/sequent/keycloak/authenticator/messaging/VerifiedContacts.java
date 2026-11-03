// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.time.Instant;
import java.util.Optional;
import java.util.Set;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Saves the contact a code verified, the same way when the voter enrolls and when an authenticated
 * voter changes how they are contacted. A voter has one channel: saving one replaces all five
 * channel attributes, and removes the first release's {@code verified-channels}, {@code
 * whatsapp-number} and {@code viber-number}. Only notes Keycloak sets are read, never form fields.
 */
@JBossLog
@UtilityClass
public class VerifiedContacts {
  public final Set<MessageChannel> MESSAGING_APPS =
      Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER);

  /** The channels whose destination is {@code message-number}. */
  public final Set<MessageChannel> NUMBER_CHANNELS =
      Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.SMS);

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
   * The recorded consent: when, which wording, the organization it named and the channel. Read back
   * by windmill and the Admin Portal; the first release's {@code time}/{@code wording_version} keys
   * stay readable there.
   */
  public String consentJson(
      Instant at, String consentVersion, String organization, MessageChannel channel) {
    ObjectNode consent = new ObjectMapper().createObjectNode();
    consent.put("at", at.toString());
    consent.put("version", consentVersion);
    consent.put("organization", organization);
    consent.put("channel", channel.name());
    return consent.toString();
  }

  /**
   * The consent the voter gave for {@code channel} in this session, as Keycloak validated it; empty
   * when the voter did not consent to that channel.
   */
  public Optional<String> verifiedConsent(
      AuthenticationSessionModel authSession,
      String consentVersion,
      String organization,
      MessageChannel channel,
      Instant at) {
    if (!consentValue(consentVersion, channel)
        .equals(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CONSENT))) {
      return Optional.empty();
    }
    return Optional.of(consentJson(at, consentVersion, organization, channel));
  }

  /**
   * The verified channel and destination of this session, read from notes only Keycloak sets.
   * Messenger needs the confirmed Page-scoped ID and Page; the other channels need the destination
   * the code was sent to (email falls back to the account's address).
   */
  public Optional<VoterChannel> verified(AuthenticationSessionModel authSession, UserModel user) {
    Optional<MessageChannel> verified =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    if (verified.isEmpty()) {
      return Optional.empty();
    }
    MessageChannel channel = verified.get();
    if (channel == MessageChannel.MESSENGER) {
      String id = present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID));
      String page =
          present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE));
      return id == null || page == null
          ? Optional.empty()
          : Optional.of(
              new VoterChannel(channel, id, Optional.of(page), VoterChannel.Source.RECORDED));
    }
    String destination =
        present(authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_DESTINATION));
    if (destination == null && channel == MessageChannel.EMAIL && user != null) {
      destination = present(user.getEmail());
    }
    return destination == null
        ? Optional.empty()
        : Optional.of(
            new VoterChannel(channel, destination, Optional.empty(), VoterChannel.Source.RECORDED));
  }

  /**
   * Saves the contact the code verified, replacing the voter's channel. Nothing is saved when the
   * session verified no channel, or when its destination is missing (logged).
   *
   * @return the saved channel
   */
  public Optional<MessageChannel> persist(
      UserModel user,
      AuthenticationSessionModel authSession,
      String organization,
      String consentVersion,
      Instant now) {
    Optional<VoterChannel> verified = verified(authSession, user);
    if (verified.isEmpty()) {
      if (authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL) != null) {
        log.warn("persist(): the verified channel has no verified destination; nothing saved");
      }
      return Optional.empty();
    }
    VoterChannel channel = verified.get();
    user.setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, channel.channel().name());
    if (NUMBER_CHANNELS.contains(channel.channel())) {
      user.setSingleAttribute(MessagingAttributes.MESSAGE_NUMBER, channel.destination());
    } else {
      user.removeAttribute(MessagingAttributes.MESSAGE_NUMBER);
    }
    if (channel.channel() == MessageChannel.MESSENGER) {
      user.setSingleAttribute(MessagingAttributes.MESSENGER_ID, channel.destination());
      user.setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, channel.messengerPage().get());
    } else {
      user.removeAttribute(MessagingAttributes.MESSENGER_ID);
      user.removeAttribute(MessagingAttributes.MESSENGER_PAGE);
    }
    Optional<String> consent =
        verifiedConsent(authSession, consentVersion, organization, channel.channel(), now);
    if (consent.isPresent()) {
      user.setSingleAttribute(MessagingAttributes.MESSAGE_CONSENT, consent.get());
    } else {
      user.removeAttribute(MessagingAttributes.MESSAGE_CONSENT);
    }
    // The first release's shape: a list of channels and a number per app. Never written again.
    user.removeAttribute(MessagingAttributes.VERIFIED_CHANNELS);
    user.removeAttribute(MessagingAttributes.WHATSAPP_NUMBER);
    user.removeAttribute(MessagingAttributes.VIBER_NUMBER);
    log.infov(
        "persist(): saved channel {0}, consent {1}",
        channel.channel(), consent.isPresent() ? "recorded" : "absent");
    return Optional.of(channel.channel());
  }

  /**
   * Replaces a Messenger voter's account and Page after a Page change, once a sign-in reconnect
   * verified the new account (R7-10). The same writes as saving a verified Messenger contact,
   * except that the voter's consent to Messenger stays: they consented to the channel, which is
   * unchanged.
   */
  public void reconnectMessenger(UserModel user, String pageScopedId, String page) {
    user.setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, MessageChannel.MESSENGER.name());
    user.removeAttribute(MessagingAttributes.MESSAGE_NUMBER);
    user.setSingleAttribute(MessagingAttributes.MESSENGER_ID, pageScopedId);
    user.setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, page);
    user.removeAttribute(MessagingAttributes.VERIFIED_CHANNELS);
    user.removeAttribute(MessagingAttributes.WHATSAPP_NUMBER);
    user.removeAttribute(MessagingAttributes.VIBER_NUMBER);
    log.info("reconnectMessenger(): saved the voter's Messenger account on the event's new Page");
  }

  private String present(String value) {
    return value == null || value.isBlank() ? null : value.trim();
  }
}
