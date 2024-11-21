// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.credential;

import org.jboss.logging.Logger;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.credential.CredentialProviderFactory;
import org.keycloak.models.KeycloakSession;

public class MessageOTPCredentialProviderFactory
    implements CredentialProviderFactory<MessageOTPCredentialProvider> {
  private static final Logger logger = Logger.getLogger(MessageOTPCredentialProviderFactory.class);

  public static final String PROVIDER_ID = "message-otp-credential";

  @Override
  public CredentialProvider<MessageOTPCredentialModel> create(KeycloakSession session) {
    logger.info("create()");
    return new MessageOTPCredentialProvider(session);
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
