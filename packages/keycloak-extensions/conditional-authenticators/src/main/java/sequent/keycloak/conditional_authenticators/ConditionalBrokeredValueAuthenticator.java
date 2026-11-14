// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import java.util.Map;
import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.broker.AbstractIdpAuthenticator;
import org.keycloak.authentication.authenticators.broker.util.PostBrokerLoginConstants;
import org.keycloak.authentication.authenticators.broker.util.SerializedBrokeredIdentityContext;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.conditional_authenticators.ConditionalBrokeredValueAuthenticatorFactory.Target;

/**
 * Matches when the username or a user attribute of the login is present and, if a regex is
 * configured, matches it entirely.
 *
 * <p>In the first broker login and post broker login flows it reads the brokered identity, so it
 * sees the values the identity provider mappers produced before the user is created. In other flows
 * it reads the user.
 */
@JBossLog
public class ConditionalBrokeredValueAuthenticator implements ConditionalAuthenticator {
  public static final ConditionalBrokeredValueAuthenticator SINGLETON =
      new ConditionalBrokeredValueAuthenticator();

  /** Set by the attribute lookup mapper when it could not resolve the username. */
  static final String USERNAME_UNRESOLVED_KEY = "sequent.brokered-username-unresolved";

  private static final String EMAIL = "email";
  private static final String FIRST_NAME = "firstName";
  private static final String LAST_NAME = "lastName";

  @Override
  public boolean matchCondition(AuthenticationFlowContext context) {
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    Map<String, String> config = authConfig == null ? null : authConfig.getConfig();
    if (config == null) {
      throw new IllegalStateException("Brokered value condition has no configuration");
    }
    Target target = ConditionalBrokeredValueAuthenticatorFactory.target(config);
    String attribute = config.get(ConditionalBrokeredValueAuthenticatorFactory.CONF_ATTRIBUTE);
    if (target == Target.USER_ATTRIBUTE && (attribute == null || attribute.isBlank())) {
      throw new IllegalStateException("Brokered value condition has no attribute configured");
    }
    Pattern pattern = compile(config.get(ConditionalBrokeredValueAuthenticatorFactory.CONF_REGEX));
    boolean negate =
        Boolean.parseBoolean(config.get(ConditionalBrokeredValueAuthenticatorFactory.CONF_NEGATE));

    String value = value(context, target, attribute);
    boolean matches =
        value != null && !value.isBlank() && (pattern == null || pattern.matcher(value).matches());
    log.debugf(
        "Condition %s: %s %s matches=%s negate=%s",
        authConfig.getAlias(), target, target == Target.USERNAME ? "" : attribute, matches, negate);
    return matches != negate;
  }

  private static Pattern compile(String regex) {
    if (regex == null || regex.isEmpty()) {
      return null;
    }
    try {
      return Pattern.compile(regex);
    } catch (PatternSyntaxException e) {
      throw new IllegalStateException("Invalid regex in brokered value condition", e);
    }
  }

  private static String value(AuthenticationFlowContext context, Target target, String attribute) {
    SerializedBrokeredIdentityContext brokered =
        brokeredIdentity(context.getAuthenticationSession());
    if (brokered != null) {
      return switch (target) {
        case USERNAME ->
            brokered.getContextData().containsKey(USERNAME_UNRESOLVED_KEY)
                ? null
                : brokered.getModelUsername();
        case USER_ATTRIBUTE -> brokeredAttribute(brokered, attribute);
      };
    }
    UserModel user = context.getUser();
    if (user == null) {
      return null;
    }
    return switch (target) {
      case USERNAME -> user.getUsername();
      case USER_ATTRIBUTE -> userAttribute(user, attribute);
    };
  }

  private static SerializedBrokeredIdentityContext brokeredIdentity(
      AuthenticationSessionModel authSession) {
    if (authSession == null) {
      return null;
    }
    SerializedBrokeredIdentityContext firstLogin =
        SerializedBrokeredIdentityContext.readFromAuthenticationSession(
            authSession, AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE);
    if (firstLogin != null) {
      return firstLogin;
    }
    return SerializedBrokeredIdentityContext.readFromAuthenticationSession(
        authSession, PostBrokerLoginConstants.PBL_BROKERED_IDENTITY_CONTEXT);
  }

  private static String brokeredAttribute(
      SerializedBrokeredIdentityContext brokered, String attribute) {
    if (attribute.equalsIgnoreCase(EMAIL)) {
      return brokered.getEmail();
    }
    if (attribute.equalsIgnoreCase(FIRST_NAME)) {
      return brokered.getFirstName();
    }
    if (attribute.equalsIgnoreCase(LAST_NAME)) {
      return brokered.getLastName();
    }
    return brokered.getFirstAttribute(attribute);
  }

  private static String userAttribute(UserModel user, String attribute) {
    if (attribute.equalsIgnoreCase(EMAIL)) {
      return user.getEmail();
    }
    if (attribute.equalsIgnoreCase(FIRST_NAME)) {
      return user.getFirstName();
    }
    if (attribute.equalsIgnoreCase(LAST_NAME)) {
      return user.getLastName();
    }
    return user.getFirstAttribute(attribute);
  }

  @Override
  public void action(AuthenticationFlowContext context) {}

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public void close() {}
}
