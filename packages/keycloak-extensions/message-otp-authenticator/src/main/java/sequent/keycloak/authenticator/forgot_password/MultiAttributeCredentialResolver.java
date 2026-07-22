// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.PasswordPolicy;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SingleUseObjectProvider;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.services.managers.BruteForceProtector;

/**
 * Resolves a user from one or more configured attributes plus a password, without a username.
 * Shared by every transport that needs this: the browser form ({@link
 * MultiAttributePasswordAuthenticator}) and the IVR Direct Grant flow
 * (MultiAttributePasswordDirectGrantAuthenticator) - same rules, same failure semantics, regardless
 * of how the values arrived.
 *
 * <p>Resolution: for each configured attribute, find every user whose attribute equals the
 * submitted value, then intersect those candidate sets across all attributes. Candidates who are
 * disabled or currently locked out by brute-force protection are excluded before any password is
 * checked. If exactly one candidate remains, its password is checked directly - this is the only
 * case where a failure can be attributed to one account (see {@link
 * Resolution#attributableUser()}), so callers should {@code setUser()} it before signaling failure,
 * letting Keycloak's normal brute-force accounting engage the same way it does for the standard
 * username/password form. If more than one candidate remains, the password disambiguates among
 * them, but a failure there can't be attributed to a single account. Any early-return path performs
 * an equivalent-cost dummy password hash first, so "no viable candidate" doesn't respond measurably
 * faster than "wrong password" - see {@link #performDummyHash}.
 *
 * <p>Two DoS mitigations bound the CPU cost an attacker can force per request, since - unlike a
 * standard username/password form - a common attribute value (e.g. a shared date of birth) can
 * legitimately resolve to many candidates, and Keycloak's own brute-force accounting can't engage
 * until resolution narrows to one account:
 *
 * <ul>
 *   <li>{@code maxCandidates} ({@link ThrottleConfig#maxCandidates()}): once attribute matching
 *       narrows to more enabled candidates than this, resolution fails generically without checking
 *       any of their passwords or their brute-force lockout state - bounding both the worst-case
 *       number of password hashes and the worst-case number of {@code BruteForceProtector} lookups
 *       per request, regardless of how many voters happen to share the submitted attribute
 *       value(s).
 *   <li>Per-tuple failure throttle ({@link ThrottleConfig#tupleMaxFailures()} / {@link
 *       ThrottleConfig#tupleFailureWindowSeconds()}): failures are counted per distinct combination
 *       of submitted attribute values (a "tuple"), independent of any single account, using {@link
 *       SingleUseObjectProvider} (replicated across Keycloak nodes) so the same tuple can't be
 *       hammered past the limit across a cluster. Once a tuple's failures reach the configured
 *       maximum within the window, further attempts against it short-circuit before any user lookup
 *       - closing the accounting gap that lets an attacker repeat a common,
 *       never-uniquely-attributable attribute value indefinitely at full amplification cost.
 * </ul>
 */
@JBossLog
public final class MultiAttributeCredentialResolver {

  private MultiAttributeCredentialResolver() {}

  private static final String TUPLE_THROTTLE_KEY_PREFIX =
      "multi-attribute-password:tuple-throttle:";
  private static final String FAILURE_COUNT_NOTE = "failures";

  /** Brute-force lockout state of a resolved candidate. */
  public enum LockoutState {
    NONE,
    TEMPORARY,
    PERMANENT
  }

  /**
   * DoS-mitigation configuration, read from each caller's own {@code AuthenticatorConfigModel} via
   * {@link Utils#getThrottleConfig}, so the browser form and the IVR Direct Grant flow can be tuned
   * independently even though they share this resolution logic.
   *
   * @param maxCandidates per-request cap on enabled candidates, checked before any brute-force
   *     lockout lookup or password disambiguation (see the class-level DoS-mitigation note).
   * @param tupleMaxFailures failures allowed for a single submitted-attribute-value combination
   *     within {@code tupleFailureWindowSeconds} before further attempts against it short-circuit.
   * @param tupleFailureWindowSeconds rolling window, in seconds, that {@code tupleMaxFailures}
   *     applies over; each new failure resets the window for that tuple.
   */
  public record ThrottleConfig(
      int maxCandidates, int tupleMaxFailures, int tupleFailureWindowSeconds) {

    public static ThrottleConfig defaults() {
      return new ThrottleConfig(
          Integer.parseInt(Utils.MAX_CANDIDATES_DEFAULT),
          Integer.parseInt(Utils.TUPLE_MAX_FAILURES_DEFAULT),
          Integer.parseInt(Utils.TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT));
    }
  }

