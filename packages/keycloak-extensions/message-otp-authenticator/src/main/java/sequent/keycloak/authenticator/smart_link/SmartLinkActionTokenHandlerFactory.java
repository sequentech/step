// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

@AutoService(ActionTokenHandlerFactory.class)
public class SmartLinkActionTokenHandlerFactory
    implements ActionTokenHandlerFactory<SmartLinkActionToken> {
  public static final String PROVIDER_ID = "smart-link";

  @Override
  public void close() {}

  @Override
  public SmartLinkActionTokenHandler create(KeycloakSession session) {
    return new SmartLinkActionTokenHandler();
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public void init(Config.Scope config) {}
}
