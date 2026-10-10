// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.events.EventBuilder;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

/** A code harvest didn't send: its reason is in the event and the log; the page stays generic. */
class SendOutcomeReasonTest {
  private static final String REASON = "Not sent: the account isn't ready (NOT_CONNECTED)";

  @Test
  void aFailedSendsReasonIsInTheEventAndTheLog() {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    when(context.getEvent()).thenReturn(event);
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      MessageAttemptState state =
          Utils.recordSendOutcome(
              context,
              MessageChannel.WHATSAPP,
              SendMessageResponse.of(MessageAttemptState.FAILED, REASON));
      assertEquals(MessageAttemptState.FAILED, state);
      assertTrue(logs.text().contains(REASON), logs.text());
    }
    verify(event).detail("send_state", "FAILED");
    verify(event).detail("send_reason", REASON);
  }

  @Test
  void aSendThatMayHaveGoneOutKeepsItsReasonToo() {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    RequiredActionContext context = mock(RequiredActionContext.class);
    when(context.getEvent()).thenReturn(event);
    String late = "The provider didn't answer in time: the message may have been sent";
    assertEquals(
        MessageAttemptState.UNKNOWN,
        Utils.recordSendOutcome(
            context,
            MessageChannel.WHATSAPP,
            SendMessageResponse.of(MessageAttemptState.UNKNOWN, late)));
    verify(event).detail("send_reason", late);
  }

  @Test
  void anAcceptedSendAddsNothing() {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    when(context.getEvent()).thenReturn(event);
    assertEquals(
        MessageAttemptState.ACCEPTED,
        Utils.recordSendOutcome(
            context,
            MessageChannel.WHATSAPP,
            SendMessageResponse.of(MessageAttemptState.ACCEPTED, null)));
    verify(event, never()).detail(anyString(), anyString());
  }
}