  /**
   * Outcome of a resolution attempt.
   *
   * @param authenticatedUser populated only on full success (matching attributes AND password).
   * @param attributableUser populated whenever resolution narrowed to exactly one viable candidate,
   *     regardless of whether their password matched - callers should {@code context.setUser()}
   *     this before signaling failure so Keycloak's brute-force accounting can attribute the
   *     attempt.
   * @param lockoutState non-{@code NONE} when the sole viable-by-attributes candidate is currently
   *     locked out by brute-force protection, so no password check was even attempted.
   */
  public record Resolution(
      Optional<UserModel> authenticatedUser,
      Optional<UserModel> attributableUser,
      LockoutState lockoutState) {

    static Resolution success(UserModel user) {
      return new Resolution(Optional.of(user), Optional.of(user), LockoutState.NONE);
    }

    static Resolution failure() {
      return new Resolution(Optional.empty(), Optional.empty(), LockoutState.NONE);
    }

    static Resolution failureAttributedTo(UserModel user) {
      return new Resolution(Optional.empty(), Optional.of(user), LockoutState.NONE);
    }

    static Resolution lockedOut(UserModel user, LockoutState state) {
      return new Resolution(Optional.empty(), Optional.of(user), state);
    }
  }

  /** Convenience overload for callers that don't need a non-default {@link ThrottleConfig}. */
  public static Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password) {
    return resolveAuthenticatedUser(
        session, realm, matchAttributes, submittedValues, password, ThrottleConfig.defaults());
  }

  public static Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password,
      ThrottleConfig throttleConfig) {
    if (matchAttributes == null || matchAttributes.isEmpty()) {
      log.warn("resolveAuthenticatedUser(): no matchAttributes configured");
      return dummyFailure(session, realm);
    }
    if (password == null || password.isBlank()) {
      return dummyFailure(session, realm);
    }

    Map<String, String> trimmedValues = new LinkedHashMap<>();
    for (String attribute : matchAttributes) {
      String value = submittedValues.get(attribute);
      if (value == null || value.isBlank()) {
        return dummyFailure(session, realm);
      }
      trimmedValues.put(attribute, value.trim());
    }

    // Computed once every attribute has a validated value, and checked before any user lookup -
    // see the class-level DoS-mitigation note. Values that never reach this point (missing/blank)
    // don't correspond to a specific tuple worth throttling: the loop above already bails out at
    // the same cost as any other invalid-input request, without searching for or hashing anything.
    String tupleKey = tupleThrottleKey(realm, matchAttributes, trimmedValues);
    if (isTupleThrottled(session, tupleKey, throttleConfig.tupleMaxFailures())) {
      log.warnv("resolveAuthenticatedUser(): tuple throttled, key={0}", tupleKey);
      return dummyFailure(session, realm);
    }

    Map<String, UserModel> candidatesById = null;
    for (String attribute : matchAttributes) {
      String value = trimmedValues.get(attribute);

      Map<String, UserModel> matchesForAttribute =
          findUsersByAttribute(session, realm, attribute, value)
              .collect(Collectors.toMap(UserModel::getId, Function.identity(), (a, b) -> a));

      if (candidatesById == null) {
        candidatesById = matchesForAttribute;
      } else {
        candidatesById.keySet().retainAll(matchesForAttribute.keySet());
      }

      if (candidatesById.isEmpty()) {
        recordTupleFailure(session, tupleKey, throttleConfig.tupleFailureWindowSeconds());
        return dummyFailure(session, realm);
      }
    }

    List<UserModel> enabledCandidates =
        candidatesById.values().stream().filter(UserModel::isEnabled).collect(Collectors.toList());

    if (enabledCandidates.size() > throttleConfig.maxCandidates()) {
      // Cap checked here, before lockoutStateOf() below touches BruteForceProtector for each
      // candidate - bounds not just the worst-case number of password hashes per request (see the
      // class-level DoS-mitigation note) but also the K brute-force-lockout lookups that would
      // otherwise run first. Never log the candidate values themselves, only the count.
      log.warnv(
          "resolveAuthenticatedUser(): candidate cap exceeded, {0} enabled candidates (max {1})",
          enabledCandidates.size(), throttleConfig.maxCandidates());
      recordTupleFailure(session, tupleKey, throttleConfig.tupleFailureWindowSeconds());
      return dummyFailure(session, realm);
    }

    Map<UserModel, LockoutState> lockoutStates =
        enabledCandidates.stream()
            .collect(Collectors.toMap(Function.identity(), c -> lockoutStateOf(session, realm, c)));
    List<UserModel> lockedOutCandidates =
        enabledCandidates.stream()
            .filter(candidate -> lockoutStates.get(candidate) != LockoutState.NONE)
            .collect(Collectors.toList());
    // Bounded by the cap above (<= maxCandidates), since it's a subset of enabledCandidates.
    List<UserModel> viableCandidates =
        enabledCandidates.stream()
            .filter(candidate -> lockoutStates.get(candidate) == LockoutState.NONE)
            .collect(Collectors.toList());

    if (viableCandidates.isEmpty()) {
      // Every enabled candidate for these attributes is currently locked out: only report the
      // specific lockout when there is exactly one such account to attribute it to - an ambiguous
      // set of locked accounts must stay as generic a failure as any other ambiguous outcome.
      recordTupleFailure(session, tupleKey, throttleConfig.tupleFailureWindowSeconds());
      if (lockedOutCandidates.size() == 1) {
        UserModel locked = lockedOutCandidates.get(0);
        return Resolution.lockedOut(locked, lockoutStates.get(locked));
      }
      return dummyFailure(session, realm);
    }

    if (viableCandidates.size() == 1) {
      UserModel candidate = viableCandidates.get(0);
      if (isPasswordValid(candidate, password)) {
        clearTupleThrottle(session, tupleKey);
        return Resolution.success(candidate);
      }
      recordTupleFailure(session, tupleKey, throttleConfig.tupleFailureWindowSeconds());
      return Resolution.failureAttributedTo(candidate);
    }

    List<UserModel> passwordMatches =
        viableCandidates.stream()
            .filter(candidate -> isPasswordValid(candidate, password))
            .collect(Collectors.toList());

    if (passwordMatches.size() == 1) {
      clearTupleThrottle(session, tupleKey);
      return Resolution.success(passwordMatches.get(0));
    }
    if (passwordMatches.size() > 1) {
      log.warnv(
          "resolveAuthenticatedUser(): ambiguous match, {0} candidates matched the submitted"
              + " password",
          passwordMatches.size());
    }
    recordTupleFailure(session, tupleKey, throttleConfig.tupleFailureWindowSeconds());
    return Resolution.failure();
  }

  private static Resolution dummyFailure(KeycloakSession session, RealmModel realm) {
    performDummyHash(session, realm);
    return Resolution.failure();
  }

  /**
   * Builds the per-tuple throttle key: {@code SHA-256(realmId + '|' + attr1=value1 + '|' +
   * attr2=value2 ...)}, sorted by attribute name so the key is independent of {@code
   * matchAttributes}' configured order.
   */
  private static String tupleThrottleKey(
      RealmModel realm, List<String> matchAttributes, Map<String, String> trimmedValues) {
    List<String> sortedAttributes = new ArrayList<>(matchAttributes);
    Collections.sort(sortedAttributes);
    StringBuilder raw = new StringBuilder(String.valueOf(realm.getId()));
    for (String attribute : sortedAttributes) {
      raw.append('|').append(attribute).append('=').append(trimmedValues.get(attribute));
    }
    return sha256Hex(raw.toString());
  }

  private static String sha256Hex(String input) {
    try {
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      byte[] hash = digest.digest(input.getBytes(StandardCharsets.UTF_8));
      StringBuilder hex = new StringBuilder(hash.length * 2);
      for (byte b : hash) {
        hex.append(String.format("%02x", b));
      }
      return hex.toString();
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException("SHA-256 not available", e);
    }
  }

  private static String tupleThrottleStoreKey(String tupleKey) {
    return TUPLE_THROTTLE_KEY_PREFIX + tupleKey;
  }

  private static int failureCount(Map<String, String> notes) {
    if (notes == null) {
      return 0;
    }
    String count = notes.get(FAILURE_COUNT_NOTE);
    return count == null ? 0 : Integer.parseInt(count);
  }

  private static boolean isTupleThrottled(
      KeycloakSession session, String tupleKey, int tupleMaxFailures) {
    Map<String, String> notes = session.singleUseObjects().get(tupleThrottleStoreKey(tupleKey));
    return failureCount(notes) >= tupleMaxFailures;
  }

  /**
   * Records a failure against this tuple, resetting its TTL to a fresh {@code windowSeconds} window
   * - a rolling window, not a fixed one, so sustained hammering of the same tuple keeps it
   * throttled rather than getting a clean slate mid-attack.
   *
   * <p>Read-then-write, not atomic: {@link SingleUseObjectProvider} exposes no compare-and-swap, so
   * concurrent requests against the same tuple can race both ways - two failures can collapse into
   * one increment (undercount, delaying engagement), or a failure can read stale data and overwrite
   * a concurrent {@link #clearTupleThrottle} (spuriously reviving a counter a success just
   * cleared). Acceptable here since this throttle only bounds CPU cost rather than enforcing a hard
   * security limit: every write resets the TTL to at most {@code windowSeconds}, so either race
   * self-clears within that window - never a stuck lockout, for attacker or legitimate voter alike.
   */
  private static void recordTupleFailure(
      KeycloakSession session, String tupleKey, int windowSeconds) {
    String storeKey = tupleThrottleStoreKey(tupleKey);
    SingleUseObjectProvider store = session.singleUseObjects();
    int nextCount = failureCount(store.get(storeKey)) + 1;
    store.put(storeKey, windowSeconds, Map.of(FAILURE_COUNT_NOTE, String.valueOf(nextCount)));
  }

  private static void clearTupleThrottle(KeycloakSession session, String tupleKey) {
    session.singleUseObjects().remove(tupleThrottleStoreKey(tupleKey));
  }

  /**
   * Performs a password-hash computation of realistic cost against fixed dummy data, matching
   * {@code org.keycloak.authentication.authenticators.util.AuthenticatorUtils#dummyHash} (used by
   * Keycloak's own {@code ValidateUsername} when no user is found), so that "no viable candidate"
   * doesn't resolve measurably faster than "found a candidate, wrong password" - both perform
   * exactly one hash comparison.
   */
  private static void performDummyHash(KeycloakSession session, RealmModel realm) {
    PasswordPolicy passwordPolicy = realm.getPasswordPolicy();
    PasswordHashProvider provider;
    if (passwordPolicy != null && passwordPolicy.getHashAlgorithm() != null) {
      provider = session.getProvider(PasswordHashProvider.class, passwordPolicy.getHashAlgorithm());
    } else {
      provider = session.getProvider(PasswordHashProvider.class);
    }
    int iterations = passwordPolicy != null ? passwordPolicy.getHashIterations() : -1;
    provider.encodedCredential("SlightlyLongerDummyPassword", iterations);
  }

  private static LockoutState lockoutStateOf(
      KeycloakSession session, RealmModel realm, UserModel user) {
    if (!realm.isBruteForceProtected()) {
      return LockoutState.NONE;
    }
    BruteForceProtector protector = session.getProvider(BruteForceProtector.class);
    if (protector.isPermanentlyLockedOut(session, realm, user)) {
      return LockoutState.PERMANENT;
    }
    if (protector.isTemporarilyDisabled(session, realm, user)) {
      return LockoutState.TEMPORARY;
    }
    return LockoutState.NONE;
  }

  /**
   * Resolves candidates for one configured attribute. {@code username} is always unique in
   * Keycloak, so a single lookup is safe there - but {@code email} is only unique when the realm
   * has {@code duplicateEmailsAllowed} disabled, so it uses the exact-match search API (rather than
   * {@code getUserByEmail}, which returns only one arbitrary match) to keep every candidate in play
   * for the password-disambiguation step in {@link #resolveAuthenticatedUser}.
   */
  public static Stream<UserModel> findUsersByAttribute(
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

  public static boolean isPasswordValid(UserModel user, String password) {
    return user.credentialManager().isValid(UserCredentialModel.password(password));
  }
}
