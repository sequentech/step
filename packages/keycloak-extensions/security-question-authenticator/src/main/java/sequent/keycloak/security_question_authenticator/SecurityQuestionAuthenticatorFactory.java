// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.security_question_authenticator;

import com.google.auto.service.AutoService;
import java.util.List;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

@AutoService(AuthenticatorFactory.class)
public class SecurityQuestionAuthenticatorFactory implements AuthenticatorFactory {
  public static final String PROVIDER_ID = "security-question-authenticator";

  private static final SecurityQuestionAuthenticator SINGLETON =
      new SecurityQuestionAuthenticator();

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Security Question Authentication";
  }

  @Override
  public String getHelpText() {
    return "Validates that the user knows some user attribute value.";
  }

  @Override
  public String getReferenceCategory() {
    return "Secret Question";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return true;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            Utils.USER_ATTRIBUTE,
            "User Attribute",
            "The user attribute to check against.",
            ProviderConfigProperty.STRING_TYPE,
            "read-only.mobile-number"),
        new ProviderConfigProperty(
            Utils.NUM_LAST_CHARS,
            "Number of last characters to check",
            "The number of characters at the end of the user attribute to validate.",
            ProviderConfigProperty.STRING_TYPE,
            "4"));
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}
}
