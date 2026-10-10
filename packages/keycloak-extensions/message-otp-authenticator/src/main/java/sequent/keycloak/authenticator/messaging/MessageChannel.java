// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.Optional;

/** Mirrors sequent-core's {@code MessageChannel}; the names are the wire values. */
public enum MessageChannel {
  EMAIL,
  SMS,
  WHATSAPP,
  VIBER,
  MESSENGER;

  /**
   * A channel as a record holds it, read exactly like sequent-core's {@code parse_channel_value}:
   * Unicode whitespace trimmed ({@code str::trim}) and ASCII letters in any case ({@code
   * to_ascii_uppercase}). Anything else is no channel.
   */
  public static Optional<MessageChannel> parse(String value) {
    if (value == null) {
      return Optional.empty();
    }
    String name = asciiUppercase(trim(value));
    for (MessageChannel channel : values()) {
      if (channel.name().equals(name)) {
        return Optional.of(channel);
      }
    }
    return Optional.empty();
  }

  /** Rust's {@code str::trim}: removes leading and trailing Unicode {@code White_Space}. */
  static String trim(String value) {
    int start = 0;
    int end = value.length();
    while (start < end && isWhitespace(value.charAt(start))) {
      start++;
    }
    while (end > start && isWhitespace(value.charAt(end - 1))) {
      end--;
    }
    return value.substring(start, end);
  }

  /**
   * Rust's {@code char::is_whitespace}, the Unicode {@code White_Space} property (all in the BMP).
   * Unlike Java's {@code String.trim}, other control characters are kept; unlike {@code
   * Character.isWhitespace}, no-break spaces are whitespace.
   */
  static boolean isWhitespace(char c) {
    return (c >= 0x09 && c <= 0x0D)
        || c == 0x20
        || c == 0x85
        || c == 0xA0
        || c == 0x1680
        || (c >= 0x2000 && c <= 0x200A)
        || c == 0x2028
        || c == 0x2029
        || c == 0x202F
        || c == 0x205F
        || c == 0x3000;
  }

  /** Rust's {@code to_ascii_uppercase}: only {@code a}–{@code z} change. */
  private static String asciiUppercase(String value) {
    StringBuilder upper = new StringBuilder(value.length());
    for (int i = 0; i < value.length(); i++) {
      char c = value.charAt(i);
      upper.append(c >= 'a' && c <= 'z' ? (char) (c - ('a' - 'A')) : c);
    }
    return upper.toString();
  }

  /** Message key of the channel's display name, e.g. {@code messageChannel.WHATSAPP}. */
  public String labelKey() {
    return "messageChannel." + name();
  }
}
