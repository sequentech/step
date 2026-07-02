// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Conditional Client Authenticator allows you to create conditional flows that only execute when a
 * specific authNote is present in the AuthSession with a given value.
 */
@JBossLog
public class ConditionalAuthNoteAuthenticator implements ConditionalAuthenticator {
  public static final ConditionalAuthNoteAuthenticator SINGLETON =
      new ConditionalAuthNoteAuthenticator();

  @Override
  public boolean matchCondition(AuthenticationFlowContext context) {
    log.info("matchCondition()");
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    if (authConfig == null) {
      log.infov("matchCondition(): NULL found authConfig={0}", authConfig);
      return false;
    }
    if (authConfig.getConfig() == null) {
      log.infov("matchCondition(): NULL found authConfig.getConfig()={0}", authConfig.getConfig());
      return false;
    }

    String requiredAuthNoteKey =
        authConfig
            .getConfig()
            .get(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_KEY);
    String requiredAuthNoteValue =
        authConfig
            .getConfig()
            .get(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_VALUE);
    boolean valueIsRegex =
        Boolean.parseBoolean(
            authConfig
                .getConfig()
                .get(ConditionalAuthNoteAuthenticatorFactory.CONF_VALUE_IS_REGEX));
    boolean negateOutput =
        Boolean.parseBoolean(
            authConfig.getConfig().get(ConditionalAuthNoteAuthenticatorFactory.CONF_NEGATE));

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (authSession == null) {
      log.infov("matchCondition(): NULL found authSession={0}", authSession);
      return false;
    }
    String authNoteValue = authSession.getAuthNote(requiredAuthNoteKey);
    if (authNoteValue == null) {
      log.infov("matchCondition(): requiredAuthNoteKey={0} not present", requiredAuthNoteKey);
      return false;
    }
    boolean authNoteMatch;
    if (requiredAuthNoteValue == null) {
      authNoteMatch = authNoteValue.isBlank() || authNoteValue.isEmpty();
    } else if (valueIsRegex) {
      try {
        authNoteMatch = authNoteValue.matches(requiredAuthNoteValue);
      } catch (java.util.regex.PatternSyntaxException e) {
        log.warnv(
            "matchCondition(): invalid regex pattern [{0}]: {1}",
            requiredAuthNoteValue, e.getMessage());
        authNoteMatch = false;
      }
    } else {
      authNoteMatch = requiredAuthNoteValue.equals(authNoteValue);
    }
    log.infov(
        "matchCondition(): key={0}, value={1}, note={2}, regex={3}, negate[{4}] != match[{5}]",
        requiredAuthNoteKey,
        requiredAuthNoteValue,
        authNoteValue,
        valueIsRegex,
        negateOutput,
        authNoteMatch);
    return negateOutput != authNoteMatch;
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    // Not used
  }

  @Override
  public boolean requiresUser() {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // Not used
  }

  @Override
  public void close() {
    // Does nothing
  }
}
