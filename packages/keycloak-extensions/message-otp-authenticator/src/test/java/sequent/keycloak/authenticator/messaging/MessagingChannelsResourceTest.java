// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.Response;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;

/** R3-07: the portal reads whether the event offers any channel for codes. */
class MessagingChannelsResourceTest {
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);

  private java.util.Optional<org.keycloak.models.UserModel> bearer = java.util.Optional.empty();

  private Response get(String projection) {
    KeycloakContext context = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(projection);
    return new MessagingChannelsResource(session, ignored -> bearer).channels();
  }

  private static final String PROJECTION =
      """
      {"version": 1, "channels": [
        {"channel": "EMAIL", "purposes": ["OTP"]},
        {"channel": "WHATSAPP", "purposes": ["OTP"]}],
       "election_channels": {"election-a": ["WHATSAPP"], "election-b": []}}
      """;

  /** The realm's message OTP steps, each with its courier (null: no configuration). */
  private void codeSteps(String... couriers) {
    org.keycloak.models.AuthenticationFlowModel flow =
        new org.keycloak.models.AuthenticationFlowModel();
    flow.setId("flow");
    flow.setTopLevel(true);
    List<org.keycloak.models.AuthenticationExecutionModel> executions = new java.util.ArrayList<>();
    for (int i = 0; i < couriers.length; i++) {
      org.keycloak.models.AuthenticationExecutionModel execution =
          new org.keycloak.models.AuthenticationExecutionModel();
      execution.setAuthenticator("message-otp-authenticator");
      execution.setRequirement(
          org.keycloak.models.AuthenticationExecutionModel.Requirement.REQUIRED);
      if (couriers[i] != null) {
        execution.setAuthenticatorConfig("config-" + i);
        org.keycloak.models.AuthenticatorConfigModel config =
            new org.keycloak.models.AuthenticatorConfigModel();
        config.setConfig(Map.of("messageCourierAttribute", couriers[i]));
        when(realm.getAuthenticatorConfigById("config-" + i)).thenReturn(config);
      }
      executions.add(execution);
    }
    when(realm.getAuthenticationFlowsStream()).thenAnswer(i -> java.util.stream.Stream.of(flow));
    when(realm.getAuthenticationExecutionsStream("flow")).thenAnswer(i -> executions.stream());
  }

  /** R7-01: while a code step uses a configured courier, the portal offers no contact change. */
  @Test
  void noChannelIsOfferedWhileACodeStepUsesAConfiguredCourier() {
    codeSteps("CHOSEN", "BOTH");
    assertEquals(
        Map.of(
            "code_channels",
            List.of(),
            "reason",
            "A code step doesn't send to the voter's channel"),
        get(PROJECTION).getEntity());
    codeSteps("CHOSEN", null);
    assertEquals(List.of(), ((Map<?, ?>) get(PROJECTION).getEntity()).get("code_channels"));
    codeSteps("CHOSEN", "CHOSEN");
    assertEquals(
        Map.of("code_channels", List.of("WHATSAPP", "EMAIL")), get(PROJECTION).getEntity());
  }

  /**
   * R7-14: with the voter's token, the answer is the action's own decision for that voter: the
   * channels their Post offers, or none with the reason the action would give.
   */
  @Test
  void aSignedInVoterGetsTheActionsDecision() {
    codeSteps("CHOSEN");
    MessageSenderProvider sender = mock(MessageSenderProvider.class);
    when(sender.getChannels()).thenReturn(java.util.Set.of(MessageChannel.WHATSAPP));
    when(sender.delivers(any())).thenCallRealMethod();
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    org.keycloak.models.UserModel voter = mock(org.keycloak.models.UserModel.class);
    when(voter.getEmail()).thenReturn("voter@example.test");
    bearer = java.util.Optional.of(voter);

    when(voter.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> java.util.stream.Stream.of("election-a"));
    assertEquals(Map.of("code_channels", List.of("WHATSAPP")), get(PROJECTION).getEntity());

    // A Post that offers no channel for codes: none, with the action's reason.
    when(voter.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> java.util.stream.Stream.of("election-b"));
    assertEquals(
        Map.of("code_channels", List.of(), "reason", "Channel not offered for codes"),
        get(PROJECTION).getEntity());

    // A voter without an email isn't offered email.
    when(voter.getEmail()).thenReturn(null);
    when(voter.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> java.util.stream.Stream.of("election-c"));
    assertEquals(Map.of("code_channels", List.of("WHATSAPP")), get(PROJECTION).getEntity());
  }

  /** The voter's token needs a CORS preflight: it allows the Authorization header. */
  @Test
  void thePreflightAllowsTheVotersToken() {
    Response preflight = new MessagingChannelsResource(session, ignored -> bearer).preflight();
    assertEquals(204, preflight.getStatus());
    assertEquals("*", preflight.getHeaderString("Access-Control-Allow-Origin"));
    assertTrue(preflight.getHeaderString("Access-Control-Allow-Headers").contains("Authorization"));
    assertTrue(preflight.getHeaderString("Access-Control-Allow-Methods").contains("GET"));
  }

  @Test
  void theCodeChannelsComeInTheTicketsOrderAndNothingElse() {
    Response response =
        get(
            """
            {"version": 1, "channels": [
              {"channel": "EMAIL", "purposes": ["OTP", "NOTICE"]},
              {"channel": "VIBER", "purposes": ["NOTICE"]},
              {"channel": "WHATSAPP", "purposes": ["OTP"], "sender_label": "Synthetic"}]}
            """);
    assertEquals(200, response.getStatus());
    assertEquals(Map.of("code_channels", List.of("WHATSAPP", "EMAIL")), response.getEntity());
    assertEquals("*", response.getHeaderString("Access-Control-Allow-Origin"));
  }

  @Test
  void anEventWithoutMessagingOffersNone() {
    assertEquals(Map.of("code_channels", List.of()), get(null).getEntity());
  }
}
