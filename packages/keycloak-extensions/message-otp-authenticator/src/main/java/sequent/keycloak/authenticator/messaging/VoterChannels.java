// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.ArrayList;
import java.util.Collection;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.Utils;

/** Which channels can reach a voter, and where. */
@UtilityClass
public class VoterChannels {
  private final Pattern E164 = Pattern.compile("^\\+[1-9][0-9]{6,14}$");
  private final List<MessageChannel> LEGACY_OTP_CHANNELS =
      List.of(MessageChannel.EMAIL, MessageChannel.SMS);

  public MessageSenderProvider sender(KeycloakSession session) {
    MessageSenderProvider sender = session.getProvider(MessageSenderProvider.class);
    return sender == null ? new DefaultMessageSenderProvider() : sender;
  }

  /** Email and SMS always have Keycloak's own providers; other channels need the sender. */
  public boolean deliverable(MessageSenderProvider sender, MessageChannel channel) {
    return channel == MessageChannel.EMAIL
        || channel == MessageChannel.SMS
        || sender.delivers(channel);
  }

  /**
   * The channels the event offers codes on to a saved voter's elections; email and SMS when it
   * publishes no channels.
   */
  public List<MessageChannel> offeredForOtp(
      Optional<PublicMessagingChannels> projection, Collection<String> electionIds) {
    return offeredForOtp(
        projection, electionIds, PublicMessagingChannels.UnmatchedPostPolicy.EVERY_CHANNEL);
  }

  /** The channels the event offers codes on to the Posts, given what an unknown Post is offered. */
  public List<MessageChannel> offeredForOtp(
      Optional<PublicMessagingChannels> projection,
      Collection<String> posts,
      PublicMessagingChannels.UnmatchedPostPolicy unmatched) {
    return projection
        .map(channels -> channels.channelsFor(MessagePurpose.OTP, posts, unmatched))
        .orElse(LEGACY_OTP_CHANNELS);
  }

  /**
   * Channels a code may be sent on, in display order.
   *
   * @param verified the voter's verified channels at sign-in; empty while enrolling, when the
   *     contact is new and the code verifies it
   */
  public List<MessageChannel> eligibleForOtp(
      List<MessageChannel> offered,
      MessageSenderProvider sender,
      Map<MessageChannel, String> contacts,
      Optional<Set<MessageChannel>> verified) {
    List<MessageChannel> eligible = new ArrayList<>();
    for (MessageChannel channel : offered) {
      boolean reachable =
          contacts.containsKey(channel)
              || (channel == MessageChannel.MESSENGER && verified.isEmpty());
      boolean allowed = verified.map(set -> set.contains(channel)).orElse(true);
      if (deliverable(sender, channel) && reachable && allowed) {
        eligible.add(channel);
      }
    }
    return eligible;
  }

  /** The voter's contacts by channel; blank values are left out. */
  public Map<MessageChannel, String> contacts(UserModel user, String mobileAttribute) {
    Map<MessageChannel, String> contacts = new EnumMap<>(MessageChannel.class);
    put(contacts, MessageChannel.EMAIL, user.getEmail());
    put(contacts, MessageChannel.SMS, user.getFirstAttribute(mobileAttribute));
    put(
        contacts,
        MessageChannel.WHATSAPP,
        user.getFirstAttribute(MessagingAttributes.WHATSAPP_NUMBER));
    put(contacts, MessageChannel.VIBER, user.getFirstAttribute(MessagingAttributes.VIBER_NUMBER));
    put(
        contacts,
        MessageChannel.MESSENGER,
        user.getFirstAttribute(MessagingAttributes.MESSENGER_ID));
    return contacts;
  }

  /** The contacts entered during enrollment, kept as authentication notes. */
  public Map<MessageChannel, String> enrollmentContacts(
      AuthenticationSessionModel authSession, String mobileAttribute) {
    Map<MessageChannel, String> contacts = new EnumMap<>(MessageChannel.class);
    put(contacts, MessageChannel.EMAIL, authSession.getAuthNote("email"));
    put(contacts, MessageChannel.SMS, authSession.getAuthNote(mobileAttribute));
    put(
        contacts,
        MessageChannel.WHATSAPP,
        authSession.getAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER));
    put(
        contacts,
        MessageChannel.VIBER,
        authSession.getAuthNote(MessagingAttributes.FORM_VIBER_NUMBER));
    return contacts;
  }

  private void put(Map<MessageChannel, String> contacts, MessageChannel channel, String value) {
    if (value != null && !value.isBlank()) {
      contacts.put(channel, value.trim());
    }
  }

  /**
   * The voter's verified channels. Voters saved before channels were tracked keep their email and
   * mobile number, which their earlier codes went to.
   */
  public Set<MessageChannel> verified(UserModel user, String mobileAttribute) {
    List<String> values =
        user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS).collect(Collectors.toList());
    Set<MessageChannel> verified = EnumSet.noneOf(MessageChannel.class);
    if (values.isEmpty()) {
      if (present(user.getEmail())) {
        verified.add(MessageChannel.EMAIL);
      }
      if (present(user.getFirstAttribute(mobileAttribute))) {
        verified.add(MessageChannel.SMS);
      }
      return verified;
    }
    for (String value : values) {
      MessageChannel.parse(value).ifPresent(verified::add);
    }
    return verified;
  }

  /** The voter's elections, which restrict the channels their Post offers. */
  public Set<String> elections(UserModel user) {
    return user.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS)
        .filter(VoterChannels::present)
        .collect(Collectors.toSet());
  }

  public boolean isE164(String value) {
    return value != null && E164.matcher(value).matches();
  }

  /** The destination as shown on the code page. Messenger IDs mean nothing to voters. */
  public String mask(MessageChannel channel, String destination) {
    if (destination == null) {
      return "";
    }
    return switch (channel) {
      case EMAIL -> Utils.obscureEmail(destination);
      case SMS, WHATSAPP, VIBER -> Utils.obscurePhoneNumber(destination);
      case MESSENGER -> "";
    };
  }

  public List<String> names(Collection<MessageChannel> channels) {
    return channels.stream().map(Enum::name).collect(Collectors.toList());
  }

  public List<MessageChannel> parseList(String value) {
    List<MessageChannel> channels = new ArrayList<>();
    if (value == null) {
      return channels;
    }
    for (String name : value.split(",")) {
      MessageChannel.parse(name).filter(c -> !channels.contains(c)).ifPresent(channels::add);
    }
    return channels;
  }

  private boolean present(String value) {
    return value != null && !value.isBlank();
  }
}
