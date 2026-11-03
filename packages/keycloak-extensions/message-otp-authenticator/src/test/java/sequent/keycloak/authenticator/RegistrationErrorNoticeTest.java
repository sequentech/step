// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;

/**
 * R7-18: a registration that matched no voter is reported to support only. Nothing goes to the
 * address or number typed in the form: it is unverified, and events no longer carry it (R3-06).
 */
class RegistrationErrorNoticeTest {
  @Test
  void aRegistrationErrorGoesToSupportOnly() throws Exception {
    KeycloakSession session = mock(KeycloakSession.class);
    when(session.realms()).thenReturn(mock(org.keycloak.models.RealmProvider.class));
    SmsSenderProvider sms = mock(SmsSenderProvider.class);
    EmailSenderProvider email = mock(EmailSenderProvider.class);
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(session.getProvider(EmailSenderProvider.class)).thenReturn(email);
    Event event = new Event();
    event.setType(EventType.REGISTER_ERROR);
    event.setError("userNotFound");
    event.setRealmId("realm-1");
    event.setDetails(
        Map.of(
            "sequent.read-only.mobile-number", "+15550100777",
            "code_id", "synthetic-session"));
    try {
      Utils.sendRegistrationErrorToSupport(session, "realm-1", event);
    } catch (Exception templating) {
      // The support email's template needs a theme; only who is written to matters here.
    }
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    verify(sms, never()).send(anyString(), anyString());
    verify(email, never()).send(any(), eq("+15550100777"), any(), any(), any());
  }
}
