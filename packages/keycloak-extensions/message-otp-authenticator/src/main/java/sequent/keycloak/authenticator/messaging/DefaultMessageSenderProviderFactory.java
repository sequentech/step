// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

@AutoService(MessageSenderProviderFactory.class)
public class DefaultMessageSenderProviderFactory implements MessageSenderProviderFactory {
  public static final String PROVIDER_ID = "default";

  @Override
  public MessageSenderProvider create(KeycloakSession session) {
    return new DefaultMessageSenderProvider();
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  /** Above harvest and below auto, which is the sender when no provider is configured. */
  @Override
  public int order() {
    return 1;
  }
}
