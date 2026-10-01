// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;

/**
 * Which of the flow's code steps the current one is, among those that run for this voter: a flow
 * can ask for several codes in a row, which otherwise look like the first one failed.
 *
 * <p>Conditional subflows are evaluated with their own conditions, with the notes the voter's
 * session has now. When a condition can't be evaluated, there is no count: a wrong one is worse
 * than none.
 *
 * @param request the position of the current code, from 1
 * @param requests the number of codes
 */
@JBossLog
public record CodeProgress(int request, int requests) {
  public static final String POLICY = "code-progress-policy";

  /** The progress to show, with the {@link CodeProgressPolicy#SHOW} policy and several codes. */
  public static Optional<CodeProgress> of(AuthenticationFlowContext context) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    if (config == null
        || config.getConfig() == null
        || CodeProgressPolicy.fromConfig(config.getConfig().get(POLICY))
            != CodeProgressPolicy.SHOW) {
      return Optional.empty();
    }
    List<String> codes = new ArrayList<>();
    try {
      collect(context, context.getTopLevelFlow().getId(), codes);
    } catch (UnknownCondition e) {
      log.warnv("of(): not counting the codes: {0}", e.getMessage());
      return Optional.empty();
    }
    int index = codes.indexOf(context.getExecution().getId());
    if (index < 0 || codes.size() < 2) {
      return Optional.empty();
    }
    return Optional.of(new CodeProgress(index + 1, codes.size()));
  }

  private static void collect(AuthenticationFlowContext context, String flowId, List<String> codes)
      throws UnknownCondition {
    RealmModel realm = context.getRealm();
    for (AuthenticationExecutionModel execution :
        realm.getAuthenticationExecutionsStream(flowId).toList()) {
      if (execution.isDisabled()) {
        continue;
      }
      if (execution.isAuthenticatorFlow()) {
        if (!execution.isConditional() || conditionsMatch(context, execution.getFlowId())) {
          collect(context, execution.getFlowId(), codes);
        }
      } else if (MessageOTPAuthenticatorFactory.PROVIDER_ID.equals(execution.getAuthenticator())) {
        codes.add(execution.getId());
      }
    }
  }

  /** Whether all the conditions of a conditional subflow match, as Keycloak requires. */
  private static boolean conditionsMatch(AuthenticationFlowContext context, String flowId)
      throws UnknownCondition {
    for (AuthenticationExecutionModel execution :
        context.getRealm().getAuthenticationExecutionsStream(flowId).toList()) {
      if (execution.isDisabled() || execution.isAuthenticatorFlow()) {
        continue;
      }
      Optional<ConditionalAuthenticator> condition = condition(context, execution);
      if (condition.isEmpty()) {
        continue;
      }
      if (!condition.get().matchCondition(at(context, execution))) {
        return false;
      }
    }
    return true;
  }

  private static Optional<ConditionalAuthenticator> condition(
      AuthenticationFlowContext context, AuthenticationExecutionModel execution)
      throws UnknownCondition {
    AuthenticatorFactory factory =
        (AuthenticatorFactory)
            context
                .getSession()
                .getKeycloakSessionFactory()
                .getProviderFactory(Authenticator.class, execution.getAuthenticator());
    if (factory == null) {
      throw new UnknownCondition("no authenticator " + execution.getAuthenticator());
    }
    Authenticator authenticator = factory.create(context.getSession());
    return authenticator instanceof ConditionalAuthenticator conditional
        ? Optional.of(conditional)
        : Optional.empty();
  }

  /** The context as the given condition sees it: its own execution and config. */
  private static AuthenticationFlowContext at(
      AuthenticationFlowContext context, AuthenticationExecutionModel execution) {
    AuthenticatorConfigModel config =
        execution.getAuthenticatorConfig() == null
            ? null
            : context.getRealm().getAuthenticatorConfigById(execution.getAuthenticatorConfig());
    return (AuthenticationFlowContext)
        Proxy.newProxyInstance(
            AuthenticationFlowContext.class.getClassLoader(),
            new Class<?>[] {AuthenticationFlowContext.class},
            (proxy, method, args) -> {
              switch (method.getName()) {
                case "getAuthenticatorConfig":
                  return config;
                case "getExecution":
                  return execution;
                default:
                  try {
                    return method.invoke(context, args);
                  } catch (InvocationTargetException e) {
                    throw e.getCause();
                  }
              }
            });
  }

  private static class UnknownCondition extends Exception {
    UnknownCondition(String message) {
      super(message);
    }
  }
}
