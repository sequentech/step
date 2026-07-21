// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.services.messages.Messages;

/**
 * Authenticates a user by matching one or more configured user attributes against submitted form
 * values, all against the same user, plus a password. Does not require a username.
 *
 * <p>Resolution: for each configured {@code matchAttributes} entry, find every user whose attribute
 * equals the submitted value, then intersect those candidate sets across all attributes. If exactly
 * one candidate's password matches the submitted password, that user authenticates. Any other
 * outcome (no candidates, no password match, more than one password match) fails with a generic
 * error to avoid revealing which part of the submission was wrong.
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class MultiAttributePasswordAuthenticator implements Authenticator, AuthenticatorFactory {
  public static final String PROVIDER_ID = "multi-attribute-password-form";

  /**
   * Renders the active theme's own {@code login.ftl} (voting-portal / admin-portal), instead of a
   * bespoke template, so this authenticator gets the same registration link, social-provider
   * buttons, remember-me and password-visibility toggle as the standard login form. {@code
   * login.ftl} renders its single "username" field as one field per {@code matchAttributes} entry
   * when that template attribute is set - see {@link #challenge}.
   */
  public static final String FORM_FTL = "login.ftl";

  public static final String FIELD_PASSWORD = "password";
  public static final MultiAttributePasswordAuthenticator SINGLETON =
      new MultiAttributePasswordAuthenticator();
  public static final Requirement[] REQUIREMENT_CHOICES = {
    Requirement.REQUIRED, Requirement.ALTERNATIVE, Requirement.DISABLED
  };

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    Response challengeResponse = challenge(context, new MultivaluedHashMap<>(), null);
    context.challenge(challengeResponse);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    if (formData.containsKey("cancel")) {
      context.cancelLogin();
      return;
    }

    List<String> matchAttributes =
        Utils.getMultivalueString(
            context.getAuthenticatorConfig(),
            Utils.MATCH_ATTRIBUTES,
            Utils.MATCH_ATTRIBUTES_DEFAULT);
    Map<String, String> submittedValues = new HashMap<>();
    for (String attribute : matchAttributes) {
      submittedValues.put(attribute, formData.getFirst(attribute));
    }
    String password = formData.getFirst(FIELD_PASSWORD);

    Optional<UserModel> user =
        resolveAuthenticatedUser(
            context.getSession(), context.getRealm(), matchAttributes, submittedValues, password);

    if (user.isPresent()) {
      context.setUser(user.get());
      context.success();
    } else {
      fail(context, formData);
    }
  }

  private void fail(AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);
    Response challengeResponse = challenge(context, formData, Messages.INVALID_USER);
    context.failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, challengeResponse);
  }

  /**
   * Resolves the single user matching every configured attribute AND the submitted password.
   *
   * <p>Returns {@link Optional#empty()} for any ambiguous or unmatched outcome (missing
   * configuration, blank input, zero candidates, or zero/multiple password matches) so callers
   * never need to distinguish "no such attributes" from "wrong password" - that distinction must
   * not leak to the end user.
   */
  protected Optional<UserModel> resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password) {
    if (matchAttributes == null || matchAttributes.isEmpty()) {
      log.warn("resolveAuthenticatedUser(): no matchAttributes configured");
      return Optional.empty();
    }
    if (password == null || password.isBlank()) {
      return Optional.empty();
    }

    Map<String, UserModel> candidatesById = null;
    for (String attribute : matchAttributes) {
      String value = submittedValues.get(attribute);
      if (value == null || value.isBlank()) {
        return Optional.empty();
      }
      value = value.trim();

      Map<String, UserModel> matchesForAttribute =
          findUsersByAttribute(session, realm, attribute, value)
              .collect(Collectors.toMap(UserModel::getId, Function.identity(), (a, b) -> a));

      if (candidatesById == null) {
        candidatesById = matchesForAttribute;
      } else {
        candidatesById.keySet().retainAll(matchesForAttribute.keySet());
      }

      if (candidatesById.isEmpty()) {
        return Optional.empty();
      }
    }

    List<UserModel> passwordMatches =
        candidatesById.values().stream()
            .filter(UserModel::isEnabled)
            .filter(candidate -> isPasswordValid(candidate, password))
            .collect(Collectors.toList());

    if (passwordMatches.size() != 1) {
      if (passwordMatches.size() > 1) {
        log.warnv(
            "resolveAuthenticatedUser(): ambiguous match, {0} candidates matched the submitted"
                + " password",
            passwordMatches.size());
      }
      return Optional.empty();
    }

    return Optional.of(passwordMatches.get(0));
  }

  /**
   * Resolves candidates for one configured attribute. {@code username} is always unique in
   * Keycloak, so a single lookup is safe there - but {@code email} is only unique when the realm
   * has {@code duplicateEmailsAllowed} disabled, so it uses the exact-match search API (rather than
   * {@code getUserByEmail}, which returns only one arbitrary match) to keep every candidate in play
   * for the password-disambiguation step in {@link #resolveAuthenticatedUser}.
   */
  protected Stream<UserModel> findUsersByAttribute(
      KeycloakSession session, RealmModel realm, String attribute, String value) {
    if ("email".equalsIgnoreCase(attribute)) {
      return session
          .users()
          .searchForUserStream(realm, Map.of(UserModel.EMAIL, value, UserModel.EXACT, "true"));
    }
    if ("username".equalsIgnoreCase(attribute)) {
      UserModel user = session.users().getUserByUsername(realm, value);
      return user == null ? Stream.empty() : Stream.of(user);
    }
    return session.users().searchForUserByUserAttributeStream(realm, attribute, value);
  }

  protected boolean isPasswordValid(UserModel user, String password) {
    return user.credentialManager().isValid(UserCredentialModel.password(password));
  }

  protected Response challenge(
      AuthenticationFlowContext context, MultivaluedMap<String, String> formData, String error) {
    LoginFormsProvider form = context.form();

    if (formData.size() > 0) {
      form.setFormData(formData);
    }
    if (error != null) {
      form.setError(error);
    }

    List<String> matchAttributes =
        Utils.getMultivalueString(
            context.getAuthenticatorConfig(),
            Utils.MATCH_ATTRIBUTES,
            Utils.MATCH_ATTRIBUTES_DEFAULT);
    form.setAttribute(
        "matchAttributes", buildAttributeFields(context.getSession(), matchAttributes));

    return form.createForm(FORM_FTL);
  }

  /**
   * Builds the {@code {name, type}} pairs the template renders one input per configured attribute
   * from. {@code type} mirrors whatever HTML5 input type (e.g. {@code date}) the realm's User
   * Profile configuration declares for that attribute, so a field like {@code dateOfBirth} renders
   * the same native date picker here as it does at registration - see {@link
   * Utils#resolveHtml5InputType}. Fetches the User Profile attribute list once up front rather than
   * once per configured attribute.
   */
  protected List<Map<String, String>> buildAttributeFields(
      KeycloakSession session, List<String> matchAttributes) {
    List<UPAttribute> profileAttributes = Utils.getRealmUserProfileAttributes(session);
    List<Map<String, String>> fields = new ArrayList<>();
    for (String attribute : matchAttributes) {
      fields.add(
          Map.of(
              "name",
              attribute,
              "type",
              Utils.resolveHtml5InputType(profileAttributes, attribute)));
    }
    return fields;
  }

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
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getReferenceCategory() {
    return PasswordCredentialModel.TYPE;
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
  public String getDisplayType() {
    return "Multi-Attribute + Password Form";
  }

  @Override
  public String getHelpText() {
    return "Authenticates a user by matching one or more configured user attributes (all must"
        + " match the same user) plus a password. Does not require a username.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            Utils.MATCH_ATTRIBUTES,
            "User attributes to match",
            "All of these user attributes must match the submitted values, for the same user."
                + " For example: dateOfBirth or dateOfBirth,nationalId",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null));
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }
}
