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
   * The destination as records, logs and screens show it, exactly like Rust's {@code
   * Destination::masked} ({@code messaging/src/destination.rs}): a number keeps its country calling
   * code and the last three digits ({@code +966 ••• ••• 567}), an email address at most its first
   * two characters, never all of its local part, and its domain ({@code ma••••••••@example.com}), a
   * Page-scoped ID its last three digits ({@code ••• 321}). The hidden parts have a fixed length,
   * so they don't reveal the address's length. An address Rust would refuse shows only {@code •••};
   * none shows nothing.
   */
  public String mask(MessageChannel channel, String destination) {
    if (destination == null || destination.isBlank()) {
      return "";
    }
    return switch (channel) {
      case EMAIL -> maskEmail(destination);
      case SMS, WHATSAPP, VIBER -> maskNumber(destination);
      case MESSENGER -> maskPageScopedId(destination);
    };
  }

  /**
   * The destination as a voter page shows it: {@link #mask}, except that a Messenger voter sees the
   * Page's name instead of an ID that means nothing to them.
   */
  public String display(MessageChannel channel, String destination) {
    return channel == MessageChannel.MESSENGER ? "" : mask(channel, destination);
  }

  private final String BULLETS = "\u2022\u2022\u2022";

  /**
   * Country calling codes (ITU-T E.164) and the global services' codes: sequent-core's {@code
   * dial_codes::CALLING_CODES} and {@code NON_GEOGRAPHIC_CODES} (a test keeps them equal). They are
   * prefix-free, so the first match by length is the code.
   */
  public final Set<String> CALLING_CODES =
      Set.of(
          ("1 7 20 27 30 31 32 33 34 36 39 40 41 43 44 45 46 47 48 49 51 52 53 54 55 56 57 58 60"
                  + " 61 62 63 64 65 66 81 82 84 86 90 91 92 93 94 95 98 211 212 213 216 218 220"
                  + " 221 222 223 224 225 226 227 228 229 230 231 232 233 234 235 236 237 238 239"
                  + " 240 241 242 243 244 245 246 247 248 249 250 251 252 253 254 255 256 257 258"
                  + " 260 261 262 263 264 265 266 267 268 269 290 291 297 298 299 350 351 352 353"
                  + " 354 355 356 357 358 359 370 371 372 373 374 375 376 377 378 379 380 381 382"
                  + " 383 385 386 387 389 420 421 423 500 501 502 503 504 505 506 507 508 509 590"
                  + " 591 592 593 594 595 596 597 598 599 670 672 673 674 675 676 677 678 679 680"
                  + " 681 682 683 685 686 687 688 689 690 691 692 850 852 853 855 856 880 886 960"
                  + " 961 962 963 964 965 966 967 968 970 971 972 973 974 975 976 977 992 993 994"
                  + " 995 996 998 800 808 870 878 881 882 883 888 979")
              .split(" "));

  /** The calling code of an E.164 number's digits, as Rust's {@code dial_codes::calling_code}. */
  Optional<String> callingCode(String digits) {
    for (int length = 1; length <= 3 && length <= digits.length(); length++) {
      String prefix = digits.substring(0, length);
      if (CALLING_CODES.contains(prefix)) {
        return Optional.of(prefix);
      }
    }
    return Optional.empty();
  }

  /** Rust's {@code normalize_phone}, then the mask; {@code •••} for what it refuses. */
  String maskNumber(String number) {
    StringBuilder compact = new StringBuilder();
    for (char c : number.toCharArray()) {
      if (c != ' ' && c != '-' && c != '(' && c != ')' && c != '.') {
        compact.append(c);
      }
    }
    if (compact.length() == 0 || compact.charAt(0) != '+') {
      return BULLETS;
    }
    String digits = compact.substring(1);
    if (digits.length() < 8
        || digits.length() > 15
        || !digits.chars().allMatch(VoterChannels::ascii)) {
      return BULLETS;
    }
    Optional<String> code = callingCode(digits);
    if (code.isEmpty()) {
      return BULLETS;
    }
    String national = digits.substring(code.get().length());
    return "+" + code.get() + " " + BULLETS + " " + BULLETS + " " + lastThree(national);
  }

  /** Rust's {@code normalize_email}, then the mask; {@code •••} for what it refuses. */
  String maskEmail(String raw) {
    String email = MessageChannel.trim(raw).toLowerCase(java.util.Locale.ROOT);
    int at = email.indexOf('@');
    if (at <= 0 || email.chars().anyMatch(c -> MessageChannel.isWhitespace((char) c))) {
      return BULLETS;
    }
    String local = email.substring(0, at);
    String domain = email.substring(at + 1);
    if (!domain.contains(".") || domain.startsWith(".")) {
      return BULLETS;
    }
    int characters = local.codePointCount(0, local.length());
    int shown = Math.min(2, characters - 1);
    String first = local.substring(0, local.offsetByCodePoints(0, shown));
    return first + BULLETS + BULLETS + "\u2022\u2022@" + domain;
  }

  /** Rust's {@code normalize_page_scoped_id} (digits only), then {@code ••• 321}. */
  String maskPageScopedId(String raw) {
    String id = MessageChannel.trim(raw);
    if (id.isEmpty() || !id.chars().allMatch(VoterChannels::ascii)) {
      return BULLETS;
    }
    return BULLETS + " " + lastThree(id);
  }

  private String lastThree(String value) {
    return value.substring(Math.max(0, value.length() - 3));
  }

  private boolean ascii(int c) {
    return c >= '0' && c <= '9';
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
