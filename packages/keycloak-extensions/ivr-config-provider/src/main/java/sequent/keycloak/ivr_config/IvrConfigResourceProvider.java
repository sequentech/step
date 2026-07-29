// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.ivr_config;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.WebApplicationException;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authorization.util.Tokens;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.representations.AccessToken;
import org.keycloak.services.resource.RealmResourceProvider;

/**
 * Serves {@code GET /realms/{realm}/ivr-config}. Walks the effective Direct Grant flow and takes
 * each {@code REQUIRED} / {@code CONDITIONAL} execution into an {@link AuthStep} the IVR Lambda can
 * collect via DTMF.
 *
 * <p>Authentication: requires a bearer token issued to a client carrying the {@code
 * ivr-config-read} realm role.
 *
 * <p>Failure semantics: an authenticator that is neither in {@link #STOCK_AUTHENTICATORS} nor
 * backed by an {@link AuthenticatorConfigModel} declaring the IVR metadata keys is a
 * deployment-time misconfiguration and yields HTTP 500.
 */
@JBossLog
public class IvrConfigResourceProvider implements RealmResourceProvider {
  /**
   * Voting client ID, it may have a specific flow override, in which case we prefer over realm's
   * default flow. This specific ID is also used internally by the platform to auth voters through
   * the IVR.
   */
  static final String IVR_VOTING_CLIENT_ID = "ivr-voting";

  /** Flow we override for the voting client, if exists. Only Direct Grant flow currently. */
  static final String IVR_VOTING_OVERRIDE_FLOW = "direct_grant";

  /** Realm role required on the caller's token. */
  static final String REQUIRED_ROLE = "ivr-config-read";

  /**
   * Lookup table for the stock Keycloak authenticators we currently support. Anything else either
   * resolves through {@link AuthenticatorConfigModel} (custom authenticator path) or triggers a 500
   * (unknown authenticator).
   */
  static final Map<String, AuthStep> STOCK_AUTHENTICATORS =
      Map.of(
          "direct-grant-validate-username",
              new AuthStep("voter_id", 8, Constants.AUTH_STEP_KIND_IDENTIFIER, "username", null),
          "direct-grant-validate-password",
              new AuthStep("pin", 8, Constants.AUTH_STEP_KIND_SECRET, "password", null));

  /** Authenticators that must not surface as an IVR-collected step. */
  static final Set<String> SKIPPED_AUTHENTICATORS = Set.of();

  private final KeycloakSession session;

