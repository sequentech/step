// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;
import org.keycloak.Config;
import org.keycloak.authentication.FormAction;
import org.keycloak.authentication.FormActionFactory;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.provider.ProviderConfigProperty;

@AutoService(FormActionFactory.class)
public class LoginHintRegistrationPrefill implements FormAction, FormActionFactory {

  public static final String PROVIDER_ID = "login-hint-registration-prefill";
  public static final String READ_ONLY_ATTRIBUTES = "loginHintReadOnlyAttributes";

  @Override
  public void buildPage(FormContext context, LoginFormsProvider form) {
    LoginHintPrefill.Prefill prefill =
        LoginHintPrefill.requireValid(resolvePrefill(context), () -> form);

    if (prefill.isEmpty()) {
      return;
    }

    // The marker is set on every render so locked fields stay locked when the
    // form is redisplayed with validation errors.
    if (!prefill.lockedAttributes().isEmpty()) {
      form.setAttribute(READ_ONLY_ATTRIBUTES, List.copyOf(prefill.lockedAttributes()));
    }

    // Only the initial render prefills, so a redisplayed form keeps what the voter typed.
    if ("GET".equals(context.getHttpRequest().getHttpMethod())) {
      form.setFormData(prefill.writableHints());
    }
  }

  @Override
  public void validate(ValidationContext context) {
    LoginHintPrefill.Prefill prefill =
        LoginHintPrefill.requireValid(
            resolvePrefill(context),
            () ->
                context
                    .getSession()
                    .getProvider(LoginFormsProvider.class)
                    .setAuthenticationSession(context.getAuthenticationSession()));

    if (prefill.lockedAttributes().isEmpty()) {
      context.success();
      return;
    }

    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    Set<String> modifiedAttributes =
        LoginHintPrefill.findModifiedLockedHints(
            prefill.writableHints(), prefill.lockedAttributes(), formData);

    if (modifiedAttributes.isEmpty()) {
      context.success();
      return;
    }

    List<FormMessage> errors =
        modifiedAttributes.stream()
            .map(
                attributeName ->
                    new FormMessage(
                        attributeName, LoginHintPrefill.READ_ONLY_FIELD_MODIFIED_MESSAGE))
            .collect(Collectors.toList());
    context.error(Errors.INVALID_REGISTRATION);
    context.validationError(
        LoginHintPrefill.restoreLockedHints(formData, prefill.writableHints(), modifiedAttributes),
        errors);
  }

  private LoginHintPrefill.HintResolution resolvePrefill(FormContext context) {
    return LoginHintPrefill.resolve(
        context.getSession(), context.getAuthenticationSession().getClientNotes(), Set.of());
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
    return "Prefills managed writable registration attributes from validated login hint parameters. Set the loginHintPrefillPolicy user profile annotation to EDITABLE, READ_ONLY or IGNORE to configure an attribute. Place this action before registration user creation.";
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
