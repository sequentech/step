// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlow;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;
import org.mockito.ArgumentCaptor;

/** The COMELEC enrollment: the details form, a code, the identity check, then the lookup. */
class StepBackTest {
  private static final URI REFRESH =
      URI.create("https://login.example/realms/r/login-actions/registration?client_id=c&tab_id=t");

  private final RealmModel realm = mock(RealmModel.class);
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final Map<String, ExecutionStatus> statuses = new HashMap<>();
  private final Map<String, List<AuthenticationExecutionModel>> flows = new HashMap<>();

  private AuthenticationExecutionModel form;
  private AuthenticationExecutionModel code;
  private AuthenticationExecutionModel identity;
  private AuthenticationExecutionModel countryCheck;
  private AuthenticationExecutionModel countryCondition;
  private AuthenticationExecutionModel lookup;

  @BeforeEach
  void setUp() {
    AuthenticationFlowModel top = flow("registration", "basic-flow");
    when(context.getTopLevelFlow()).thenReturn(top);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getRefreshUrl(false)).thenReturn(REFRESH);
    when(authSession.getExecutionStatus()).thenReturn(statuses);
    doAnswer(
            invocation -> {
              statuses.put(invocation.getArgument(0), invocation.getArgument(1));
              return null;
            })
        .when(authSession)
        .setExecutionStatus(anyString(), any());
    when(realm.getAuthenticationExecutionsStream(anyString()))
        .thenAnswer(
            invocation -> flows.getOrDefault(invocation.getArgument(0), List.of()).stream());

