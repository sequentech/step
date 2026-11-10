// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.sign_in_links;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import sequent.keycloak.login_bridge.LoginBridgeActionToken;

@AutoService(ActionTokenHandlerFactory.class)
public class LoginBridgeActionTokenHandlerFactory
    implements ActionTokenHandlerFactory<LoginBridgeActionToken> {
  public static final String PROVIDER_ID = LoginBridgeActionToken.TOKEN_TYPE;

  @Override
  public void close() {}

  @Override
  public LoginBridgeActionTokenHandler create(KeycloakSession session) {
    return new LoginBridgeActionTokenHandler();
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
