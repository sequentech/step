// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Properties;
import java.util.Set;
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
    notes.put(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD, PHONE);
  }

  private void enrolledWith(MessageChannel verified, String destination) {
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, verified.name());
    notes.put(MessagingAttributes.NOTE_VERIFIED_DESTINATION, destination);
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
  void aNoticeGoesToTheVerifiedChannelOnly() throws Exception {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    enrolledWith(MessageChannel.WHATSAPP, WHATSAPP);
    sendPending(Utils.MessageCourier.BOTH);
    // R2-07: harvest logs the attempt; Keycloak adds no second entry.
    verify(event, never()).detail(eq("msgBody"), anyString());

    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertEquals(MessageChannel.WHATSAPP, request.getValue().channel());
    assertEquals(MessagePurpose.NOTICE, request.getValue().purpose());
    assertEquals(WHATSAPP, request.getValue().destination());
    assertEquals(TENANT, request.getValue().tenantId());
    assertEquals(
        "Your enrollment is pending: INSUFFICIENT_INFORMATION",
        request.getValue().content().text());
    assertEquals("messagePendingSms", request.getValue().templateKey());
    assertEquals("en", request.getValue().language());
    verifyNoInteractions(sms);
  }

  @Test
  void aFormFieldCannotChooseWhereNoticesGo() throws Exception {
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, MessageChannel.WHATSAPP.name());
    sendPending(Utils.MessageCourier.CHOSEN);
    verify(sender, never()).send(any());
    verify(sms)
        .send(eq(PHONE), eq(Utils.SEND_PENDING_SMS_I18N_KEY), anyList(), any(), any(), any());
  }

  @Test
  void everyNoticeKeepsTheMessageKeyTheAdminPortalBindsTemplatesTo() {
    assertEquals("messageSuccessSms", Utils.SEND_SUCCESS_SMS_I18N_KEY);
    assertEquals("messageSuccessSmsKiosk", Utils.SEND_SUCCESS_SMS_I18N_KEY_KIOSK);
    assertEquals("messagePendingSms", Utils.SEND_PENDING_SMS_I18N_KEY);
    assertEquals("messageRejectedSms", Utils.SEND_REJECT_SMS_I18N_KEY);
    assertEquals(
        "forgotPassword.sms.text",
        sequent.keycloak.authenticator.forgot_password.ResetCredentialNotification
            .RESET_LINK_MESSAGE_KEY);
    assertEquals(
        "newPassword.message.text",
        sequent.keycloak.authenticator.forgot_password.Utils.NEW_PASSWORD_MESSAGE_KEY);
  }

  /** KC-R6: the reset page names neither the channel nor whether the account exists. */
  @Test
  void theResetPageIsTheSameForEveryone() {
    org.keycloak.models.utils.FormMessage message =
        sequent.keycloak.authenticator.forgot_password.ResetCredentialNotification.successMessage();
    assertEquals("forgotPassword.success.contact.message", message.getMessage());
    assertTrue(message.getParameters() == null || message.getParameters().length == 0);
  }

  @Test
  void aConfirmedFailureIsRecordedAndNeverSentElsewhere() throws Exception {
    enrolledWith(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "rejected-422"));
    sendPending(Utils.MessageCourier.SMS);
    verify(sender).send(any());
    verifyNoInteractions(sms);
  }

  @Test
  void anUnknownOutcomeIsNotResentElsewhere() throws Exception {
    enrolledWith(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "UNKNOWN", "timeout"));
    sendPending(Utils.MessageCourier.SMS);
    verifyNoInteractions(sms);
  }

  /**
   * R2-22 and the fix-round decision: Keycloak always hands a notice on the voter's channel to
   * harvest, which refuses a channel the event doesn't use for notices with its one reason (and a
   * ledger row). Keycloak never decides that itself, and never sends it elsewhere.
   */
  @Test
  void aChannelTheEventDoesNotUseForNoticesIsRefusedByHarvestNotByKeycloak() throws Exception {
    enrolledWith(MessageChannel.VIBER, WHATSAPP);
    when(sender.send(any()))
        .thenReturn(new SendMessageResponse("m-9", "FAILED", "Channel not offered for notices"));
    sendPending(Utils.MessageCourier.SMS);
    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertEquals(MessageChannel.VIBER, request.getValue().channel());
    assertEquals(MessagePurpose.NOTICE, request.getValue().purpose());
    verifyNoInteractions(sms);
  }

  /**
   * R2-22: a voter with no channel at all is also handed to harvest (no channel, no destination),
   * which records the notice as FAILED with "No contact channel".
   */
  @Test
  void aVoterWithNoChannelIsRecordedByHarvest() throws Exception {
    notes.remove(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    when(sender.send(any()))
        .thenReturn(new SendMessageResponse("m-10", "FAILED", "No contact channel"));
    sendPending(Utils.MessageCourier.CHOSEN);
    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertNull(request.getValue().channel());
    assertNull(request.getValue().destination());
    assertEquals(MessagePurpose.NOTICE, request.getValue().purpose());
    verifyNoInteractions(sms);
  }

  /** R2-23: a notice sent by Keycloak's own SMS provider is logged without its content. */
  @Test
  void aKeycloakSentNoticeIsLoggedWithoutItsContent() throws Exception {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    when(sms.send(anyString(), anyString(), anyList(), any(), any(), any()))
        .thenReturn("Your enrollment is pending: INSUFFICIENT_INFORMATION " + PHONE);
    sendPending(Utils.MessageCourier.CHOSEN);
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
    ArgumentCaptor<String> body = ArgumentCaptor.forClass(String.class);
    verify(event).detail(eq("msgBody"), body.capture());
    assertFalse(body.getValue().contains("pending"));
    assertFalse(body.getValue().contains(PHONE));
    assertTrue(body.getValue().contains("\"channel\":\"SMS\""));
  }

  @Test
  void withoutAMessageSenderAnAppVoterGetsNothingElsewhere() throws Exception {
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(null);
    enrolledWith(MessageChannel.WHATSAPP, WHATSAPP);
    sendPending(Utils.MessageCourier.SMS);
    verifyNoInteractions(sms);
  }

  @Test
  void theChosenCourierSendsOneNoticeOnTheVotersSmsNumber() throws Exception {
    enrolledWith(MessageChannel.SMS, "+639171234567");
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
    verify(sms, times(1)).send(eq("+639171234567"), anyString(), anyList(), any(), any(), any());
    verify(sender, never()).send(any());
  }

  @Test
  void aRealmWithoutMessagingKeepsItsConfiguredCourier() throws Exception {
    sendPending(Utils.MessageCourier.BOTH);
    verify(sms).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
    verify(sender, never()).send(any());
  }

  @Test
  void aSavedVotersNoticesFollowTheirOneChannel() {
    UserModel user = mock(UserModel.class);
    Map<String, java.util.List<String>> attributes = new HashMap<>();
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), java.util.List.of()).stream());
    when(user.getEmail()).thenReturn("voter@example.test");
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, java.util.List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, java.util.List.of(WHATSAPP));
    NoticeRecipient recipient =
        NoticeRecipient.fromUser(user, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    assertEquals(MessageChannel.WHATSAPP, recipient.channel().get().channel());
    assertEquals(WHATSAPP, recipient.channel().get().destination());

    attributes.clear();
    assertEquals(
        Optional.of(MessageChannel.EMAIL),
        NoticeRecipient.fromUser(user, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD)
            .channel()
            .map(c -> c.channel()));
  }

  /** U8: a temporary password goes to the voter's channel, and never by email as well. */
  @Test
  void aTemporaryPasswordGoesOnlyToTheVotersChannel() throws Exception {
    when(session.getContext().getRealm()).thenReturn(realm);
    UserModel user = mock(UserModel.class);
    Map<String, java.util.List<String>> attributes = new HashMap<>();
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, java.util.List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, java.util.List.of(WHATSAPP));
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), java.util.List.of()).stream());
    when(user.getEmail()).thenReturn("voter@example.test");
    EmailTemplateProvider email = mock(EmailTemplateProvider.class, RETURNS_SELF);
    when(session.getProvider(EmailTemplateProvider.class)).thenReturn(email);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(PROJECTION);

    sequent.keycloak.authenticator.forgot_password.Utils.sendNewPasswordNotification(
        session, user, "synthetic-temporary");

    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertEquals(MessageChannel.WHATSAPP, request.getValue().channel());
    assertEquals("newPassword.message.text", request.getValue().templateKey());
    verify(email, never()).send(anyString(), anyList(), anyString(), any());
    verifyNoInteractions(sms);
  }

  /**
   * R7-01: a saved voter whose recorded channel has no destination gets no notice by the realm's
   * configured courier: it goes to harvest without a channel, which records "No contact channel".
   */
  @Test
  void aRecordedVoterWithoutADestinationGetsNothingByTheConfiguredCourier() throws Exception {
    UserModel user = mock(UserModel.class);
    Map<String, java.util.List<String>> attributes = new HashMap<>();
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, java.util.List.of("WHATSAPP"));
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), java.util.List.of()).stream());
    when(user.getEmail()).thenReturn("voter@example.test");
    when(user.getId()).thenReturn("voter-1");
    Utils.sendManualCommunication(
        session,
        realm,
        Utils.MessageCourier.BOTH,
        null,
        PHONE,
        "INSUFFICIENT_INFORMATION",
        new HashMap<>(),
        NoticeRecipient.fromUser(user, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        context);

    verifyNoInteractions(sms);
    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    assertNull(request.getValue().channel());
    assertNull(request.getValue().destination());
  }
}
