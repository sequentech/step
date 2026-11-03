// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.when;

import java.net.URI;
import java.util.HashMap;
import java.util.Map;
import java.util.stream.Stream;
import org.keycloak.authentication.AuthenticationFlow;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.RealmModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;

/** A flow whose first step (an enrollment details form, or a reset's identifier) is done. */
final class StepBackFlow {
  private StepBackFlow() {}

  /** The enrollment's details form, done, then the context's code step. */
  static Map<String, ExecutionStatus> afterTheDetailsForm(
      AuthenticationFlowContext context, RealmModel realm, AuthenticationSessionModel authSession) {
    AuthenticationFlowModel formFlow = new AuthenticationFlowModel();
    formFlow.setId("registration-form");
    formFlow.setProviderId(AuthenticationFlow.FORM_FLOW);
    when(realm.getAuthenticationFlowById(formFlow.getId())).thenReturn(formFlow);
    AuthenticationExecutionModel details = new AuthenticationExecutionModel();
    details.setId("details");
    details.setAuthenticatorFlow(true);
    details.setFlowId(formFlow.getId());
    return afterAStep(context, realm, authSession, details);
  }

  /**
   * The details form and the context's code step, done, then the identity check: the step Back
   * leaves to run the code step again. Returns the identity check's execution.
   */
  static AuthenticationExecutionModel atTheIdentityCheck(
      AuthenticationFlowContext context, RealmModel realm, AuthenticationSessionModel authSession) {
    AuthenticationExecutionModel code = context.getExecution();
    AuthenticationExecutionModel identity = new AuthenticationExecutionModel();
    identity.setId("identity");
    identity.setAuthenticator("scanovate-authenticator");
    Map<String, ExecutionStatus> statuses = afterTheDetailsForm(context, realm, authSession);
    AuthenticationExecutionModel details = new AuthenticationExecutionModel();
    details.setId("details");
    details.setAuthenticatorFlow(true);
    details.setFlowId("registration-form");
    when(realm.getAuthenticationExecutionsStream("top"))
        .thenAnswer(i -> Stream.of(details, code, identity));
    statuses.put(code.getId(), ExecutionStatus.SUCCESS);
    statuses.put(identity.getId(), ExecutionStatus.CHALLENGED);
    return identity;
  }

  /** A reset's identifier step (an authenticator), done, then the context's code step. */
  static Map<String, ExecutionStatus> afterTheIdentifier(
      AuthenticationFlowContext context, RealmModel realm, AuthenticationSessionModel authSession) {
    AuthenticationExecutionModel identifier = new AuthenticationExecutionModel();
    identifier.setId("choose-user");
    identifier.setAuthenticator("forgot.choose-user");
    return afterAStep(context, realm, authSession, identifier);
  }

  private static Map<String, ExecutionStatus> afterAStep(
      AuthenticationFlowContext context,
      RealmModel realm,
      AuthenticationSessionModel authSession,
      AuthenticationExecutionModel previous) {
    Map<String, ExecutionStatus> statuses = new HashMap<>();
    AuthenticationFlowModel top = new AuthenticationFlowModel();
    top.setId("top");
    AuthenticationExecutionModel code = context.getExecution();
    code.setId("code");
    when(context.getTopLevelFlow()).thenReturn(top);
    when(realm.getAuthenticationExecutionsStream(top.getId()))
        .thenAnswer(i -> Stream.of(previous, code));
    when(context.getRefreshUrl(false)).thenReturn(URI.create("https://login.example/"));
    when(authSession.getExecutionStatus()).thenReturn(statuses);
    doAnswer(i -> statuses.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setExecutionStatus(anyString(), any());
    statuses.put(previous.getId(), ExecutionStatus.SUCCESS);
    return statuses;
  }
}