  public IvrConfigResourceProvider(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @GET
  @Path("/")
  @Produces(MediaType.APPLICATION_JSON)
  public Response getIvrConfig() {
    checkAuthorization();

    RealmModel realm = session.getContext().getRealm();
    AuthenticationFlowModel flow = effectiveDirectGrantFlow(realm);

    List<AuthStep> steps = new ArrayList<>();
    realm
        .getAuthenticationExecutionsStream(flow.getId())
        .filter(e -> !e.isAuthenticatorFlow()) // skip sub-flow references
        .filter(IvrConfigResourceProvider::isRequiredOrConditional)
        .filter(e -> !SKIPPED_AUTHENTICATORS.contains(e.getAuthenticator()))
        .forEachOrdered(e -> steps.addAll(buildSteps(realm, e)));

    // If there are no auth steps "left" configured, it is very likely that was a configuration
    // issue.
    if (steps.isEmpty()) {
      throw new WebApplicationException(
          "There are no viable auth steps for IVR.", Response.Status.INTERNAL_SERVER_ERROR);
    }

    return Response.ok(Map.of(Constants.IVR_CONFIG_FIELD_STEPS, steps)).build();
  }

  // Primarily a test helper
  AccessToken extractToken() {
    return Tokens.getAccessToken(session);
  }

  /** Reject the request unless the bearer token carries the required realm role. */
  private void checkAuthorization() {
    AccessToken token = extractToken();
    if (token == null) {
      log.debug("ivr-config: no bearer token");
      throw new WebApplicationException(Response.Status.UNAUTHORIZED);
    }

    AccessToken.Access realmAccess = token.getRealmAccess();
    if (realmAccess == null || !realmAccess.isUserInRole(REQUIRED_ROLE)) {
      log.debug("ivr-config: token missing required realm role");
      throw new WebApplicationException(Response.Status.FORBIDDEN);
    }
  }

  /**
   * Pick the {@code ivr-voting} client's Direct Grant override if present, otherwise the realm
   * default.
   */
  private static AuthenticationFlowModel effectiveDirectGrantFlow(RealmModel realm) {
    ClientModel ivrClient = realm.getClientByClientId(IVR_VOTING_CLIENT_ID);
    if (ivrClient != null) {
      String overrideId = ivrClient.getAuthenticationFlowBindingOverride(IVR_VOTING_OVERRIDE_FLOW);
      if (overrideId != null) {
        AuthenticationFlowModel override = realm.getAuthenticationFlowById(overrideId);
        if (override != null) {
          return override;
        }
        log.warnf(
            "ivr-config: client '%s' declares direct_grant override '%s' but it does not resolve; "
                + "falling back to realm default",
            IVR_VOTING_CLIENT_ID, overrideId);
      }
    }
    return realm.getDirectGrantFlow();
  }

  private static boolean isRequiredOrConditional(AuthenticationExecutionModel exec) {
    AuthenticationExecutionModel.Requirement r = exec.getRequirement();
    return r == AuthenticationExecutionModel.Requirement.REQUIRED
        || r == AuthenticationExecutionModel.Requirement.CONDITIONAL;
  }

  /**
   * Values for {@code field}/{@code max_digits}/{@code kind}/{@code maps_to}/{@code prompt_key} may
   * be {@value #MULTIVALUE_SEPARATOR}-separated to fan one execution out into several ordered
   * {@link AuthStep}s (e.g. a single multi-attribute authenticator collecting both a date of birth
   * and a PIN). All multivalued properties present must split into the same count; {@code
   * prompt_key} is the only one allowed to be entirely absent.
   */
  private static final String MULTIVALUE_SEPARATOR = "##";

  private static List<AuthStep> buildSteps(RealmModel realm, AuthenticationExecutionModel exec) {
    String authenticatorId = exec.getAuthenticator();
    AuthStep stock = STOCK_AUTHENTICATORS.get(authenticatorId);
    if (stock != null) {
      return List.of(stock);
    }

    String configId = exec.getAuthenticatorConfig();
    AuthenticatorConfigModel cfg =
        configId == null ? null : realm.getAuthenticatorConfigById(configId);
    if (cfg == null) {
      String msg =
          "Unknown IVR authenticator '%s' has no AuthenticatorConfig, cannot derive IVR auth step";
      throw new WebApplicationException(
          msg.formatted(authenticatorId), Response.Status.INTERNAL_SERVER_ERROR);
    }

    Map<String, String> c = cfg.getConfig();
    if (c == null) {
      String msg = "Custom authenticator '%s' config is empty, can't derive IVR auth step";
      throw new WebApplicationException(
          msg.formatted(authenticatorId), Response.Status.INTERNAL_SERVER_ERROR);
    }
    String fieldRaw = c.get(Constants.AUTH_STEP_PROP_FIELD);
    String mapsToRaw = c.get(Constants.AUTH_STEP_PROP_MAPS_TO);
    String kindRaw = c.get(Constants.AUTH_STEP_PROP_KIND);
    if (fieldRaw == null || mapsToRaw == null || kindRaw == null) {
      String msg = "AuthenticatorConfig for '%s' is missing required IVR keys (%s, %s, %s)";
      throw new WebApplicationException(
          msg.formatted(
              authenticatorId,
              Constants.AUTH_STEP_PROP_FIELD,
              Constants.AUTH_STEP_PROP_MAPS_TO,
              Constants.AUTH_STEP_PROP_KIND),
          Response.Status.INTERNAL_SERVER_ERROR);
    }

    List<String> fields = splitMultivalue(fieldRaw);
    List<String> mapsTos = splitMultivalue(mapsToRaw);
    List<String> kinds = splitMultivalue(kindRaw);
    List<String> maxDigitsRaw =
        splitMultivalue(c.getOrDefault(Constants.AUTH_STEP_PROP_MAX_DIGITS, "10"));
    String promptKeyRaw = c.get(Constants.AUTH_STEP_PROP_PROMPT_KEY); // optional, may be null
    List<String> promptKeys = promptKeyRaw == null ? null : splitMultivalue(promptKeyRaw);

    int count = fields.size();
    if (mapsTos.size() != count
        || kinds.size() != count
        || maxDigitsRaw.size() != count
        || (promptKeys != null && promptKeys.size() != count)) {
      String msg =
          "AuthenticatorConfig for '%s' has mismatched %s-separated value counts across field/"
              + "max_digits/kind/maps_to/prompt_key";
      throw new WebApplicationException(
          msg.formatted(authenticatorId, MULTIVALUE_SEPARATOR),
          Response.Status.INTERNAL_SERVER_ERROR);
    }

    List<AuthStep> steps = new ArrayList<>();
    for (int i = 0; i < count; i++) {
      int maxDigits;
      try {
        maxDigits = Integer.parseInt(maxDigitsRaw.get(i));
      } catch (NumberFormatException e) {
        String msg = "AuthenticatorConfig for '%s' has non-numeric max_digits";
        throw new WebApplicationException(
            msg.formatted(authenticatorId), Response.Status.INTERNAL_SERVER_ERROR);
      }
      String promptKey = promptKeys == null ? null : promptKeys.get(i);
      steps.add(new AuthStep(fields.get(i), maxDigits, kinds.get(i), mapsTos.get(i), promptKey));
    }
    return steps;
  }

  private static List<String> splitMultivalue(String raw) {
    return Arrays.asList(raw.split(MULTIVALUE_SEPARATOR));
  }

  @Override
  public void close() {}
}
