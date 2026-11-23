// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.login_timeout;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;
import org.keycloak.services.managers.ClientSessionCode.ActionType;

class LoginTimeoutResourceProviderTest {

  @Test
  void loginStepsUseTheLoginTimeout() {
    assertEquals(ActionType.LOGIN, LoginTimeoutResourceProvider.actionTypeOf("AUTHENTICATE", null));
    assertEquals(
        ActionType.LOGIN,
        LoginTimeoutResourceProvider.actionTypeOf("AUTHENTICATE", "authenticate"));
    assertEquals(
        ActionType.LOGIN,
        LoginTimeoutResourceProvider.actionTypeOf("AUTHENTICATE", "registration"));
    assertEquals(
        ActionType.LOGIN,
        LoginTimeoutResourceProvider.actionTypeOf("AUTHENTICATE", "first-broker-login"));
    assertEquals(ActionType.LOGIN, LoginTimeoutResourceProvider.actionTypeOf(null, null));
  }

  @Test
  void requiredActionsAndResetCredentialsUseTheActionTimeout() {
    assertEquals(
        ActionType.USER, LoginTimeoutResourceProvider.actionTypeOf("REQUIRED_ACTIONS", null));
    assertEquals(
        ActionType.USER,
        LoginTimeoutResourceProvider.actionTypeOf("AUTHENTICATE", "reset-credentials"));
  }

  @Test
  void secondsLeftCountsDownFromTheLastPage() {
    assertEquals(1800, LoginTimeoutResourceProvider.secondsLeft(1000, 1800, 1000));
    assertEquals(1, LoginTimeoutResourceProvider.secondsLeft(1000, 1800, 2799));
  }

  @Test
  void expiredLoginsHaveNoSecondsLeft() {
    // Keycloak rejects the form once now - timestamp reaches the lifespan.
    assertEquals(0, LoginTimeoutResourceProvider.secondsLeft(1000, 1800, 2800));
    assertEquals(0, LoginTimeoutResourceProvider.secondsLeft(1000, 1800, 99999));
  }

  @Test
  void eachStepUsesTheLifespanKeycloakChecks() {
    assertEquals(1800, LoginTimeoutResourceProvider.lifespanOf(ActionType.LOGIN, 1800, 300, 60));
    assertEquals(300, LoginTimeoutResourceProvider.lifespanOf(ActionType.USER, 1800, 300, 60));
    assertEquals(60, LoginTimeoutResourceProvider.lifespanOf(ActionType.CLIENT, 1800, 300, 60));
  }

  @Test
  void realmsWithoutALoginTimeoutUseTheActionTimeout() {
    assertEquals(300, LoginTimeoutResourceProvider.lifespanOf(ActionType.LOGIN, 0, 300, 60));
  }
}
