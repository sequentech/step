// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.authentication.requiredactions.TermsAndConditions;
import org.keycloak.common.util.Time;
import org.keycloak.events.Details;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RequiredActionProviderModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.provider.ProviderConfigProperty;

/** Lookups an user using a field */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class LookupAndUpdateUser implements Authenticator, AuthenticatorFactory {

  public static final String PROVIDER_ID = "lookup-and-update-user";
  public static final String SEARCH_ATTRIBUTES = "search-attributes";
  public static final String UNSET_ATTRIBUTES = "unset-attributes";
  public static final String UPDATE_ATTRIBUTES = "update-attributes";
  public static final String AUTO_LOGIN = "auto-login";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate(): start");
    // Retrieve the configuration
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    // Extract the attributes to search and update from the configuration
    String searchAttributes = configMap.get(SEARCH_ATTRIBUTES);
    String unsetAttributes = configMap.get(UNSET_ATTRIBUTES);
    String updateAttributes = configMap.get(UPDATE_ATTRIBUTES);
    boolean autoLogin = Boolean.parseBoolean(configMap.get(AUTO_LOGIN));

    // Parse attributes lists
    List<String> searchAttributesList = parseAttributesList(searchAttributes);
    List<String> unsetAttributesList = parseAttributesList(unsetAttributes);
    List<String> updateAttributesList = parseAttributesList(updateAttributes);

    // Lookup user by attributes in authNotes
    UserModel user = lookupUserByAuthNotes(context, searchAttributesList);

    // check user was found
    if (user == null) {
      log.error("authenticate(): user not found");
      context.attempted();
      return;
    }
    // check user has no credentials yet
    else if (user.credentialManager().getStoredCredentialsStream().count() > 0) {
      log.error("authenticate(): user found but already has credentials");
      context.attempted();
      return;
    }

    // check that the user doesn't have set any of the unset attributes
    boolean unsetAttributesChecked = checkUnsetAttributes(user, context, unsetAttributesList);

    if (!unsetAttributesChecked) {
      log.error("authenticate(): some user unset attributes are set");
      context.attempted();
      return;
    }

    // User was found and is verified to be an updateable user: we then
    // update user attributes and set it as the current auth context user
    // for other authentication models in the authentication flow
    log.info("authenticate(): updating user attributes..");
    updateUserAttributes(user, context, updateAttributesList);
    log.info("authenticate(): done");

    // Success event, similar to RegistrationUserCreation.java in keycloak
    String email = user.getEmail();
    String username = user.getUsername();

    if (context.getRealm().isRegistrationEmailAsUsername()) {
      username = email;
    }

    context
        .getEvent()
        .detail(Details.USERNAME, username)
        .detail(Details.REGISTER_METHOD, "form")
        .detail(Details.EMAIL, email);
    user.setEnabled(true);

    if ("on".equals(context.getAuthenticationSession().getAuthNote("termsAccepted"))) {
      // if accepted terms and conditions checkbox, remove action and add
      // the attribute if enabled
      RequiredActionProviderModel tacModel =
          context
              .getRealm()
              .getRequiredActionProviderByAlias(
                  UserModel.RequiredAction.TERMS_AND_CONDITIONS.name());
      if (tacModel != null && tacModel.isEnabled()) {
        user.setSingleAttribute(
            TermsAndConditions.USER_ATTRIBUTE, Integer.toString(Time.currentTime()));
        context
            .getAuthenticationSession()
            .removeRequiredAction(UserModel.RequiredAction.TERMS_AND_CONDITIONS);
        user.removeRequiredAction(UserModel.RequiredAction.TERMS_AND_CONDITIONS);
      }
    }
    log.info("authenticate(): setUser");
    context.setUser(user);

    String password =
        context.getAuthenticationSession().getAuthNote(RegistrationPage.FIELD_PASSWORD);
    try {
      user.credentialManager().updateCredential(UserCredentialModel.password(password, false));
    } catch (Exception me) {
      user.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD);
    }

    context.getAuthenticationSession().setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, username);

    context.getEvent().user(user);
    context.getEvent().success();
    context.newEvent().event(EventType.LOGIN);
    context
        .getEvent()
        .client(context.getAuthenticationSession().getClient().getClientId())
        .detail(Details.REDIRECT_URI, context.getAuthenticationSession().getRedirectUri())
        .detail(Details.AUTH_METHOD, context.getAuthenticationSession().getProtocol());
    String authType = context.getAuthenticationSession().getAuthNote(Details.AUTH_TYPE);
    if (authType != null) {
      context.getEvent().detail(Details.AUTH_TYPE, authType);
    }
    log.info("authenticate(): success");

    if (autoLogin) {
      context.success();
    } else {
      context.clearUser();
      Response form = context.form().createForm("registration-finish.ftl");
      context.challenge(form);
    }
  }

  private UserModel lookupUserByAuthNotes(
      AuthenticationFlowContext context, List<String> attributes) {
    log.info("lookupUserByAuthNotes(): start");

    return Utils.lookupUserByAuthNotes(context);
  }

  private boolean checkUnsetAttributes(
      UserModel user, AuthenticationFlowContext context, List<String> attributes) {
    Map<String, List<String>> userAttributes = user.getAttributes();
    for (String attributeName : attributes) {
      if (attributeName.equals("email")) {
        // Only assume email is valid if it's verified
        if (user.isEmailVerified() && user.getEmail() != null && !user.getEmail().isBlank()) {
          log.info("checkUnsetAttributes(): user has email=" + user.getEmail());
          return false;
        }
      } else {
        if (userAttributes.containsKey(attributeName)
            && userAttributes.get(attributeName) != null
            && userAttributes.get(attributeName).size() > 0
            && userAttributes.get(attributeName).get(0) != null
            && !userAttributes.get(attributeName).get(0).isBlank()) {
          log.info(
              "checkUnsetAttributes(): user has attribute "
                  + attributeName
                  + " with value="
                  + userAttributes.get(attributeName));
          return false;
        }
      }
    }
    return true;
  }

  private void updateUserAttributes(
      UserModel user, AuthenticationFlowContext context, List<String> attributes) {
    for (String attribute : attributes) {
      List<String> values = Utils.getAttributeValuesFromAuthNote(context, attribute);
      if (values != null && !values.isEmpty()) {
        if (attribute.equals("username")) {
          user.setUsername(values.get(0));
        } else if (attribute.equals("email")) {
          user.setEmail(values.get(0));
        }
        user.setAttribute(attribute, values);
      }
    }
  }

  private List<String> parseAttributesList(String attributes) {
    if (attributes == null || attributes.trim().isEmpty()) {
      return Collections.emptyList();
    }
    return List.of(attributes.split(","));
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action(): start");

    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    boolean autoLogin = Boolean.parseBoolean(configMap.get(AUTO_LOGIN));

    if (autoLogin) {
      context.success();
    }
  }

  @Override
  public boolean requiresUser() {
    // This authenticator does not necessarily require an existing user
    return false;
  }

  @Override
  public boolean configuredFor(
      KeycloakSession session, org.keycloak.models.RealmModel realm, UserModel user) {
    // Applicable for any user
    return true;
  }

  @Override
  public void setRequiredActions(
      KeycloakSession session, org.keycloak.models.RealmModel realm, UserModel user) {
    // No additional required actions
  }

  @Override
  public void close() {
    // No resources to close
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return new LookupAndUpdateUser();
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
  public String getDisplayType() {
    return "Lookup User from Authentication Notes";
  }

  @Override
  public String getHelpText() {
    return "Looks up and optionally updates a user based on attributes stored in authentication notes.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    // Define configuration properties
    return List.of(
        new ProviderConfigProperty(
            SEARCH_ATTRIBUTES,
            "Search Attributes",
            "Comma-separated list of attributes to use for searching the user in auth notes.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UNSET_ATTRIBUTES,
            "Unset Attributes",
            "Comma-separated list of attributes that the user needs to have unset and otherwise the authenticator should fail.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UPDATE_ATTRIBUTES,
            "Update Attributes",
            "Comma-separated list of attributes to update for the user from auth notes.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            AUTO_LOGIN,
            "Login after registration",
            "If enabled the user will automatically login after registration.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            true));
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public String getReferenceCategory() {
    return "user-lookup";
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }
}
