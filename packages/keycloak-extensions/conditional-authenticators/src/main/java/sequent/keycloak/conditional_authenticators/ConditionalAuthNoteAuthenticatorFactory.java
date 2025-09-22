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
public class ConditionalAuthNoteAuthenticatorFactory implements ConditionalAuthenticatorFactory {
  public static final String PROVIDER_ID = "conditional-auth-note";

  public static final String CONDITIONAL_AUTH_NOTE_KEY = "conditional-auth-note-key";
  public static final String CONDITIONAL_AUTH_NOTE_VALUE = "conditional-auth-note-value";
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
    return "Condition - Auth Note";
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
    return "Flow is executed only if authentication session has the correct Auth Note.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONDITIONAL_AUTH_NOTE_KEY,
            "Auth Note Key",
            "Auth Note Key that should be present in the Auth Session executing this flow.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            CONDITIONAL_AUTH_NOTE_VALUE,
            "Auth Note Value",
            "Auth Note Value that the Auth Note Key should have in the Auth Session executing this flow.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            CONF_NEGATE,
            "Negate output",
            "Apply a NOT to the check result.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false));
  }

  @Override
  public ConditionalAuthenticator getSingleton() {
    return ConditionalAuthNoteAuthenticator.SINGLETON;
  }
}
