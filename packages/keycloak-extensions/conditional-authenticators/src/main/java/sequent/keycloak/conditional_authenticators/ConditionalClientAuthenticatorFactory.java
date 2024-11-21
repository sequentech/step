// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import java.util.List;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

@AutoService(AuthenticatorFactory.class)
public class ConditionalClientAuthenticatorFactory implements ConditionalAuthenticatorFactory {
  public static final String PROVIDER_ID = "conditional-client";

  public static final String CONDITIONAL_CLIENT_ID = "conditional-client";
  public static final String CONF_NEGATE = "negate";

  private static final Requirement[] REQUIREMENT_CHOICES = {
    Requirement.REQUIRED, Requirement.DISABLED
  };

  @Override
  public void init(Scope config) {
    // no-op
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    // no-op
  }

  @Override
  public void close() {
    // no-op
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Condition - Client Id";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Flow is executed only if authentication is performed by the correct client.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONDITIONAL_CLIENT_ID,
            "Client",
            "Client id that should be executing this flow.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            CONF_NEGATE,
            "Negate output",
            "Apply a NOT to the check result. When this is true, then the condition will evaluate to true just if the authentication is NOT performed using the specified client id. When this is false, the condition will evaluate to true just if the authentication performed using the specified client id.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false));
  }

  @Override
  public ConditionalAuthenticator getSingleton() {
    return ConditionalClientAuthenticator.SINGLETON;
  }
}
