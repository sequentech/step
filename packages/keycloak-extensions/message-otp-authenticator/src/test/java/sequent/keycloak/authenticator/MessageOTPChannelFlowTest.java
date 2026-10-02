// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.CreateMessengerLinkRequest;
import sequent.keycloak.authenticator.messaging.CreateMessengerLinkResponse;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagePurpose;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkRequest;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.SendMessageRequest;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

class MessageOTPChannelFlowTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String EVENT = "11111111-2222-3333-4444-555555555555";
  private static final String PHONE = "+15550123456";
  private static final String WHATSAPP = "+15550199999";
  private static final String EMAIL = "voter@example.test";
  private static final String PROJECTION =
      """
      {"version": 1, "channels": [
        {"channel": "EMAIL", "purposes": ["OTP"]},
        {"channel": "SMS", "purposes": ["OTP"]},
        {"channel": "WHATSAPP", "purposes": ["OTP"], "sender_label": "Synthetic Commission"},
        {"channel": "VIBER", "purposes": ["OTP"]},
        {"channel": "MESSENGER", "purposes": ["OTP"],
         "messenger_page": {"page_id": "page-1", "name": "Synthetic Page"}}]}
      """;

  private final MessageOTPAuthenticator authenticator = new MessageOTPAuthenticator();
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final UserModel user = mock(UserModel.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
  private final LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
  private final Response response = mock(Response.class);
  private final SmsSenderProvider sms = mock(SmsSenderProvider.class);
  private final MessageSenderProvider sender = mock(MessageSenderProvider.class);
  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, String> configMap = new HashMap<>();
  private final MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();
  private final List<String> verifiedChannels = new ArrayList<>();
  private final AuthenticationExecutionModel execution = new AuthenticationExecutionModel();

  @BeforeEach
  void setup() throws Exception {
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    when(rootSession.getId()).thenReturn("synthetic-session");
    when(authSession.getParentSession()).thenReturn(rootSession);
    when(authSession.getTabId()).thenReturn("synthetic-tab");
    when(authSession.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-" + EVENT);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(PROJECTION);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    doAnswer(i -> notes.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), anyString());
    doAnswer(i -> notes.remove(i.getArgument(0))).when(authSession).removeAuthNote(anyString());
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getUser()).thenReturn(user);
    when(context.getEvent()).thenReturn(event);
    when(context.form()).thenReturn(form);
    when(form.createForm(anyString())).thenReturn(response);
    when(form.createErrorPage(any())).thenReturn(response);
    HttpRequest request = mock(HttpRequest.class);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(parameters);
    execution.setRequirement(AuthenticationExecutionModel.Requirement.REQUIRED);
    when(context.getExecution()).thenReturn(execution);
    UserProfileProvider profiles = mock(UserProfileProvider.class);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profiles);
    UPConfig profileConfig = new UPConfig();
    profileConfig.setAttributes(List.of());
    when(profiles.getConfiguration()).thenReturn(profileConfig);
    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(keycloakContext);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    ThemeManager themes = mock(ThemeManager.class);
    Theme theme = mock(Theme.class);
    when(session.theme()).thenReturn(themes);
    when(themes.getTheme(Theme.Type.LOGIN)).thenReturn(theme);
    Properties messages = new Properties();
    messages.setProperty(
        Utils.SEND_CODE_MESSAGE_I18N_KEY, "Your {0} verification code is {1}, valid {2} minutes.");
    when(theme.getEnhancedMessages(any(), any())).thenReturn(messages);

    when(user.getId()).thenReturn("voter-1");
    when(user.getAttributes()).thenReturn(Map.of());
    when(user.getEmail()).thenReturn(EMAIL);
    when(user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD)).thenReturn(PHONE);
    when(user.getFirstAttribute(MessagingAttributes.WHATSAPP_NUMBER)).thenReturn(WHATSAPP);
    when(user.getFirstAttribute(MessagingAttributes.MESSENGER_ID)).thenReturn("psid-1");
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> verifiedChannels.stream());
    when(user.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> Stream.of());
    SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
    when(user.credentialManager()).thenReturn(credentials);
    when(credentials.getStoredCredentialsByTypeStream(MessageOTPCredentialModel.TYPE))
        .thenAnswer(i -> Stream.of(MessageOTPCredentialModel.create(true)));

    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(sms.send(anyString(), anyString(), anyList(), any(), any(), any())).thenReturn("sent");
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    when(sender.getChannels())
        .thenReturn(
            Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER));
    when(sender.delivers(any())).thenCallRealMethod();
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "ACCEPTED", null));

    configMap.putAll(MessageOTPAuthenticatorFactory.getConfigMap(null));
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.CHOSEN.name());
    configMap.put(Utils.TEL_USER_ATTRIBUTE, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(configMap);
    when(context.getAuthenticatorConfig()).thenReturn(config);
  }

  private void verified(MessageChannel... channels) {
    for (MessageChannel channel : channels) {
      verifiedChannels.add(channel.name());
    }
  }

  private void enrolling(MessageChannel channel) {
    configMap.put(Utils.DEFERRED_USER_ATTRIBUTE, "true");
    when(context.getUser()).thenReturn(null);
    notes.put("email", EMAIL);
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, channel.name());
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, WHATSAPP);
  }

  private void codeIssuedSecondsAgo(long seconds) {
    long ttlMillis = Long.parseLong(configMap.get(Utils.CODE_TTL)) * 1000L;
    notes.put(
        Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + ttlMillis - seconds * 1000L));
  }

  private SendMessageRequest sentRequest() {
    ArgumentCaptor<SendMessageRequest> captor = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(captor.capture());
    return captor.getValue();
  }

  @Test
  void existingCourierValuesKeepTheirMeaning() {
    assertEquals(Utils.MessageCourier.SMS, Utils.MessageCourier.fromString("SMS"));
    assertEquals(Utils.MessageCourier.EMAIL, Utils.MessageCourier.fromString("email"));
    assertEquals(Utils.MessageCourier.BOTH, Utils.MessageCourier.fromString(null));
    assertEquals(Utils.MessageCourier.NONE, Utils.MessageCourier.fromString("NONE"));
    assertEquals(Utils.MessageCourier.CHOSEN, Utils.MessageCourier.fromString("CHOSEN"));
    assertThrows(IllegalArgumentException.class, () -> Utils.MessageCourier.fromString("FAX"));
  }

  @Test
  void signInAsksWhichVerifiedContactToUseBeforeSendingAnyCode() throws Exception {
    verified(MessageChannel.EMAIL, MessageChannel.WHATSAPP);
    authenticator.authenticate(context);

    verify(form).setAttribute("otpView", "CHOOSE");
    verify(form).setAttribute("otherWayChannels", List.of("EMAIL", "WHATSAPP"));
    verify(context).challenge(response);
    verify(sender, never()).send(any());
    verifyNoInteractions(sms);
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void choosingAChannelSendsOneCodeThroughTheSender() throws Exception {
    verified(MessageChannel.EMAIL, MessageChannel.WHATSAPP);
    parameters.putSingle("channel", "WHATSAPP");
    String code;
    try (CapturedLogs utilsLogs = new CapturedLogs(Utils.class);
        CapturedLogs authenticatorLogs = new CapturedLogs(MessageOTPAuthenticator.class)) {
      authenticator.action(context);
      code = notes.get(Utils.CODE);
      assertNotNull(code);
      assertFalse(utilsLogs.text().contains(code));
      assertFalse(authenticatorLogs.text().contains(code));
    }
    SendMessageRequest request = sentRequest();
    assertEquals(TENANT, request.tenantId());
    assertEquals(EVENT, request.electionEventId());
    assertEquals("voter-1", request.voterId());
    assertEquals(MessageChannel.WHATSAPP, request.channel());
    assertEquals(MessagePurpose.OTP, request.purpose());
    assertEquals(WHATSAPP, request.destination());
    assertEquals(code, request.content().code());
    assertTrue(request.content().text().contains(code));
    assertEquals("otp:" + notes.get(MessagingAttributes.NOTE_CODE_ID), request.logicalKey());
    assertNotNull(request.expiresAt());
    verifyNoInteractions(sms);
    verify(form).setAttribute("channel", "WHATSAPP");
    verify(form).setAttribute("deliveryState", "ACCEPTED");
    verify(form).setAttribute("senderLabel", "Synthetic Commission");
    verify(form).setAttribute("address", "+155*****999");
    verify(event, never()).detail(eq("msgBody"), contains(code));
  }

  @Test
  void aSingleVerifiedContactIsUsedWithoutAsking() throws Exception {
    verified(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);
    assertEquals(MessageChannel.WHATSAPP, sentRequest().channel());
  }

  @Test
  void anUnverifiedContactCannotBeChosen() throws Exception {
    verified(MessageChannel.EMAIL, MessageChannel.WHATSAPP);
    parameters.putSingle("channel", "VIBER");
    authenticator.action(context);
    verify(sender, never()).send(any());
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void switchingChannelDoesNotBypassTheResendTimer() throws Exception {
    verified(MessageChannel.SMS, MessageChannel.WHATSAPP);
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, "WHATSAPP");
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    parameters.putSingle("channel", "SMS");
    authenticator.action(context);

    verifyNoInteractions(sms);
    verify(sender, never()).send(any());
    assertEquals("482619", notes.get(Utils.CODE));
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.FORM_OTP_CHANNEL));
  }

  @Test
  void switchingChannelAfterTheTimerReplacesTheCodeAndTheMessengerReference() throws Exception {
    verified(MessageChannel.SMS, MessageChannel.MESSENGER);
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, "MESSENGER");
    notes.put(Utils.CODE, "482619");
    notes.put(MessagingAttributes.NOTE_CODE_ID, "previous-code");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-old");
    notes.put(MessagingAttributes.NOTE_MESSENGER_LINK, "https://m.me/synthetic?ref=ref-old");
    notes.put(MessagingAttributes.NOTE_MESSENGER_WORD, "MAPLE");
    codeIssuedSecondsAgo(61);
    parameters.putSingle("channel", "SMS");
    authenticator.action(context);

    verify(sms).send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    assertNotEquals("482619", notes.get(Utils.CODE));
    assertNotEquals("previous-code", notes.get(MessagingAttributes.NOTE_CODE_ID));
    assertEquals("SMS", notes.get(MessagingAttributes.FORM_OTP_CHANNEL));
    assertNull(notes.get(MessagingAttributes.NOTE_MESSENGER_REFERENCE));
    assertNull(notes.get(MessagingAttributes.NOTE_MESSENGER_LINK));
    assertNull(notes.get(MessagingAttributes.NOTE_MESSENGER_WORD));
  }

  @Test
  void switchingChannelCannotRefreshAnExhaustedCodeEarly() throws Exception {
    verified(MessageChannel.SMS, MessageChannel.WHATSAPP);
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "1");
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, "WHATSAPP");
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));

    parameters.clear();
    parameters.putSingle("channel", "SMS");
    authenticator.action(context);
    verifyNoInteractions(sms);
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void anUnconfirmedDeliveryIsNeitherSentNorFailed() throws Exception {
    verified(MessageChannel.EMAIL, MessageChannel.WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "UNKNOWN", "timeout"));
    parameters.putSingle("channel", "WHATSAPP");
    authenticator.action(context);

    verify(form).setAttribute("deliveryState", "UNKNOWN");
    verify(form).setAttribute("otherWayChannels", List.of("EMAIL"));
    assertEquals(
        MessageAttemptState.UNKNOWN.name(), notes.get(MessagingAttributes.NOTE_DELIVERY_STATE));
  }

  @Test
  void aCorrectCodeRecordsTheVerifiedChannel() throws Exception {
    enrolling(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);
    assertEquals(WHATSAPP, sentRequest().destination());

    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertNull(notes.get("Email verified"));
  }

  @Test
  void messengerCodesAreHandedToHarvestThroughAOneTimeLink() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", "MAPLE", "2026-10-02T10:05:00Z"));
    authenticator.authenticate(context);

    ArgumentCaptor<CreateMessengerLinkRequest> captor =
        ArgumentCaptor.forClass(CreateMessengerLinkRequest.class);
    verify(sender).createMessengerLink(captor.capture());
    CreateMessengerLinkRequest link = captor.getValue();
    String code = notes.get(Utils.CODE);
    assertEquals(code, link.code());
    assertFalse(link.challenge().contains(code));
    assertFalse(link.authSession().contains("synthetic-session"));
    assertEquals(TENANT, link.tenantId());
    verify(sender, never()).send(any());
    verify(sender, never()).confirmMessengerLink(any());
    verify(form).setAttribute("messengerLink", "https://m.me/synthetic?ref=ref-1");
    verify(form).setAttribute("messengerWord", "MAPLE");
    verify(form).setAttribute("messengerPage", "Synthetic Page");
    assertEquals("ref-1", notes.get(MessagingAttributes.NOTE_MESSENGER_REFERENCE));

    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-9", "page-1"));
    parameters.putSingle(Utils.CODE, code);
    authenticator.action(context);

    ArgumentCaptor<MessengerLinkRequest> confirm =
        ArgumentCaptor.forClass(MessengerLinkRequest.class);
    verify(sender).confirmMessengerLink(confirm.capture());
    assertEquals("ref-1", confirm.getValue().reference());
    assertEquals(link.authSession(), confirm.getValue().authSession());
    assertEquals(link.challenge(), confirm.getValue().challenge());
    verify(context).success();
    assertEquals("MESSENGER", notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertEquals("psid-9", notes.get(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID));
    assertEquals("page-1", notes.get(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE));
  }

  @Test
  void aReplacedMessengerReferenceDoesNotVerify() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    notes.put(MessagingAttributes.NOTE_CODE_ID, "code-1");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("REPLACED", null, null));
    parameters.putSingle(Utils.CODE, "482619");
    authenticator.action(context);

    verify(context, never()).success();
    verify(context).failureChallenge(eq(AuthenticationFlowError.INVALID_CREDENTIALS), any());
    assertNull(notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertNull(notes.get(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID));
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void messengerWithoutAReferenceDoesNotVerify() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, "482619");
    authenticator.action(context);
    verify(context, never()).success();
    verify(sender, never()).confirmMessengerLink(any());
  }

  @Test
  void signingInWithMessengerRequiresTheVotersOwnAccount() throws Exception {
    verified(MessageChannel.MESSENGER);
    when(user.getFirstAttribute(MessagingAttributes.MESSENGER_PAGE)).thenReturn("page-1");
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, "MESSENGER");
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    notes.put(MessagingAttributes.NOTE_CODE_ID, "code-1");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-someone-else", "page-1"));
    parameters.putSingle(Utils.CODE, "482619");
    authenticator.action(context);
    verify(context, never()).success();

    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-1", "page-1"));
    notes.put(Utils.CODE, "482619");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
    authenticator.action(context);
    verify(context).success();
  }

  @Test
  void checkingMessengerShowsTheLinkState() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    notes.put(MessagingAttributes.NOTE_CODE_ID, "code-1");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
    when(sender.messengerLinkStatus(any()))
        .thenReturn(new MessengerLinkStatus("CODE_SENT", null, null));
    parameters.putSingle("messengerStatus", "true");
    authenticator.action(context);

    verify(sender).messengerLinkStatus(any());
    verify(form).setAttribute("messengerState", "CODE_SENT");
    verify(sender, never()).createMessengerLink(any());
    assertEquals("482619", notes.get(Utils.CODE));
  }
}
