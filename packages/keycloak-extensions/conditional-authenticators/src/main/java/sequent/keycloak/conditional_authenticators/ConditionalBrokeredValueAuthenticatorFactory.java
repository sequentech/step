// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

@AutoService(AuthenticatorFactory.class)
public class ConditionalBrokeredValueAuthenticatorFactory
    implements ConditionalAuthenticatorFactory {
  public static final String PROVIDER_ID = "conditional-brokered-value";
  public static final String CONF_TARGET = "target";
  public static final String CONF_ATTRIBUTE = "attribute";
  public static final String CONF_REGEX = "regex";
  public static final String CONF_NEGATE = "negate";
  private static final Requirement[] REQUIREMENT_CHOICES = {
    Requirement.REQUIRED, Requirement.DISABLED
  };

  /** Which value of the login the condition checks. */
  public enum Target {
    USERNAME,
    USER_ATTRIBUTE
  }

  static Target target(Map<String, String> config) {
    String value = config.get(CONF_TARGET);
    if (value == null || value.isEmpty()) {
      return Target.USER_ATTRIBUTE;
    }
    try {
      return Target.valueOf(value);
    } catch (IllegalArgumentException e) {
      throw new IllegalStateException("Invalid target '" + value + "' in brokered value condition");
    }
  }

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
    return "Condition - Brokered Value";
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
    return "Matches when the username or a user attribute is present and matches the regex. In"
        + " identity provider login flows it checks the values the mappers produced, before the"
        + " user is created.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    ProviderConfigProperty target =
        new ProviderConfigProperty(
            CONF_TARGET,
            "Target",
            "USERNAME checks the username. USER_ATTRIBUTE checks the attribute below.",
            ProviderConfigProperty.LIST_TYPE,
            Target.USER_ATTRIBUTE.name());
    target.setOptions(Arrays.stream(Target.values()).map(Enum::name).toList());
    return List.of(
        target,
        new ProviderConfigProperty(
            CONF_ATTRIBUTE,
            "Attribute",
            "User attribute to check. 'email', 'firstName' and 'lastName' check those user"
                + " properties.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            CONF_REGEX,
            "Regex",
            "Optional. The value must match it entirely. Without a regex, any non-empty value"
                + " matches.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            CONF_NEGATE,
            "Negate output",
            "Apply a NOT to the check result, for example to deny access when the value does not"
                + " match.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false));
  }

  @Override
  public ConditionalAuthenticator getSingleton() {
    return ConditionalBrokeredValueAuthenticator.SINGLETON;
  }
}
