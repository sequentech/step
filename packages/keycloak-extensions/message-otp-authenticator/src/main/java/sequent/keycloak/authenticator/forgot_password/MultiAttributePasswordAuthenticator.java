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

    // A browser authentication session survives failureChallenge(), so a retry may still carry
    // the user attributed by the previous attempt. Clear it before resolving the new submission;
    // otherwise setUser() rejects a different unique candidate as USER_CONFLICT, and failures
    // without an attributable candidate can be charged to the stale account.
    context.clearUser();

    List<String> matchAttributes =
        Utils.getMultivalueString(
            context.getAuthenticatorConfig(),
            Utils.MATCH_ATTRIBUTES,
            Utils.MATCH_ATTRIBUTES_DEFAULT);
    Map<String, String> submittedValues =
        collectSubmittedValues(context.getSession(), matchAttributes, formData);
    String password = formData.getFirst(FIELD_PASSWORD);

    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        Utils.getThrottleConfig(context.getAuthenticatorConfig());
    MultiAttributeCredentialResolver.MatchPolicy matchPolicy =
        Utils.getMatchPolicy(context.getAuthenticatorConfig());
    MultiAttributeCredentialResolver.Resolution result =
        resolveAuthenticatedUser(
            context.getSession(),
            context.getRealm(),
            matchAttributes,
            submittedValues,
            password,
            throttleConfig,
            matchPolicy);

    // Set even on failure/lockout, before signaling the outcome - Keycloak's brute-force
    // accounting (DefaultAuthenticationFlow -> AuthenticationProcessor.logFailure()) only fires
    // for a user set on the authentication session, same as the standard username/password form.
    result.attributableUser().ifPresent(context::setUser);

    if (result.authenticatedUser().isPresent()) {
      context.success();
    } else if (result.lockoutState() != MultiAttributeCredentialResolver.LockoutState.NONE) {
      lockedOut(context, formData, result.lockoutState());
    } else {
      fail(context, formData);
    }
  }

  private void fail(AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);
    Response challengeResponse = challenge(context, formData, Messages.INVALID_USER);
    context.failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, challengeResponse);
    context.clearUser();
  }

  /**
   * Mirrors {@code AbstractUsernameFormAuthenticator.isDisabledByBruteForce}: uses {@code
   * forceChallenge} rather than {@code failureChallenge} so the flow engine doesn't log yet another
   * failure against an account that's already locked out.
   */
  private void lockedOut(
      AuthenticationFlowContext context,
      MultivaluedMap<String, String> formData,
      MultiAttributeCredentialResolver.LockoutState state) {
    boolean permanent = state == MultiAttributeCredentialResolver.LockoutState.PERMANENT;
    context.getEvent().error(permanent ? Errors.USER_DISABLED : Errors.USER_TEMPORARILY_DISABLED);
    Response challengeResponse =
        challenge(
            context,
            formData,
            permanent
                ? Messages.ACCOUNT_PERMANENTLY_DISABLED
                : Messages.ACCOUNT_TEMPORARILY_DISABLED);
    context.forceChallenge(challengeResponse);
    context.clearUser();
  }

  /**
   * Resolves the single user matching every configured attribute AND the submitted password. See
   * {@link MultiAttributeCredentialResolver} for the resolution rules (shared with the IVR Direct
   * Grant authenticator).
   */
  protected MultiAttributeCredentialResolver.Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password) {
    return MultiAttributeCredentialResolver.resolveAuthenticatedUser(
        session, realm, matchAttributes, submittedValues, password);
  }

  /** Overload used by {@link #action} once the DoS-mitigation config has been read. */
  protected MultiAttributeCredentialResolver.Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password,
      MultiAttributeCredentialResolver.ThrottleConfig throttleConfig) {
    return MultiAttributeCredentialResolver.resolveAuthenticatedUser(
        session, realm, matchAttributes, submittedValues, password, throttleConfig);
  }

  /**
   * Overload used by {@link #action} once the DoS-mitigation and match-policy config has been read.
   */
  protected MultiAttributeCredentialResolver.Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password,
      MultiAttributeCredentialResolver.ThrottleConfig throttleConfig,
      MultiAttributeCredentialResolver.MatchPolicy matchPolicy) {
    return MultiAttributeCredentialResolver.resolveAuthenticatedUser(
        session, realm, matchAttributes, submittedValues, password, throttleConfig, matchPolicy);
  }

  /**
   * Reads each configured attribute's submitted value, normalizing date-typed attributes (per the
   * realm's User Profile {@code html5-date} annotation, the same source {@link
   * #buildAttributeFields} reads) into the canonical {@code YYYY-MM-DD} storage format - see {@link
   * Utils#normalizeDate}. An HTML5 date input already submits exactly {@code YYYY-MM-DD}, so this
   * is a no-op for the common case; it's still applied defensively so the browser path stays
   * consistent with the IVR path, which needs real reordering.
   */
  protected Map<String, String> collectSubmittedValues(
      KeycloakSession session,
      List<String> matchAttributes,
      MultivaluedMap<String, String> formData) {
    List<UPAttribute> profileAttributes = Utils.getRealmUserProfileAttributes(session);
    Map<String, String> submittedValues = new HashMap<>();
    for (String attribute : matchAttributes) {
      String rawValue = formData.getFirst(attribute);
      if ("date".equals(Utils.resolveHtml5InputType(profileAttributes, attribute))) {
        rawValue = Utils.normalizeDate(rawValue, "YYYY-MM-DD");
      }
      submittedValues.put(attribute, rawValue);
    }
    return submittedValues;
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
   * Builds the field metadata the template renders one input per configured attribute from. {@code
   * type} mirrors whatever HTML5 input type (e.g. {@code date}) the realm's User Profile
   * configuration declares for that attribute, so a field like {@code dateOfBirth} renders the same
   * native date picker here as it does at registration. A configured {@code inputTypeMax} is
   * forwarded as {@code max}; the template supplies its date default when this key is absent. See
   * {@link Utils#resolveHtml5InputType}. Fetches the User Profile attribute list once up front
   * rather than once per configured attribute.
   */
  protected List<Map<String, String>> buildAttributeFields(
      KeycloakSession session, List<String> matchAttributes) {
    List<UPAttribute> profileAttributes = Utils.getRealmUserProfileAttributes(session);
    List<Map<String, String>> fields = new ArrayList<>();
    for (String attribute : matchAttributes) {
      Map<String, String> field = new HashMap<>();
      field.put("name", attribute);
      field.put("type", Utils.resolveHtml5InputType(profileAttributes, attribute));
      profileAttributes.stream()
          .filter(profileAttribute -> attribute.equals(profileAttribute.getName()))
          .findFirst()
          .map(UPAttribute::getAnnotations)
          .map(annotations -> annotations.get("inputTypeMax"))
          .filter(String.class::isInstance)
          .map(String.class::cast)
          .filter(max -> !max.isBlank())
          .ifPresent(max -> field.put("max", max));
      fields.add(field);
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
    ProviderConfigProperty matchPolicy =
        new ProviderConfigProperty(
            Utils.MATCH_POLICY,
            "Multiple-candidate match policy",
            "How to resolve when more than one candidate shares the configured attribute"
                + " value(s). REJECT_AMBIGUOUS (default, safe): only succeed if the submitted"
                + " password matches exactly one candidate; if it matches more than one, fail"
                + " generically. FIRST_MATCH: succeed as soon as any candidate's password"
                + " matches, without checking the rest. WARNING: FIRST_MATCH is only secure if"
                + " passwords are guaranteed unique across every candidate this could ever match"
                + " - if two candidates share a password, which one authenticates is unspecified,"
                + " letting one voter log in as another's account. Do not enable it unless"
                + " password uniqueness across candidates is guaranteed.",
            ProviderConfigProperty.LIST_TYPE,
            Utils.MATCH_POLICY_DEFAULT);
    matchPolicy.setOptions(
        List.of(
            MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS.name(),
            MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH.name()));

    return List.of(
        new ProviderConfigProperty(
            Utils.MATCH_ATTRIBUTES,
            "User attributes to match",
            "All of these user attributes must match the submitted values, for the same user."
                + " For example: dateOfBirth or dateOfBirth,nationalId",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            Utils.MAX_CANDIDATES,
            "Max candidates per request",
            "DoS guard: once the configured attributes match more than this many enabled,"
                + " non-locked-out candidates, the request fails generically without checking any"
                + " of their passwords. Bounds the worst-case number of password hashes per"
                + " request. Default: "
                + Utils.MAX_CANDIDATES_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.MAX_CANDIDATES_DEFAULT),
        new ProviderConfigProperty(
            Utils.TUPLE_MAX_FAILURES,
            "Max failures per attribute-value combination",
            "DoS guard: failures allowed for a single submitted combination of attribute values"
                + " (e.g. one date of birth) within the failure window below, before further"
                + " attempts against it are rejected without any user lookup. Default: "
                + Utils.TUPLE_MAX_FAILURES_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.TUPLE_MAX_FAILURES_DEFAULT),
        new ProviderConfigProperty(
            Utils.TUPLE_FAILURE_WINDOW_SECONDS,
            "Failure window (seconds)",
            "Rolling window the failure count above applies over; each new failure against the"
                + " same attribute-value combination resets the window. Default: "
                + Utils.TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT),
        new ProviderConfigProperty(
            Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS,
            "Max user-store rows per attribute lookup",
            "DoS guard: hard ceiling on rows the user store may return for the combined query"
                + " across every configured attribute - bounds worst-case database/memory cost,"
                + " separately from the (much tighter) max candidates guard above, which bounds"
                + " password-hash cost. Since all configured attributes are ANDed together in one"
                + " query, the rows returned are always the true multi-attribute match, so this"
                + " ceiling can only ever discard results in a genuinely pathological case (more"
                + " true combined matches than the ceiling) - keep it well above the largest"
                + " realistic combined match count you'd expect a legitimate voter lookup to"
                + " produce. Default: "
                + Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT),
        matchPolicy);
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }
}
