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

  public static Optional<MessageChannel> parse(String value) {
    if (value == null) {
      return Optional.empty();
    }
    for (MessageChannel channel : values()) {
      if (channel.name().equalsIgnoreCase(value.trim())) {
        return Optional.of(channel);
      }
    }
    return Optional.empty();
  }

  /** Message key of the channel's display name, e.g. {@code messageChannel.WHATSAPP}. */
  public String labelKey() {
    return "messageChannel." + name();
  }
}
