// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.io.IOException;
import java.util.Set;

/**
 * Delivers nothing itself: email and SMS keep using Keycloak's email provider and the configured
 * SMS sender, and no messaging-app channel is offered.
 */
public class DefaultMessageSenderProvider implements MessageSenderProvider {
  static final String UNSUPPORTED = "unsupported-channel";

  @Override
  public Set<MessageChannel> getChannels() {
    return Set.of();
  }

  @Override
  public SendMessageResponse send(SendMessageRequest request) {
    return SendMessageResponse.of(MessageAttemptState.FAILED, UNSUPPORTED);
  }

  @Override
  public CreateMessengerLinkResponse createMessengerLink(CreateMessengerLinkRequest request)
      throws IOException {
    throw new IOException(UNSUPPORTED);
  }

  @Override
  public MessengerLinkStatus messengerLinkStatus(MessengerLinkRequest request) throws IOException {
    throw new IOException(UNSUPPORTED);
  }

  @Override
  public MessengerLinkStatus confirmMessengerLink(MessengerLinkRequest request) throws IOException {
    throw new IOException(UNSUPPORTED);
  }
}