    flow("registration-form", AuthenticationFlow.FORM_FLOW);
    form = subflow("registration", "registration-form");
    add("registration-form", execution("registration-form", "deferred-registration"));
    code = add("registration", execution("registration", "message-otp-authenticator"));
    identity = add("registration", execution("registration", "scanovate-authenticator"));
    flow("country-check", "basic-flow");
    countryCheck = subflow("registration", "country-check");
    countryCondition = add("country-check", execution("country-check", "conditional-auth-note"));
    lookup = add("registration", execution("registration", "lookup-and-update-user"));
  }

  @Test
  void requestedOnlyByTheBackButton() {
    MultivaluedMap<String, String> back = new MultivaluedHashMap<>();
    back.putSingle(StepBack.PARAM, "true");
    assertTrue(StepBack.requested(back));
    assertFalse(StepBack.requested(new MultivaluedHashMap<>()));
    MultivaluedMap<String, String> other = new MultivaluedHashMap<>();
    other.putSingle(StepBack.PARAM, "false");
    assertFalse(StepBack.requested(other));
  }

  @Test
  void theCodeStepGoesBackToTheDetailsForm() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(code.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(code);

    assertTrue(StepBack.available(context));
    assertTrue(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(form.getId()));
    assertRedirectedToTheFlow();
    // The code step reached again waits for the resend timer.
    verify(authSession).setAuthNote(Utils.CODE_PACED, "true");
  }

  @Test
  void theIdentityStepGoesBackToTheCodeStep() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(code.getId(), ExecutionStatus.SUCCESS);
    statuses.put(identity.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(identity);

    assertTrue(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.SUCCESS, statuses.get(form.getId()));
    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(code.getId()));
    assertRedirectedToTheFlow();
    verify(authSession).setAuthNote(Utils.CODE_PACED, "true");
  }

  @Test
  void stepsInsideSubflowsReopenTheirSubflows() {
    AuthenticationExecutionModel later = add("country-check", execution("country-check", "deny"));
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(code.getId(), ExecutionStatus.SUCCESS);
    statuses.put(identity.getId(), ExecutionStatus.SUCCESS);
    statuses.put(countryCheck.getId(), ExecutionStatus.CHALLENGED);
    statuses.put(countryCondition.getId(), ExecutionStatus.EVALUATED_TRUE);
    statuses.put(later.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(later);

    assertTrue(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(identity.getId()));
    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(countryCheck.getId()));
    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(countryCondition.getId()));
    assertEquals(ExecutionStatus.SUCCESS, statuses.get(code.getId()));
  }

  @Test
  void skippedStepsAreNotGoneBackTo() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(code.getId(), ExecutionStatus.SKIPPED);
    statuses.put(identity.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(identity);

    assertTrue(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(form.getId()));
    assertEquals(ExecutionStatus.CHALLENGED, statuses.get(code.getId()));
  }

  @Test
  void theFirstStepHasNothingToGoBackTo() {
    statuses.put(code.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(code);

    assertFalse(StepBack.available(context));
    assertFalse(StepBack.toPreviousStep(context));

    verify(context, never()).forceChallenge(any());
    verify(authSession, never()).setAuthNote(eq(Utils.CODE_PACED), anyString());
  }

  @Test
  void notOnceAUserIsAuthenticated() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(code.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(code);
    when(authSession.getAuthenticatedUser()).thenReturn(mock(UserModel.class));

    assertFalse(StepBack.available(context));
    assertFalse(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.SUCCESS, statuses.get(form.getId()));
    verify(context, never()).forceChallenge(any());
    verify(authSession, never()).setAuthNote(eq(Utils.CODE_PACED), anyString());
  }

  /** A code step after the lookup decided can't go back: the application is already recorded. */
  @Test
  void notOnceTheDecisionIsRecorded() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    statuses.put(identity.getId(), ExecutionStatus.SUCCESS);
    statuses.put(lookup.getId(), ExecutionStatus.SUCCESS);
    AuthenticationExecutionModel lateCode =
        add("registration", execution("registration", "message-otp-authenticator"));
    statuses.put(lateCode.getId(), ExecutionStatus.CHALLENGED);
    when(context.getExecution()).thenReturn(lateCode);
    when(authSession.getAuthNote(Utils.DECISION_RECORDED)).thenReturn("true");

    assertFalse(StepBack.available(context));
    assertFalse(StepBack.toPreviousStep(context));

    assertEquals(ExecutionStatus.SUCCESS, statuses.get(lookup.getId()));
    verify(context, never()).forceChallenge(any());
  }

  @Test
  void anExecutionOutsideTheFlowHasNothingToGoBackTo() {
    statuses.put(form.getId(), ExecutionStatus.SUCCESS);
    when(context.getExecution()).thenReturn(execution("elsewhere", "other"));

    assertFalse(StepBack.toPreviousStep(context));
  }

  private void assertRedirectedToTheFlow() {
    ArgumentCaptor<Response> response = ArgumentCaptor.forClass(Response.class);
    verify(context).forceChallenge(response.capture());
    assertEquals(Response.Status.SEE_OTHER.getStatusCode(), response.getValue().getStatus());
    assertEquals(REFRESH, response.getValue().getLocation());
  }

  private AuthenticationFlowModel flow(String id, String providerId) {
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId(id);
    flow.setAlias(id);
    flow.setProviderId(providerId);
    when(realm.getAuthenticationFlowById(id)).thenReturn(flow);
    flows.putIfAbsent(id, new ArrayList<>());
    return flow;
  }

  private AuthenticationExecutionModel subflow(String parent, String flowId) {
    AuthenticationExecutionModel execution = execution(parent, null);
    execution.setId(parent + "/" + flowId);
    execution.setAuthenticatorFlow(true);
    execution.setFlowId(flowId);
    return add(parent, execution);
  }

  private AuthenticationExecutionModel add(String parent, AuthenticationExecutionModel execution) {
    List<AuthenticationExecutionModel> executions = flows.get(parent);
    execution.setPriority(executions.size());
    executions.add(execution);
    return execution;
  }

  private static AuthenticationExecutionModel execution(String parent, String authenticator) {
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setId(parent + "/" + authenticator);
    execution.setParentFlow(parent);
    execution.setAuthenticator(authenticator);
    execution.setRequirement(AuthenticationExecutionModel.Requirement.REQUIRED);
    return execution;
  }
}
