// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;

/**
 * The contact-change gate (R7-01): the message OTP steps that could send a code elsewhere than the
 * voter's channel. Only steps that can run count: executions reachable from a top-level flow, not
 * DISABLED. CHOSEN and NONE (which sends nothing) don't block.
 */
class CodeStepGateTest {
  private final RealmModel realm = mock(RealmModel.class);
  private final Map<String, AuthenticationFlowModel> flows = new LinkedHashMap<>();
  private final Map<String, List<AuthenticationExecutionModel>> executions = new HashMap<>();
  private int ids;

  @BeforeEach
  void setup() {
    when(realm.getAuthenticationFlowsStream()).thenAnswer(i -> flows.values().stream());
    when(realm.getAuthenticationExecutionsStream(org.mockito.ArgumentMatchers.anyString()))
        .thenAnswer(i -> executions.getOrDefault(i.<String>getArgument(0), List.of()).stream());
    when(realm.getAuthenticationFlowById(org.mockito.ArgumentMatchers.anyString()))
        .thenAnswer(i -> flows.get(i.<String>getArgument(0)));
  }

  private AuthenticationFlowModel flow(String id, boolean topLevel) {
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId(id);
    flow.setAlias(id);
    flow.setTopLevel(topLevel);
    flows.put(id, flow);
    return flow;
  }

  private AuthenticationExecutionModel codeStep(String flowId, String courier, Requirement req) {
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setId("step-" + ++ids);
    execution.setParentFlow(flowId);
    execution.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    execution.setRequirement(req);
    if (courier != null) {
      String configId = "config-" + ids;
      execution.setAuthenticatorConfig(configId);
      AuthenticatorConfigModel config = new AuthenticatorConfigModel();
      config.setConfig(Map.of(Utils.MESSAGE_COURIER_ATTRIBUTE, courier));
      when(realm.getAuthenticatorConfigById(configId)).thenReturn(config);
    }
    executions.computeIfAbsent(flowId, k -> new ArrayList<>()).add(execution);
    return execution;
  }

  private void subFlow(String parentId, String childId, Requirement req) {
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setId("sub-" + childId + "-" + ++ids);
    execution.setParentFlow(parentId);
    execution.setAuthenticatorFlow(true);
    execution.setFlowId(childId);
    execution.setRequirement(req);
    executions.computeIfAbsent(parentId, k -> new ArrayList<>()).add(execution);
  }

  @Test
  void chosenAndNoneStepsDoNotBlock() {
    flow("browser", true);
    codeStep("browser", "CHOSEN", Requirement.REQUIRED);
    codeStep("browser", "NONE", Requirement.REQUIRED);
    assertEquals(List.of(), Utils.codeStepsNotOnVoterChannel(realm));
  }

  @Test
  void configuredCouriersAndUnconfiguredStepsBlock() {
    flow("browser", true);
    AuthenticationExecutionModel both = codeStep("browser", "BOTH", Requirement.REQUIRED);
    AuthenticationExecutionModel none = codeStep("browser", null, Requirement.ALTERNATIVE);
    codeStep("browser", "EMAIL", Requirement.DISABLED);
    assertEquals(List.of(both.getId(), none.getId()), Utils.codeStepsNotOnVoterChannel(realm));
  }

  /** An orphan sub-flow never runs, and no migration reaches it: it doesn't block. */
  @Test
  void anOrphanSubFlowDoesNotBlock() {
    flow("browser", true);
    codeStep("browser", "CHOSEN", Requirement.REQUIRED);
    flow("orphan", false);
    codeStep("orphan", "BOTH", Requirement.REQUIRED);
    assertEquals(List.of(), Utils.codeStepsNotOnVoterChannel(realm));
  }

  /** A step nested in sub-flows of a top-level flow runs, so it blocks. */
  @Test
  void aNestedStepReachedFromATopLevelFlowBlocks() {
    flow("browser", true);
    flow("forms", false);
    flow("second-factor", false);
    subFlow("browser", "forms", Requirement.ALTERNATIVE);
    subFlow("forms", "second-factor", Requirement.CONDITIONAL);
    AuthenticationExecutionModel nested = codeStep("second-factor", "BOTH", Requirement.REQUIRED);
    assertEquals(List.of(nested.getId()), Utils.codeStepsNotOnVoterChannel(realm));
  }

  /** A DISABLED sub-flow never runs: its steps don't block. Cycles end. */
  @Test
  void aDisabledSubFlowDoesNotBlockAndCyclesEnd() {
    flow("browser", true);
    flow("off", false);
    flow("loop", false);
    subFlow("browser", "off", Requirement.DISABLED);
    codeStep("off", "BOTH", Requirement.REQUIRED);
    subFlow("browser", "loop", Requirement.REQUIRED);
    subFlow("loop", "browser", Requirement.REQUIRED);
    subFlow("loop", "loop", Requirement.REQUIRED);
    AuthenticationExecutionModel looped = codeStep("loop", "SMS", Requirement.REQUIRED);
    assertEquals(List.of(looped.getId()), Utils.codeStepsNotOnVoterChannel(realm));
  }

  /** A step reached from two top-level flows is counted once. */
  @Test
  void aSharedSubFlowIsCountedOnce() {
    flow("browser", true);
    flow("reset", true);
    flow("shared", false);
    subFlow("browser", "shared", Requirement.REQUIRED);
    subFlow("reset", "shared", Requirement.REQUIRED);
    AuthenticationExecutionModel shared = codeStep("shared", "BOTH", Requirement.REQUIRED);
    assertEquals(List.of(shared.getId()), Utils.codeStepsNotOnVoterChannel(realm));
  }
}
