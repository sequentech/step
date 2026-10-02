// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Properties;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagePurpose;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.NoticeRecipient;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.SendMessageRequest;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

class NoticeChannelTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String PHONE = "+15550123456";
  private static final String WHATSAPP = "+15550199999";
  private static final String PROJECTION =
      """
      {"version": 1, "channels": [
        {"channel": "SMS", "purposes": ["OTP", "NOTICE"]},
        {"channel": "WHATSAPP", "purposes": ["OTP", "NOTICE"]},
        {"channel": "VIBER", "purposes": ["OTP"]}]}
      """;

  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final SmsSenderProvider sms = mock(SmsSenderProvider.class);
  private final MessageSenderProvider sender = mock(MessageSenderProvider.class);
  private final Map<String, String> notes = new HashMap<>();

  @BeforeEach
  void setup() throws Exception {
    when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-synthetic-event");
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(PROJECTION);
    when(context.getEvent()).thenReturn(mock(EventBuilder.class, RETURNS_SELF));
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(keycloakContext);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    ThemeManager themes = mock(ThemeManager.class);
    Theme theme = mock(Theme.class);
    when(session.theme()).thenReturn(themes);
    when(themes.getTheme(Theme.Type.LOGIN)).thenReturn(theme);
    Properties messages = new Properties();
    messages.setProperty(Utils.SEND_PENDING_SMS_I18N_KEY, "Your enrollment is pending: {0}");
    when(theme.getEnhancedMessages(any(), any())).thenReturn(messages);
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(session.getProvider(EmailTemplateProvider.class))
        .thenReturn(mock(EmailTemplateProvider.class));
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    when(sender.getChannels()).thenReturn(Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER));
    when(sender.delivers(any())).thenCallRealMethod();
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "ACCEPTED", null));
    when(sms.send(anyString(), anyString(), anyList(), any(), any(), any())).thenReturn("sent");
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, WHATSAPP);
    notes.put(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD, PHONE);
  }

  private void enrolledWith(MessageChannel verified, MessageChannel notices) {
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, verified.name());
    notes.put(MessagingAttributes.FORM_NOTICE_CHANNEL, notices.name());
  }

  private void sendPending(Utils.MessageCourier courier) throws Exception {
    Utils.sendManualCommunication(
        session,
        realm,
        courier,
        null,
        PHONE,
        "INSUFFICIENT_INFORMATION",
        new HashMap<>(),
        NoticeRecipient.fromEnrollment(authSession, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        context);
  }

  @Test
  void aNoticeGoesToTheVerifiedMessagingAppTheVoterChose() throws Exception {
    enrolledWith(MessageChannel.WHATSAPP, MessageChannel.WHATSAPP);
    sendPending(Utils.MessageCourier.BOTH);

    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertEquals(MessageChannel.WHATSAPP, request.getValue().channel());
    assertEquals(MessagePurpose.NOTICE, request.getValue().purpose());
    assertEquals(WHATSAPP, request.getValue().destination());
    assertEquals(TENANT, request.getValue().tenantId());
    assertEquals(
        "Your enrollment is pending: INSUFFICIENT_INFORMATION",
        request.getValue().content().text());
    verifyNoInteractions(sms);
  }

  @Test
  void anOtpChoiceAloneDoesNotRouteNotices() throws Exception {
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, MessageChannel.WHATSAPP.name());
    sendPending(Utils.MessageCourier.SMS);
    verify(sender, never()).send(any());
    verify(sms)
        .send(eq(PHONE), eq(Utils.SEND_PENDING_SMS_I18N_KEY), anyList(), any(), any(), any());
  }

  @Test
  void anUnverifiedChoiceIsIgnored() throws Exception {
    enrolledWith(MessageChannel.SMS, MessageChannel.WHATSAPP);
    sendPending(Utils.MessageCourier.SMS);
    verify(sender, never()).send(any());
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
  }

  @Test
  void aChannelTheEventDoesNotUseForNoticesIsIgnored() throws Exception {
    notes.put(MessagingAttributes.FORM_VIBER_NUMBER, WHATSAPP);
    enrolledWith(MessageChannel.VIBER, MessageChannel.VIBER);
    sendPending(Utils.MessageCourier.SMS);
    verify(sender, never()).send(any());
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
  }

  @Test
  void aConfirmedFailureFallsBackToTheConfiguredCourier() throws Exception {
    enrolledWith(MessageChannel.WHATSAPP, MessageChannel.WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "rejected-422"));
    sendPending(Utils.MessageCourier.SMS);
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
  }

  @Test
  void anUnknownOutcomeIsNotResentElsewhere() throws Exception {
    enrolledWith(MessageChannel.WHATSAPP, MessageChannel.WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "UNKNOWN", "timeout"));
    sendPending(Utils.MessageCourier.SMS);
    verifyNoInteractions(sms);
  }

  @Test
  void theChosenCourierSendsOneNotice() throws Exception {
    enrolledWith(MessageChannel.SMS, MessageChannel.SMS);
    Utils.sendManualCommunication(
        session,
        realm,
        Utils.MessageCourier.CHOSEN,
        "voter@example.test",
        PHONE,
        "INSUFFICIENT_INFORMATION",
        new HashMap<>(),
        NoticeRecipient.fromEnrollment(authSession, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        context);
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
    verify(sender, never()).send(any());
  }

  @Test
  void withTheDefaultSenderNoticesKeepTheirCourier() throws Exception {
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(null);
    enrolledWith(MessageChannel.WHATSAPP, MessageChannel.WHATSAPP);
    sendPending(Utils.MessageCourier.SMS);
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
  }

  @Test
  void aSavedVotersChoiceMustBeVerified() {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(MessagingAttributes.MESSAGE_CHANNEL)).thenReturn("WHATSAPP");
    when(user.getFirstAttribute(MessagingAttributes.WHATSAPP_NUMBER)).thenReturn(WHATSAPP);
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> Stream.of("SMS"));
    when(user.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> Stream.of());
    assertEquals(
        Optional.empty(),
        NoticeRecipient.fromUser(user, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD).preferred());

    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> Stream.of("SMS", "WHATSAPP"));
    assertEquals(
        Optional.of(MessageChannel.WHATSAPP),
        NoticeRecipient.fromUser(user, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD).preferred());
  }
}
