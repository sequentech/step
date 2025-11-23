// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Conditional Client Authenticator allows you to create conditional flows that only execute when a
 * specific client is performing the authentication
 */
@JBossLog
public class ConditionalClientAuthenticator implements ConditionalAuthenticator {
  public static final ConditionalClientAuthenticator SINGLETON =
      new ConditionalClientAuthenticator();

  @Override
  public boolean matchCondition(AuthenticationFlowContext context) {
    log.info("matchCondition()");
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    if (authConfig == null) {
      log.infov("matchCondition(): NULL found authConfig={0}", authConfig);
      return false;
    }
    log.infov("matchCondition(): alias={0}", authConfig.getAlias());
    if (authConfig.getConfig() == null) {
      log.infov("matchCondition(): NULL found authConfig.getConfig()={0}", authConfig.getConfig());
      return false;
    }

    String requiredClientId =
        authConfig.getConfig().get(ConditionalClientAuthenticatorFactory.CONDITIONAL_CLIENT_ID);
    boolean negateOutput =
        Boolean.parseBoolean(
            authConfig.getConfig().get(ConditionalClientAuthenticatorFactory.CONF_NEGATE));

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (authSession == null) {
      log.infov("matchCondition(): NULL found authSession={0}", authSession);
      return false;
    }
    ClientModel client = authSession.getClient();
    if (client == null) {
      log.infov("matchCondition(): NULL found client={0}", client);
      return false;
    }
    boolean clientIdMatch = requiredClientId.equals(client.getClientId());
    log.infov(
        "matchCondition(): client.getClientId()={0}, requiredClientId={1}, negateOutput[{2}] != clientIdMatch[{3}]",
        client.getClientId(), requiredClientId, negateOutput, clientIdMatch);
    return negateOutput != clientIdMatch;
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
