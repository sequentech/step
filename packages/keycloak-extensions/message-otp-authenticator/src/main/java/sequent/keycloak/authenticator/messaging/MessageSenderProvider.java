// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.io.IOException;
import java.util.Set;
import org.keycloak.provider.Provider;

/**
 * Transports codes and notices on the channels it delivers. Keycloak stays the challenge authority:
 * it generates, validates and replaces codes; a sender only carries them. Channels a sender does
 * not deliver keep using Keycloak's email and SMS providers.
 */
public interface MessageSenderProvider extends Provider {

  /** The channels this sender delivers. */
  Set<MessageChannel> getChannels();

  default boolean delivers(MessageChannel channel) {
    return getChannels().contains(channel);
  }

  /** Never throws: transport problems are reported as FAILED or UNKNOWN outcomes. */
  SendMessageResponse send(SendMessageRequest request);

  CreateMessengerLinkResponse createMessengerLink(CreateMessengerLinkRequest request)
      throws IOException;

  MessengerLinkStatus messengerLinkStatus(MessengerLinkRequest request) throws IOException;

  /** Only call once Keycloak verified the code in the session that created the link. */
  MessengerLinkStatus confirmMessengerLink(MessengerLinkRequest request) throws IOException;

  @Override
  default void close() {}
}
