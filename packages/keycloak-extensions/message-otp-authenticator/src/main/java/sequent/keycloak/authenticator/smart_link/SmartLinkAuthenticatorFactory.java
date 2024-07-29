// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.google.auto.service.AutoService;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.provider.ProviderEvent;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class SmartLinkAuthenticatorFactory implements AuthenticatorFactory {

  public static final String PROVIDER_ID = "smart-link";

  private static final AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public Authenticator create(KeycloakSession session) {
    return new SmartLinkAuthenticator();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getReferenceCategory() {
    return "alternate-auth";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return true;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public String getDisplayType() {
    return "Smart Link";
  }

  @Override
  public String getHelpText() {
    return "Sign in with a smart link that will be sent to your email and/or sms or through an external service that calls a resource API to generate the link.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            SmartLinkAuthenticator.CREATE_NONEXISTENT_USER_CONFIG_PROPERTY,
            "Force create user",
            "Creates a new user when an email is provided that does not match an existing user.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            true),
        new ProviderConfigProperty(
            SmartLinkAuthenticator.UPDATE_PROFILE_ACTION_CONFIG_PROPERTY,
            "Update profile on create",
            "Add an UPDATE_PROFILE required action if the user was created.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            SmartLinkAuthenticator.UPDATE_PASSWORD_ACTION_CONFIG_PROPERTY,
            "Update password on create",
            "Add an UPDATE_PASSWORD required action if the user was created.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            SmartLinkAuthenticator.ACTION_TOKEN_PERSISTENT_CONFIG_PROPERTY,
            "Allow magic link to be reusable",
            "Toggle whether magic link should be persistent until expired.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            true));
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    factory.register(
        (ProviderEvent event) -> {
          if (event instanceof RealmModel.RealmPostCreateEvent) {
            SmartLink.realmPostCreate(factory, (RealmModel.RealmPostCreateEvent) event);
          }
        });
  }

  @Override
  public void close() {}
}
