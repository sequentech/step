// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.directgrant.AbstractDirectGrantAuthenticator;
import org.keycloak.events.Errors;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * IVR-facing Direct Grant analog of {@link MultiAttributePasswordAuthenticator}: resolves a voter
 * from one or more configured attributes (e.g. date of birth) plus a PIN submitted together in a
 * single {@code grant_type=password} request, without a username. Shares its resolution rules with
 * the browser authenticator via {@link MultiAttributeCredentialResolver}.
 *
 * <p>Unlike the browser authenticator, there is no {@code matchAttributes} config property here.
 * The {@code field}/{@code max_digits}/{@code kind}/{@code maps_to}/{@code prompt_key} properties
 * this authenticator declares are read generically by the {@code ivr-config-provider} module's
 * {@code IvrConfigResourceProvider} to describe each DTMF collection step to the IVR Lambda - see
 * that module for the full contract. Rather than duplicate that same field list in a second,
 * parallel config property (risking the two drifting out of sync), this authenticator derives its
 * own resolution inputs directly from {@code maps_to}/{@code kind}: every entry whose {@code kind}
 * is {@code identifier} is a Keycloak user attribute to match (also the Direct Grant POST parameter
 * name its value arrives under); exactly one entry must be {@code secret} - the password/PIN
 * parameter name.
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class MultiAttributePasswordDirectGrantAuthenticator
    extends AbstractDirectGrantAuthenticator {
  public static final String PROVIDER_ID = "multi-attribute-password-direct-grant";

  /**
   * Config keys read by {@code ivr-config-provider}'s {@code Constants} - kept in sync manually
   * since the two modules intentionally don't share a compile-time dependency.
   */
  public static final String CONFIG_FIELD = "field";

  public static final String CONFIG_MAX_DIGITS = "max_digits";
  public static final String CONFIG_KIND = "kind";
  public static final String CONFIG_MAPS_TO = "maps_to";
  public static final String CONFIG_PROMPT_KEY = "prompt_key";

  /**
   * Authenticator-internal only - not part of the {@code ivr-config-provider} contract, so it's not
   * subject to that module's strict field-count validation. Optional, {@code ##}-separated, aligned
   * by index with the {@code identifier}-kind fields in the order they appear (i.e. with {@code
   * matchAttributes}, not the full field list) - e.g. {@code "MMDDYYYY"} for the raw digit order
   * IVR DTMF collection uses for a date. Missing/blank entries mean "no normalization" for that
   * field.
   */
  public static final String CONFIG_DATE_FORMAT = "date_format";

  public static final String KIND_IDENTIFIER = "identifier";
  public static final String KIND_SECRET = "secret";

  public static final Requirement[] REQUIREMENT_CHOICES = {Requirement.REQUIRED};

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    List<String> mapsTos = Utils.getMultivalueString(authConfig, CONFIG_MAPS_TO, List.of());
    List<String> kinds = Utils.getMultivalueString(authConfig, CONFIG_KIND, List.of());

    if (mapsTos.isEmpty() || mapsTos.size() != kinds.size()) {
      // Logged at ERROR: a static misconfiguration, not a one-off bad request - every Direct
      // Grant attempt through this authenticator config fails until an admin fixes it, so it
      // needs to stand out clearly in production monitoring. IVR callers have no client-side way
      // to report the actual cause, so the config alias here is the only lead an operator has.
      log.errorv(
          "authenticate(): misconfigured - maps_to/kind missing or count mismatch, realm={0},"
              + " authenticatorConfig={1}",
          context.getRealm().getName(), configAlias(authConfig));
      fail(context);
      return;
    }

    List<String> matchAttributes = new ArrayList<>();
    String passwordField = null;
    for (int i = 0; i < mapsTos.size(); i++) {
      if (KIND_SECRET.equalsIgnoreCase(kinds.get(i))) {
        passwordField = mapsTos.get(i);
      } else {
        matchAttributes.add(mapsTos.get(i));
      }
    }
    if (passwordField == null || matchAttributes.isEmpty()) {
      log.errorv(
          "authenticate(): misconfigured - config must declare exactly one 'secret' kind entry"
              + " and at least one 'identifier' entry, realm={0}, authenticatorConfig={1}",
          context.getRealm().getName(), configAlias(authConfig));
      fail(context);
      return;
    }

    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    Map<String, String> submittedValues =
        collectSubmittedValues(authConfig, matchAttributes, formData);
    String password = formData.getFirst(passwordField);

    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        Utils.getThrottleConfig(authConfig);
    MultiAttributeCredentialResolver.MatchPolicy matchPolicy = Utils.getMatchPolicy(authConfig);
    MultiAttributeCredentialResolver.Resolution result =
        MultiAttributeCredentialResolver.resolveAuthenticatedUser(
            context.getSession(),
            context.getRealm(),
            matchAttributes,
            submittedValues,
            password,
            throttleConfig,
            matchPolicy);

    // Set even on failure/lockout, before signaling the outcome - Keycloak's brute-force
    // accounting (DefaultAuthenticationFlow -> AuthenticationProcessor.logFailure()) only fires
    // for a user set on the authentication session, same as the standard ValidateUsername.
    result.attributableUser().ifPresent(context::setUser);

    if (result.authenticatedUser().isPresent()) {
      context.success();
    } else if (result.lockoutState() != MultiAttributeCredentialResolver.LockoutState.NONE) {
      lockedOut(context, result.lockoutState());
    } else {
      fail(context);
    }
  }

  /**
   * Normalizes date-typed identifier values from the raw digit order the IVR Lambda collects (per
   * the optional {@link #CONFIG_DATE_FORMAT} config, aligned by index with {@code matchAttributes})
   * into the canonical {@code YYYY-MM-DD} storage format - see {@link Utils#normalizeDate}. Without
   * this, IVR-collected digits in any order other than YYYY-MM-DD never match a stored value.
   */
  protected Map<String, String> collectSubmittedValues(
      AuthenticatorConfigModel authConfig,
      List<String> matchAttributes,
      MultivaluedMap<String, String> formData) {
    List<String> dateFormats = Utils.getMultivalueString(authConfig, CONFIG_DATE_FORMAT, List.of());
    Map<String, String> submittedValues = new HashMap<>();
    for (int i = 0; i < matchAttributes.size(); i++) {
      String attribute = matchAttributes.get(i);
      String rawValue = formData.getFirst(attribute);
      String format = i < dateFormats.size() ? dateFormats.get(i) : "";
      if (format != null && !format.isBlank()) {
        rawValue = Utils.normalizeDate(rawValue, format);
      }
      submittedValues.put(attribute, rawValue);
    }
    return submittedValues;
  }

  /**
   * The admin-visible config alias (e.g. as shown next to the flow step in the Admin Console), or a
   * placeholder when no config is attached - the only way a log line can point an operator at which
   * specific authenticator config to fix, since a realm can have more than one.
   */
  private static String configAlias(AuthenticatorConfigModel authConfig) {
    return authConfig == null || authConfig.getAlias() == null ? "(none)" : authConfig.getAlias();
  }

  private void fail(AuthenticationFlowContext context) {
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);
    Response challengeResponse =
        errorResponse(
            Response.Status.BAD_REQUEST.getStatusCode(),
            "invalid_grant",
            "Invalid user credentials");
    context.failure(AuthenticationFlowError.INVALID_USER, challengeResponse);
  }

  /**
   * Mirrors the stock {@code ValidateUsername} Direct Grant authenticator's brute-force handling:
   * uses {@code forceChallenge} rather than {@code failure} so the flow engine doesn't log yet
   * another failure against an account that's already locked out.
   */
  private void lockedOut(
      AuthenticationFlowContext context, MultiAttributeCredentialResolver.LockoutState state) {
    boolean permanent = state == MultiAttributeCredentialResolver.LockoutState.PERMANENT;
    context.getEvent().error(permanent ? Errors.USER_DISABLED : Errors.USER_TEMPORARILY_DISABLED);
    Response challengeResponse =
        errorResponse(
            Response.Status.BAD_REQUEST.getStatusCode(),
            "invalid_grant",
            permanent ? "Account permanently disabled" : "Account temporarily disabled");
    context.forceChallenge(challengeResponse);
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
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getDisplayType() {
    return "Multi-Attribute + Password (Direct Grant)";
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
  public String getHelpText() {
    return "Authenticates a caller (e.g. via IVR) by matching one or more configured user"
        + " attributes (all must match the same user) plus a password/PIN submitted together in"
        + " a single Direct Grant request. Does not require a username.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    ProviderConfigProperty matchPolicy =
        new ProviderConfigProperty(
            Utils.MATCH_POLICY,
            "Multiple-candidate match policy",
            "How to resolve when more than one candidate shares the identifying attribute"
                + " value(s). REJECT_AMBIGUOUS (default, safe): only succeed if the submitted PIN"
                + " matches exactly one candidate; if it matches more than one, fail generically."
                + " FIRST_MATCH: succeed as soon as any candidate's PIN matches, without checking"
                + " the rest. WARNING: FIRST_MATCH is only secure if PINs are guaranteed unique"
                + " across every candidate this could ever match - if two candidates share a PIN,"
                + " which one authenticates is unspecified, letting one voter authenticate as"
                + " another's account. Do not enable it unless PIN uniqueness across candidates is"
                + " guaranteed.",
            ProviderConfigProperty.LIST_TYPE,
            Utils.MATCH_POLICY_DEFAULT);
    matchPolicy.setOptions(
        List.of(
            MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS.name(),
            MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH.name()));

    return List.of(
        new ProviderConfigProperty(
            CONFIG_FIELD,
            "IVR field labels",
            "One label per collected field, in order (e.g. dob, pin). Shown to the IVR Lambda"
                + " only, not used for resolution.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_MAX_DIGITS,
            "Max digits per field",
            "Maximum DTMF digits per field, same order as the other field lists (e.g. 8).",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_KIND,
            "Kind per field",
            "\"identifier\" or \"secret\" per field, same order. Exactly one field must be"
                + " \"secret\" (the password/PIN); the rest identify the voter and must jointly"
                + " match a single account.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_MAPS_TO,
            "Direct Grant parameter per field",
            "The grant_type=password POST parameter name each field's value arrives under, same"
                + " order. For \"identifier\" fields this is also the Keycloak user attribute"
                + " matched against; the \"secret\" field's value should normally be \"password\".",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_PROMPT_KEY,
            "IVR prompt key per field (optional)",
            "Overrides the IVR Lambda's default prompt key per field, same order. Leave empty to"
                + " use the Lambda's well-known defaults.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_DATE_FORMAT,
            "Date format per identifier field (optional)",
            "For date-valued identifier fields only (e.g. a date of birth collected as raw DTMF"
                + " digits): the digit order the IVR Lambda collects it in, e.g. \"MMDDYYYY\"."
                + " Aligned by index with the identifier fields only (not the secret field) -"
                + " leave an entry empty for identifier fields that aren't dates. Normalizes into"
                + " the canonical YYYY-MM-DD storage format before matching.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            Utils.MAX_CANDIDATES,
            "Max candidates per request",
            "DoS guard: once the identifier fields match more than this many enabled,"
                + " non-locked-out candidates, the request fails generically without checking any"
                + " of their PINs. Bounds the worst-case number of password hashes per request."
                + " Default: "
                + Utils.MAX_CANDIDATES_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.MAX_CANDIDATES_DEFAULT),
        new ProviderConfigProperty(
            Utils.TUPLE_MAX_FAILURES,
            "Max failures per identifier-value combination",
            "DoS guard: failures allowed for a single submitted combination of identifier values"
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
                + " same identifier-value combination resets the window. Default: "
                + Utils.TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT),
        new ProviderConfigProperty(
            Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS,
            "Max user-store rows per identifier lookup",
            "DoS guard: hard ceiling on rows the user store may return for the combined query"
                + " across every identifier field - bounds worst-case database/memory cost,"
                + " separately from the (much tighter) max candidates guard above, which bounds"
                + " PIN-hash cost. Since all identifier fields are ANDed together in one query, the"
                + " rows returned are always the true multi-field match, so this ceiling can only"
                + " ever discard results in a genuinely pathological case (more true combined"
                + " matches than the ceiling) - keep it well above the largest realistic combined"
                + " match count you'd expect a legitimate voter lookup to produce. Default: "
                + Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT
                + ".",
            ProviderConfigProperty.STRING_TYPE,
            Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT),
        matchPolicy);
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
