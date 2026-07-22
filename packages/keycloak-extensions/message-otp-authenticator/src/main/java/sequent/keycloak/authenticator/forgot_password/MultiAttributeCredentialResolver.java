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
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;

/**
 * Resolves a user from one or more configured attributes plus a password, without a username.
 * Shared by every transport that needs this: the browser form ({@link
 * MultiAttributePasswordAuthenticator}) and the IVR Direct Grant flow
 * (MultiAttributePasswordDirectGrantAuthenticator) - same rules, same failure semantics, regardless
 * of how the values arrived.
 *
 * <p>Resolution: for each configured attribute, find every user whose attribute equals the
 * submitted value, then intersect those candidate sets across all attributes. If exactly one
 * candidate's password matches the submitted password, that user authenticates. Any other outcome
 * (no candidates, no password match, more than one password match) fails generically, so callers
 * never need to distinguish "no such attributes" from "wrong password" - that distinction must not
 * leak to the end user.
 */
@JBossLog
public final class MultiAttributeCredentialResolver {

  private MultiAttributeCredentialResolver() {}

  public static Optional<UserModel> resolveAuthenticatedUser(
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
