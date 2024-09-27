// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    boolean authNoteMatch =
        requiredAuthNoteValue == null
            ? authNoteValue.isBlank() || authNoteValue.isEmpty()
            : requiredAuthNoteValue.equals(authNoteValue);
    log.infov(
        "matchCondition(): requiredAuthNoteKey={0}, requiredAuthNoteValue={1}, authNoteValue={2}, negateOutput[{3}] != authNoteMatch[{4}]",
        requiredAuthNoteKey, requiredAuthNoteValue, authNoteValue, negateOutput, authNoteMatch);
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
