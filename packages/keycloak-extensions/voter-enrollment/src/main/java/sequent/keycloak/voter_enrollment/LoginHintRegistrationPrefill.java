// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.keycloak.Config;
import org.keycloak.authentication.FormAction;
import org.keycloak.authentication.FormActionFactory;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

@AutoService(FormActionFactory.class)
public class LoginHintRegistrationPrefill implements FormAction, FormActionFactory {

  public static final String PROVIDER_ID = "login-hint-registration-prefill";

  @Override
  public void buildPage(FormContext context, LoginFormsProvider form) {
    if (!"GET".equals(context.getHttpRequest().getHttpMethod())) {
      return;
    }

    Map<String, String> hints;
    try {
      hints = LoginHintPrefill.extractHints(context.getAuthenticationSession().getClientNotes());
    } catch (IllegalArgumentException invalidHints) {
      return;
    }

    if (hints.isEmpty()) {
      return;
    }

    MultivaluedMap<String, String> candidateFormData = new MultivaluedHashMap<>();
    hints.forEach(candidateFormData::putSingle);
    UserProfile profile =
        context
            .getSession()
            .getProvider(UserProfileProvider.class)
            .create(UserProfileContext.REGISTRATION, candidateFormData);
    MultivaluedMap<String, String> writableHints =
        LoginHintPrefill.filterWritableHints(hints, profile.getAttributes(), Set.of());

    if (!writableHints.isEmpty()) {
      form.setFormData(writableHints);
    }
  }

  @Override
  public void validate(ValidationContext context) {
    context.success();
  }

  @Override
  public void success(FormContext context) {}

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public FormAction create(KeycloakSession session) {
    return this;
  }

  @Override
  public String getDisplayType() {
    return "Sequent: Login hint registration prefill";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
    return false;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return new AuthenticationExecutionModel.Requirement[] {
      AuthenticationExecutionModel.Requirement.REQUIRED,
      AuthenticationExecutionModel.Requirement.DISABLED
    };
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Prefills managed writable registration attributes from validated login hint parameters. Place this action before registration user creation.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}
}
