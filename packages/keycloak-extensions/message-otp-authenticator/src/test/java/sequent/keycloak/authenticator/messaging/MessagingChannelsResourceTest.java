// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.*;
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

  private Response get(String projection) {
    KeycloakContext context = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(projection);
    return new MessagingChannelsResource(session).channels();
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
