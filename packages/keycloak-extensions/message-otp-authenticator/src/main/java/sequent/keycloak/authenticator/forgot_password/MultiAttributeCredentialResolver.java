// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

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
 */
@JBossLog
public final class MultiAttributeCredentialResolver {

  private MultiAttributeCredentialResolver() {}

  /** Brute-force lockout state of a resolved candidate. */
  public enum LockoutState {
    NONE,
    TEMPORARY,
    PERMANENT
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

  public static Resolution resolveAuthenticatedUser(
      KeycloakSession session,
      RealmModel realm,
      List<String> matchAttributes,
      Map<String, String> submittedValues,
      String password) {
    if (matchAttributes == null || matchAttributes.isEmpty()) {
      log.warn("resolveAuthenticatedUser(): no matchAttributes configured");
      return dummyFailure(session, realm);
    }
    if (password == null || password.isBlank()) {
      return dummyFailure(session, realm);
    }

    Map<String, UserModel> candidatesById = null;
    for (String attribute : matchAttributes) {
      String value = submittedValues.get(attribute);
      if (value == null || value.isBlank()) {
        return dummyFailure(session, realm);
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
        return dummyFailure(session, realm);
      }
    }

    List<UserModel> enabledCandidates =
        candidatesById.values().stream().filter(UserModel::isEnabled).collect(Collectors.toList());
    Map<UserModel, LockoutState> lockoutStates =
        enabledCandidates.stream()
            .collect(Collectors.toMap(Function.identity(), c -> lockoutStateOf(session, realm, c)));
    List<UserModel> lockedOutCandidates =
        enabledCandidates.stream()
            .filter(candidate -> lockoutStates.get(candidate) != LockoutState.NONE)
            .collect(Collectors.toList());
    List<UserModel> viableCandidates =
        enabledCandidates.stream()
            .filter(candidate -> lockoutStates.get(candidate) == LockoutState.NONE)
            .collect(Collectors.toList());

    if (viableCandidates.isEmpty()) {
      // Every enabled candidate for these attributes is currently locked out: only report the
      // specific lockout when there is exactly one such account to attribute it to - an ambiguous
      // set of locked accounts must stay as generic a failure as any other ambiguous outcome.
      if (lockedOutCandidates.size() == 1) {
        UserModel locked = lockedOutCandidates.get(0);
        return Resolution.lockedOut(locked, lockoutStates.get(locked));
      }
      return dummyFailure(session, realm);
    }

    if (viableCandidates.size() == 1) {
      UserModel candidate = viableCandidates.get(0);
      if (isPasswordValid(candidate, password)) {
        return Resolution.success(candidate);
      }
      return Resolution.failureAttributedTo(candidate);
    }

    List<UserModel> passwordMatches =
        viableCandidates.stream()
            .filter(candidate -> isPasswordValid(candidate, password))
            .collect(Collectors.toList());

    if (passwordMatches.size() == 1) {
      return Resolution.success(passwordMatches.get(0));
    }
    if (passwordMatches.size() > 1) {
      log.warnv(
          "resolveAuthenticatedUser(): ambiguous match, {0} candidates matched the submitted"
              + " password",
          passwordMatches.size());
    }
    return Resolution.failure();
  }

  private static Resolution dummyFailure(KeycloakSession session, RealmModel realm) {
    performDummyHash(session, realm);
    return Resolution.failure();
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
