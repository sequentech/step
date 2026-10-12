// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;

/**
 * R7-15: the sign-in link goes only by email, so only to a voter whose one channel is email, or who
 * has no messaging channel (the realm's rule for them). Any other voter's link is refused with the
 * reason, never sent to an email that isn't their channel.
 */
class SmartLinkChannelTest {
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserModel user = mock(UserModel.class);
  private final EmailTemplateProvider email = mock(EmailTemplateProvider.class, RETURNS_SELF);
  private final Map<String, List<String>> attributes = new HashMap<>();

  @BeforeEach
  void setup() {
    KeycloakContext context = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("synthetic");
    when(session.getProvider(EmailTemplateProvider.class)).thenReturn(email);
    when(user.getEmail()).thenReturn("voter@example.test");
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), List.of()).stream());
  }

  @Test
  void aWhatsAppVotersLinkIsRefusedAndNotEmailed() throws Exception {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of("+15550100777"));
    assertEquals(Optional.of(MessageReason.SMART_LINK_ONLY_BY_EMAIL), SmartLink.refusal(user));
    assertFalse(SmartLink.sendSmartLinkNotification(session, user, "https://auth.example/x"));
    verify(email, never()).send(anyString(), anyList(), anyString(), anyMap());
  }

  @Test
  void anEmailVoterAndAVoterWithoutAMessagingChannelGetTheLinkByEmail() throws Exception {
    assertEquals(Optional.empty(), SmartLink.refusal(user));
    assertTrue(SmartLink.sendSmartLinkNotification(session, user, "https://auth.example/x"));

    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("EMAIL"));
    assertEquals(Optional.empty(), SmartLink.refusal(user));
    assertTrue(SmartLink.sendSmartLinkNotification(session, user, "https://auth.example/x"));
    verify(email, times(2)).send(eq("SmartLinkSubject"), anyList(), anyString(), anyMap());
  }

  @Test
  void aVoterWithoutAnEmailIsRefused() throws Exception {
    when(user.getEmail()).thenReturn(null);
    assertEquals(Optional.of(MessageReason.NO_CONTACT_CHANNEL), SmartLink.refusal(user));
    assertFalse(SmartLink.sendSmartLinkNotification(session, user, "https://auth.example/x"));
    verify(email, never()).send(anyString(), anyList(), anyString(), anyMap());
  }

  @Test
  void theResponseCarriesTheReason() {
    SmartLinkResponse response = new SmartLinkResponse();
    response.setReason(MessageReason.SMART_LINK_ONLY_BY_EMAIL.reason());
    assertEquals("The sign-in link is sent only by email", response.getReason());
  }
}
