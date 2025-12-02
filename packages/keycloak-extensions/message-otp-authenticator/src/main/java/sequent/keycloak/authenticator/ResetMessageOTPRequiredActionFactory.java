// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import java.util.List;
import org.keycloak.Config;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

@AutoService(RequiredActionFactory.class)
public class ResetMessageOTPRequiredActionFactory implements RequiredActionFactory {

  @Override
  public RequiredActionProvider create(KeycloakSession keycloakSession) {
    return new ResetMessageOTPRequiredAction();
  }

  @Override
  public String getDisplayText() {
    return "Reset Message OTP";
  }

  @Override
  public List<ProviderConfigProperty> getConfigMetadata() {
    // TODO Auto-generated method stub
    return RequiredActionFactory.super.getConfigMetadata();
  }

  @Override
  public void init(Config.Scope scope) {}

  @Override
  public void postInit(KeycloakSessionFactory keycloakSessionFactory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return ResetMessageOTPRequiredAction.PROVIDER_ID;
  }
}
