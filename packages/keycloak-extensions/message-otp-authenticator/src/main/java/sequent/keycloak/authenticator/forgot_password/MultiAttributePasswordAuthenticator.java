// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
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
import org.keycloak.models.UserSessionModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.sessions.AuthenticationSessionModel;

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
  public static final String EXISTING_USER_SESSION_POLICY = "existingUserSessionPolicy";

  public enum ExistingUserSessionPolicy {
    KEEP,
    TERMINATE_BEFORE_LOGIN;

    public static ExistingUserSessionPolicy fromString(String value) {
      if (value == null || value.isBlank()) {
        return KEEP;
      }
      for (ExistingUserSessionPolicy policy : values()) {
        if (policy.name().equalsIgnoreCase(value)) {
          return policy;
        }
      }
      throw new IllegalArgumentException("No constant with text " + value + " found");
    }
  }

  /**
   * Shown for every failed attempt. Deliberately not Keycloak's {@code invalidUserMessage}: that
   * reads "Invalid username or password", and this form has no username field - the voter matched
   * on profile attributes. Defined in the Sequent theme's message bundles.
   */
  public static final String INVALID_CREDENTIALS_MESSAGE = "invalidCredentialsMessage";

  /**
   * Renders the active theme's own {@code login.ftl} (voting-portal / admin-portal), instead of a
   * bespoke template, so this authenticator gets the same registration link, social-provider
   * buttons, remember-me and password-visibility toggle as the standard login form. {@code
   * login.ftl} renders its single "username" field as one field per {@code matchAttributes} entry
   * when that template attribute is set, looking each one up in the {@code profile} attribute (a
   * {@link LoginBean}) via {@code profile.attributesByName} and rendering it with the same {@code
   * user-profile-commons.ftl} macros {@code register.ftl} uses - see {@link #challenge}.
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
    applyExistingUserSessionPolicy(context);
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
    Set<String> optionalAttributes = optionalAttributes(context, matchAttributes);
    List<String> attributesToMatch =
        effectiveMatchAttributes(matchAttributes, submittedValues, optionalAttributes);
    MultiAttributeCredentialResolver.Resolution result =
        EncryptedAttributeCredential.usesPassword(context.getAuthenticatorConfig())
            ? resolveAuthenticatedUser(
                context.getSession(),
                context.getRealm(),
                attributesToMatch,
                submittedValues,
                password,
                throttleConfig,
                matchPolicy)
            : MultiAttributeCredentialResolver.resolveAuthenticatedUser(
                context.getSession(),
                context.getRealm(),
                attributesToMatch,
                submittedValues,
                password,
                throttleConfig,
                matchPolicy,
                context.getAuthenticatorConfig());

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

  private void applyExistingUserSessionPolicy(AuthenticationFlowContext context) {
    ExistingUserSessionPolicy policy =
        ExistingUserSessionPolicy.fromString(
            Utils.getString(
                context.getAuthenticatorConfig(),
                EXISTING_USER_SESSION_POLICY,
                ExistingUserSessionPolicy.KEEP.name()));
    if (policy != ExistingUserSessionPolicy.TERMINATE_BEFORE_LOGIN) {
      return;
    }

    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
    if (authenticationSession == null) {
      return;
    }

    String sessionId = authenticationSession.getParentSession().getId();
    UserSessionModel existingSession =
        context.getSession().sessions().getUserSession(context.getRealm(), sessionId);
    if (existingSession != null) {
      context.getSession().sessions().removeUserSession(context.getRealm(), existingSession);
    }
  }

  private void fail(AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);
    Response challengeResponse = challenge(context, formData, INVALID_CREDENTIALS_MESSAGE);
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
    Response challengeResponse = challenge(context, formData, INVALID_CREDENTIALS_MESSAGE);
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
   * When {@link Utils#HONOR_USER_PROFILE_REQUIRED} is enabled, returns the subset of {@code
   * matchAttributes} the realm's User Profile does <em>not</em> mark required for this same {@link
   * LoginBean} context - the same source of truth {@link #challenge} uses to decide the client-side
   * required marking, so a field is never marked required in the form while being optional to
   * match, or vice versa. An attribute with no User Profile entry at all stays mandatory (the
   * conservative default - see the fallback field in {@code login.ftl}). When disabled (default),
   * returns an empty set, so every configured attribute stays mandatory, unchanged from before this
   * setting existed.
   *
   * <p>Consumed by {@link #effectiveMatchAttributes} - {@link MultiAttributeCredentialResolver}
   * itself has no notion of "optional", it's purely a form-level concern handled before calling it.
   */
  protected Set<String> optionalAttributes(
      AuthenticationFlowContext context, List<String> matchAttributes) {
    if (!Utils.getBoolean(
        context.getAuthenticatorConfig(),
        Utils.HONOR_USER_PROFILE_REQUIRED,
        Boolean.parseBoolean(Utils.HONOR_USER_PROFILE_REQUIRED_DEFAULT))) {
      return Set.of();
    }
    LoginBean profile = new LoginBean(context.getSession(), matchAttributes);
    Map<String, LoginBean.Attribute> attributesByName = profile.getAttributesByName();
    Set<String> optional = new HashSet<>();
    for (String attribute : matchAttributes) {
      LoginBean.Attribute declared = attributesByName.get(attribute);
      if (declared != null && !declared.isRequired()) {
        optional.add(attribute);
      }
    }
    return optional;
  }

  /**
   * Drops any {@code optionalAttributes} entry left blank in {@code submittedValues} from the list
   * passed to {@link MultiAttributeCredentialResolver#resolveAuthenticatedUser} - the resolver then
   * never sees that attribute for this request, the same as if it weren't configured at all, rather
   * than the resolver needing its own "optional" concept. A mandatory attribute left blank is
   * deliberately kept in the list even though it'll fail: that's what makes the resolver's existing
   * blank-attribute check reject it (with its usual dummy-hash timing safety), exactly as it
   * already does today.
   *
   * <p>Falls back to the original, unfiltered {@code matchAttributes} if every entry would be
   * dropped (every configured attribute is optional and blank): passing an empty list to the
   * resolver would misreport a normal, if unusual, submission as a static misconfiguration (see
   * {@link MultiAttributeCredentialResolver#resolveAuthenticatedUser}'s empty-list check), and
   * would let the request through as an unconstrained "match everyone" query. Passing the original
   * list instead lets the resolver's own blank-attribute check reject it the same way as any other
   * invalid submission.
   */
  protected List<String> effectiveMatchAttributes(
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      Set<String> optionalAttributes) {
    List<String> kept =
        matchAttributes.stream()
            .filter(
                attribute ->
                    !optionalAttributes.contains(attribute)
                        || hasValue(submittedValues.get(attribute)))
            .toList();
    return kept.isEmpty() ? matchAttributes : kept;
  }

  private static boolean hasValue(String value) {
    return value != null && !value.isBlank();
  }

  /**
   * Reads each configured attribute's submitted value, normalizing date-typed attributes (per the
   * realm's User Profile {@code html5-date} annotation - see {@link Utils#resolveHtml5InputType})
   * into the canonical {@code YYYY-MM-DD} storage format - see {@link Utils#normalizeDate}. An
   * HTML5 date input already submits exactly {@code YYYY-MM-DD}, so this is a no-op for the common
   * case; it's still applied defensively so the browser path stays consistent with the IVR path,
   * which needs real reordering.
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
      MultivaluedMap<String, String> safeFormData = new MultivaluedHashMap<>(formData);
      safeFormData.remove(FIELD_PASSWORD);
      form.setFormData(safeFormData);
    }
    if (error != null) {
      form.setError(error);
    }

    List<String> matchAttributes =
        Utils.getMultivalueString(
            context.getAuthenticatorConfig(),
            Utils.MATCH_ATTRIBUTES,
            Utils.MATCH_ATTRIBUTES_DEFAULT);
    form.setAttribute("matchAttributes", matchAttributes);
    form.setAttribute("profile", new LoginBean(context.getSession(), matchAttributes));
    if (Utils.getBoolean(
        context.getAuthenticatorConfig(),
        Utils.HONOR_USER_PROFILE_REQUIRED,
        Boolean.parseBoolean(Utils.HONOR_USER_PROFILE_REQUIRED_DEFAULT))) {
      form.setAttribute("honorUserProfileRequired", true);
    }

    return form.createForm(FORM_FTL);
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

    ProviderConfigProperty existingUserSessionPolicy =
        new ProviderConfigProperty(
            EXISTING_USER_SESSION_POLICY,
            "Existing browser session policy",
            "KEEP preserves any existing browser session. TERMINATE_BEFORE_LOGIN removes the"
                + " existing session before showing this login form, allowing a different user"
                + " to authenticate on a shared device.",
            ProviderConfigProperty.LIST_TYPE,
            ExistingUserSessionPolicy.KEEP.name());
    existingUserSessionPolicy.setOptions(
        List.of(
            ExistingUserSessionPolicy.KEEP.name(),
            ExistingUserSessionPolicy.TERMINATE_BEFORE_LOGIN.name()));

    return List.of(
        EncryptedAttributeCredential.policyProperty(),
        EncryptedAttributeCredential.attributeProperty(),
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
        matchPolicy,
        existingUserSessionPolicy,
        new ProviderConfigProperty(
            Utils.HONOR_USER_PROFILE_REQUIRED,
            "Honor User Profile required attributes",
            "When enabled, each matchAttributes field's required-ness - both for matching and for"
                + " the rendered form - comes from the realm's User Profile required setting for"
                + " that attribute, instead of every configured attribute being unconditionally"
                + " mandatory. An attribute User Profile marks required gets the HTML5 required"
                + " attribute plus the asterisk + \"Required fields\" note register.ftl shows, and"
                + " must be filled in to match. An attribute NOT marked required becomes optional:"
                + " a voter may leave it blank and still match on the remaining attributes, as long"
                + " as at least one configured attribute has a value - an all-blank submission"
                + " still fails, same as today. WARNING: making an attribute optional widens who it"
                + " can match (e.g. dateOfBirth alone instead of dateOfBirth+nationalId), so only"
                + " enable this if that's the intended tradeoff for that attribute. Disabled by"
                + " default: every configured attribute stays unconditionally mandatory, exactly as"
                + " before this setting existed.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            Utils.HONOR_USER_PROFILE_REQUIRED_DEFAULT));
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }
}
