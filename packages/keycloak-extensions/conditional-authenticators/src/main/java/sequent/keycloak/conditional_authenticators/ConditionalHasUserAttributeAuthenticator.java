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

/**
 * Conditional Client Authenticator allows you to create conditional flows that only execute when a
 * specific user attribute is present in the user.
 */
@JBossLog
public class ConditionalHasUserAttributeAuthenticator implements ConditionalAuthenticator {
  public static final ConditionalHasUserAttributeAuthenticator SINGLETON =
      new ConditionalHasUserAttributeAuthenticator();

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

    String requiredUserAttributeKey =
        authConfig
            .getConfig()
            .get(ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY);
    boolean negateOutput =
        Boolean.parseBoolean(
            authConfig
                .getConfig()
                .get(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE));

    UserModel user = context.getUser();
    if (user == null) {
      log.infov("matchCondition(): NULL found user={0}", user);
      return false;
    }
    String userAttributeValue = user.getFirstAttribute(requiredUserAttributeKey);
    boolean userAttributePresent = (userAttributeValue != null && !userAttributeValue.isBlank());
    log.infov(
        "matchCondition(): requiredUserAttributeKey={0}, userAttributeValue={1}, negateOutput[{2}] != userAttributePresent[{3}]",
        requiredUserAttributeKey, userAttributeValue, negateOutput, userAttributePresent);

    return negateOutput != userAttributePresent;
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action()");
    // Not used
  }

  @Override
  public boolean requiresUser() {
    log.info("requiresUser()");
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // Not used
  }

  @Override
  public void close() {}
}
