// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlow;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.RealmModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;

/**
 * Takes the voter back to the step before the current one. Keycloak can't: an earlier step's
 * address shows "page expired".
 *
 * <p>The previous step is the last one done before the current one: an authenticator, or a whole
 * form flow. It, the subflows it is in and every execution after it are marked as not done, and the
 * browser is sent to the flow, which runs again from the first step not done.
 *
 * <p>Only while no user is authenticated: going back must not keep an identity a step established.
 */
@JBossLog
public final class StepBack {
  /** The form parameter of the Back button. */
  public static final String PARAM = "back";

  /** The page attribute that says whether the page offers Back. */
  public static final String ATTRIBUTE = "stepBack";

  private StepBack() {}

  /** Whether the submitted form is the Back button. */
  public static boolean requested(MultivaluedMap<String, String> formData) {
    return "true".equals(formData.getFirst(PARAM));
  }

  /** Whether the current step can go back. */
  public static boolean available(AuthenticationFlowContext context) {
    return context.getAuthenticationSession().getAuthenticatedUser() == null
        && !toRedo(context).isEmpty();
  }

  /**
   * Goes back to the previous step.
   *
   * @return false, changing nothing, when there is no step to go back to
   */
  public static boolean toPreviousStep(AuthenticationFlowContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (authSession.getAuthenticatedUser() != null) {
      log.info("toPreviousStep: a user is authenticated, not going back");
      return false;
    }
    List<String> redo = toRedo(context);
    if (redo.isEmpty()) {
      log.info("toPreviousStep: no previous step");
      return false;
    }
    log.infov("toPreviousStep: running {0} again", redo);
    // setExecutionStatus stores the change; the map Keycloak hands out is only a view.
    redo.forEach(id -> authSession.setExecutionStatus(id, ExecutionStatus.CHALLENGED));
    authSession.setAuthNote(Utils.CODE_PACED, "true");
    context.forceChallenge(Response.seeOther(context.getRefreshUrl(false)).build());
    return true;
  }

  /** A position in the flow, with the subflow executions it is in. */
  private record Step(AuthenticationExecutionModel execution, boolean single, List<String> in) {}

  /**
   * The executions with a status to run again, or none without a previous step or once the
   * enrollment's decision is recorded.
   */
  private static List<String> toRedo(AuthenticationFlowContext context) {
    if (Boolean.parseBoolean(
        context.getAuthenticationSession().getAuthNote(Utils.DECISION_RECORDED))) {
      return List.of();
    }
    AuthenticationExecutionModel current = context.getExecution();
    AuthenticationFlowModel top = context.getTopLevelFlow();
    if (current == null || top == null) {
      return List.of();
    }
    List<Step> steps = new ArrayList<>();
    walk(context.getRealm(), top.getId(), List.of(), steps);
    int currentIndex = -1;
    for (int i = 0; i < steps.size(); i++) {
      if (steps.get(i).execution().getId().equals(current.getId())) {
        currentIndex = i;
        break;
      }
    }
    Map<String, ExecutionStatus> statuses = context.getAuthenticationSession().getExecutionStatus();
    for (int i = currentIndex - 1; i >= 0; i--) {
      Step step = steps.get(i);
      if (step.single() && statuses.get(step.execution().getId()) == ExecutionStatus.SUCCESS) {
        Set<String> redo = new LinkedHashSet<>(step.in());
        for (int j = i; j <= currentIndex; j++) {
          redo.add(steps.get(j).execution().getId());
        }
        return redo.stream().filter(statuses::containsKey).toList();
      }
    }
    return List.of();
  }

  /** The flow's executions in the order they run. A form flow is a single step. */
  private static void walk(RealmModel realm, String flowId, List<String> in, List<Step> steps) {
    realm
        .getAuthenticationExecutionsStream(flowId)
        .forEachOrdered(
            execution -> {
              if (!execution.isAuthenticatorFlow()) {
                steps.add(new Step(execution, true, in));
                return;
              }
              AuthenticationFlowModel flow = realm.getAuthenticationFlowById(execution.getFlowId());
              boolean form =
                  flow != null && AuthenticationFlow.FORM_FLOW.equals(flow.getProviderId());
              steps.add(new Step(execution, form, in));
              if (!form) {
                List<String> inner = new ArrayList<>(in);
                inner.add(execution.getId());
                walk(realm, execution.getFlowId(), List.copyOf(inner), steps);
              }
            });
  }
}
