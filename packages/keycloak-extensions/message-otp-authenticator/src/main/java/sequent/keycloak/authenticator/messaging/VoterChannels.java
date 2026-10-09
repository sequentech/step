// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;

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

  /** The voter's elections, which restrict the channels their Post offers. */
  public Set<String> elections(UserModel user) {
    return user.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS)
        .filter(VoterChannels::present)
        .collect(Collectors.toSet());
  }

  public boolean isE164(String value) {
    return value != null && E164.matcher(value).matches();
  }

  /**
   * The destination as shown to the voter: a number keeps its country code and last three digits
   * ({@code +966 ••• ••• 567}), an email address its first two characters and its domain ({@code
   * ma••••••••@example.com}). Messenger IDs mean nothing to voters and are not shown.
   */
  public String mask(MessageChannel channel, String destination) {
    if (destination == null || destination.isBlank()) {
      return "";
    }
    return switch (channel) {
      case EMAIL -> maskEmail(destination.trim());
      case SMS, WHATSAPP, VIBER -> maskNumber(destination.trim());
      case MESSENGER -> "";
    };
  }

  private final String HIDDEN = "\u2022";

  /** Country calling codes of two digits (ITU-T E.164 zones); 1 and 7 have one, others three. */
  private final Set<String> TWO_DIGIT_CODES =
      Set.of(
          "20", "27", "30", "31", "32", "33", "34", "36", "39", "40", "41", "43", "44", "45", "46",
          "47", "48", "49", "51", "52", "53", "54", "55", "56", "57", "58", "60", "61", "62", "63",
          "64", "65", "66", "81", "82", "84", "86", "90", "91", "92", "93", "94", "95", "98");

  /** The length of the country calling code a number in E.164 starts with. */
  int callingCodeLength(String digits) {
    if (digits.startsWith("1") || digits.startsWith("7")) {
      return 1;
    }
    if (digits.length() >= 2 && TWO_DIGIT_CODES.contains(digits.substring(0, 2))) {
      return 2;
    }
    return 3;
  }

  String maskNumber(String number) {
    String digits = number.startsWith("+") ? number.substring(1) : number;
    if (!digits.matches("[0-9]{4,}")) {
      return HIDDEN.repeat(3);
    }
    int codeLength = Math.min(callingCodeLength(digits), digits.length() - 1);
    String code = digits.substring(0, codeLength);
    String national = digits.substring(codeLength);
    if (national.length() <= 3) {
      return "+" + code + " " + HIDDEN.repeat(national.length());
    }
    int hidden = national.length() - 3;
    StringBuilder groups = new StringBuilder();
    int first = hidden % 3 == 0 ? 3 : hidden % 3;
    groups.append(HIDDEN.repeat(first));
    for (int left = hidden - first; left > 0; left -= 3) {
      groups.append(' ').append(HIDDEN.repeat(3));
    }
    return "+" + code + " " + groups + " " + national.substring(national.length() - 3);
  }

  String maskEmail(String email) {
    int at = email.indexOf('@');
    if (at <= 0) {
      return HIDDEN.repeat(3);
    }
    String local = email.substring(0, at);
    int shown = Math.min(2, Math.max(1, local.length() - 1));
    return local.substring(0, shown) + HIDDEN.repeat(local.length() - shown) + email.substring(at);
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
