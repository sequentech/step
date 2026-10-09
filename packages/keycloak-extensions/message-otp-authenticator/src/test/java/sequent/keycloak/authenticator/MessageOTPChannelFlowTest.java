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
  private final Map<String, List<String>> attributes = new HashMap<>();
  private final AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
  private String email = EMAIL;

  @BeforeEach
  void setup() throws Exception {
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    when(rootSession.getId()).thenReturn("synthetic-session");
    when(authSession.getParentSession()).thenReturn(rootSession);
    when(authSession.getTabId()).thenReturn("synthetic-tab");
    when(authSession.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-" + EVENT);
    when(realm.getDisplayName()).thenReturn("Synthetic Commission");
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
    when(user.getEmail()).thenAnswer(i -> email);
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), List.of()).stream());
    when(user.getFirstAttribute(anyString()))
        .thenAnswer(
            i -> {
              List<String> values = attributes.get(i.<String>getArgument(0));
              return values == null || values.isEmpty() ? null : values.get(0);
            });
    attributes.put(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD, List.of(PHONE));
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

  /** A voter whose one channel is recorded with the ticket's attributes. */
  private void recorded(MessageChannel channel, String number) {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of(channel.name()));
    if (number != null) {
      attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of(number));
    }
  }

  private void recordedMessenger(String psid, String page) {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("MESSENGER"));
    attributes.put(MessagingAttributes.MESSENGER_ID, List.of(psid));
    attributes.put(MessagingAttributes.MESSENGER_PAGE, List.of(page));
  }

  private void enrolling(MessageChannel channel) {
    configMap.put(Utils.DEFERRED_USER_ATTRIBUTE, "true");
    when(context.getUser()).thenReturn(null);
    notes.put("email", EMAIL);
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, channel.name());
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, WHATSAPP);
  }

  /** A live code on the channel, sent a second ago. */
  private void liveCode(MessageChannel channel, String code) {
    notes.put(Utils.CODE, code);
    notes.put(MessagingAttributes.NOTE_CODE_CHANNEL, channel.name());
    codeIssuedSecondsAgo(1);
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
  void signInSendsTheCodeOnlyToTheVotersChannel() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    String code;
    try (CapturedLogs utilsLogs = new CapturedLogs(Utils.class);
        CapturedLogs authenticatorLogs = new CapturedLogs(MessageOTPAuthenticator.class)) {
      authenticator.authenticate(context);
      code = notes.get(Utils.CODE);
      assertNotNull(code);
      for (String text : List.of(utilsLogs.text(), authenticatorLogs.text())) {
        assertFalse(text.contains(code));
        assertFalse(text.contains(WHATSAPP));
        assertFalse(text.contains(EMAIL));
        assertFalse(text.contains(PHONE));
      }
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
    verify(form).setAttribute("address", "+1 • ••• ••• 999");
    verify(form, never()).setAttribute(eq("otpView"), any());
    verify(form, never()).setAttribute(eq("otherWayChannels"), any());
    verify(form, never()).setAttribute(eq("channelAddresses"), any());
    verify(event, never()).detail(eq("msgBody"), contains(code));
  }

  @Test
  void aFirstReleaseRecordUsesItsVerifiedNoticeChannel() throws Exception {
    attributes.put(MessagingAttributes.VERIFIED_CHANNELS, List.of("EMAIL", "WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.WHATSAPP_NUMBER, List.of(WHATSAPP));
    authenticator.authenticate(context);
    assertEquals(MessageChannel.WHATSAPP, sentRequest().channel());
    assertEquals(WHATSAPP, sentRequest().destination());
  }

  @Test
  void aVoterWithoutAMessagingChannelGetsTheCodeByEmailElseSms() throws Exception {
    email = null;
    authenticator.authenticate(context);
    verify(sms).send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    verify(sender, never()).send(any());
    assertEquals("SMS", notes.get(MessagingAttributes.NOTE_CODE_CHANNEL));
  }

  @Test
  void anSmsVoterGetsTheCodeOnTheirRecordedNumber() throws Exception {
    recorded(MessageChannel.SMS, "+639171234567");
    authenticator.authenticate(context);
    verify(sms)
        .send(
            eq("+639171234567"), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    verify(sms, never()).send(eq(PHONE), anyString(), anyList(), any(), any(), any());
  }

  @Test
  void aChannelParameterCannotChooseAnotherChannel() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);
    codeIssuedSecondsAgo(61);
    parameters.putSingle("channel", "SMS");
    authenticator.action(context);

    verifyNoInteractions(sms);
    verify(sender, times(1)).send(any());
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.NOTE_CODE_CHANNEL));
  }

  @Test
  void aChannelTheEventNoLongerOffersIsRefusedWithItsReasonAndNothingElseIsTried()
      throws Exception {
    recorded(MessageChannel.VIBER, WHATSAPP);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE))
        .thenReturn(
            """
            {"version": 1, "channels": [
              {"channel": "EMAIL", "purposes": ["OTP"]},
              {"channel": "SMS", "purposes": ["OTP"]},
              {"channel": "VIBER", "purposes": ["NOTICE"]}]}
            """);
    authenticator.authenticate(context);

    verify(sender, never()).send(any());
    verifyNoInteractions(sms);
    verify(form).setError(eq("messageOtp.auth.channelNotOffered"), any());
    verify(form).createErrorPage(Response.Status.BAD_REQUEST);
    verify(event).detail("reason", "Channel not offered for codes");
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void aChannelNothingDeliversIsRefused() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(null);
    authenticator.authenticate(context);
    verifyNoInteractions(sms);
    verify(form).setError(eq("messageOtp.auth.channelUnavailable"), any());
  }

  @Test
  void aVoterWithNoContactAtAllIsRefused() throws Exception {
    email = null;
    attributes.clear();
    authenticator.authenticate(context);
    verify(form).setError(eq("messageOtp.auth.noChannel"), any());
    verifyNoInteractions(sms);
    verify(sender, never()).send(any());
  }

  @Test
  void anUnconfirmedDeliveryIsNeitherSentNorFailed() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "UNKNOWN", "timeout"));
    authenticator.authenticate(context);

    verify(form).setAttribute("deliveryState", "UNKNOWN");
    assertEquals(
        MessageAttemptState.UNKNOWN.name(), notes.get(MessagingAttributes.NOTE_DELIVERY_STATE));
  }

  @Test
  void aCorrectCodeRecordsTheVerifiedChannelAndDestination() throws Exception {
    enrolling(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);
    assertEquals(WHATSAPP, sentRequest().destination());

    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertEquals(WHATSAPP, notes.get(MessagingAttributes.NOTE_VERIFIED_DESTINATION));
    assertNull(notes.get("Email verified"));
  }

  @Test
  void aCodeProvesOnlyTheContactItWasSentTo() throws Exception {
    enrolling(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);
    notes.put(MessagingAttributes.FORM_OTP_CHANNEL, "VIBER");
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+15550100000");

    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertEquals(WHATSAPP, notes.get(MessagingAttributes.NOTE_VERIFIED_DESTINATION));
  }

  @Test
  void messengerCodesAreHandedToHarvestThroughAOneTimeLinkWithAQrCode() throws Exception {
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
    verify(form)
        .setAttribute(
            eq("messengerQr"), argThat((String qr) -> qr.startsWith("data:image/svg+xml;base64,")));
    verify(form).setAttribute("messengerPage", "Synthetic Page");
    verify(form).setAttribute("messagingOrganization", "Synthetic Commission");
    verify(form, never()).setAttribute(eq("messengerWord"), any());
    assertNull(notes.get(MessagingAttributes.NOTE_MESSENGER_WORD));
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
  void aReplacedMessengerReferenceDoesNotVerifyAndIsNeverAServerError() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    liveCode(MessageChannel.MESSENGER, "482619");
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

    // Entering the code again asks for a new one rather than failing.
    authenticator.action(context);
    verify(context, never()).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    verify(form).setError(MessageOTPAuthenticator.NO_CODE_KEY);
  }

  @Test
  void messengerWithoutAReferenceDoesNotVerify() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    liveCode(MessageChannel.MESSENGER, "482619");
    parameters.putSingle(Utils.CODE, "482619");
    authenticator.action(context);
    verify(context, never()).success();
    verify(sender, never()).confirmMessengerLink(any());
  }

  @Test
  void signingInWithMessengerRequiresTheVotersOwnAccountOnTheirPage() throws Exception {
    recordedMessenger("psid-1", "page-1");
    for (MessengerLinkStatus wrong :
        List.of(
            new MessengerLinkStatus("CONFIRMED", "psid-someone-else", "page-1"),
            new MessengerLinkStatus("CONFIRMED", "psid-1", "page-2"),
            new MessengerLinkStatus("CONFIRMED", "psid-1", null))) {
      liveCode(MessageChannel.MESSENGER, "482619");
      notes.put(MessagingAttributes.NOTE_CODE_ID, "code-1");
      notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
      when(sender.confirmMessengerLink(any())).thenReturn(wrong);
      parameters.putSingle(Utils.CODE, "482619");
      authenticator.action(context);
      verify(context, never()).success();
    }

    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-1", "page-1"));
    liveCode(MessageChannel.MESSENGER, "482619");
    notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
    authenticator.action(context);
    verify(context).success();
  }

  @Test
  void checkingMessengerShowsTheLinkState() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    liveCode(MessageChannel.MESSENGER, "482619");
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

  @Test
  void aCodeNamesItsTemplateAndTheVotersLanguage() throws Exception {
    when(session.getContext().resolveLocale(any())).thenReturn(Locale.forLanguageTag("tl"));
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);

    SendMessageRequest request = sentRequest();
    assertEquals("otp", request.templateKey());
    assertEquals("tl", request.language());
  }

  @Test
  void aOneTimeLinkNamesItsOwnTemplate() {
    assertEquals("otp", Utils.codeTemplateKey(false));
    assertEquals("otl", Utils.codeTemplateKey(true));
  }

  @Test
  void aLanguageIsAlwaysSent() throws Exception {
    when(session.getContext().resolveLocale(any())).thenReturn(null);
    when(realm.getDefaultLocale()).thenReturn("es");
    assertEquals("es", Utils.languageCode(session, realm, user));

    when(realm.getDefaultLocale()).thenReturn(null);
    assertEquals("en", Utils.languageCode(session, realm, user));
  }

  @Test
  void theMessengerLinkCarriesTheVotersLanguage() throws Exception {
    when(session.getContext().resolveLocale(any())).thenReturn(Locale.forLanguageTag("tl"));
    enrolling(MessageChannel.MESSENGER);
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-02T10:05:00Z"));
    authenticator.authenticate(context);

    ArgumentCaptor<CreateMessengerLinkRequest> captor =
        ArgumentCaptor.forClass(CreateMessengerLinkRequest.class);
    verify(sender).createMessengerLink(captor.capture());
    assertEquals("tl", captor.getValue().language());
  }

  /** KC-R4: a failed send (a 4xx such as 429, or unreachable) starts the resend timer. */
  @Test
  void aFailedSendStartsTheResendTimer() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "rejected-429"));
    authenticator.authenticate(context);

    verify(form).setAttribute("deliveryState", "FAILED");
    verify(form).setAttribute("codeJustSent", false);
    verify(form).setAttribute("resendTimerStarted", true);
    assertNull(notes.get(Utils.CODE));

    when(sender.send(any())).thenReturn(new SendMessageResponse("m-2", "ACCEPTED", null));
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sender, times(1)).send(any());
    assertNull(notes.get(Utils.CODE));

    codeIssuedSecondsAgo(61);
    authenticator.action(context);
    verify(sender, times(2)).send(any());
    assertNotNull(notes.get(Utils.CODE));
  }

  @Test
  void aFailedSendDoesNotRepeatItselfWhenThePageIsShownAgain() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "unreachable"));
    authenticator.authenticate(context);
    authenticator.authenticate(context);
    verify(sender, times(1)).send(any());
  }

  @Test
  void aFailedSendLeavesNoCodeToGuess() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "unreachable"));
    authenticator.authenticate(context);

    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    verify(context, never()).success();
    verify(context).failureChallenge(eq(AuthenticationFlowError.INVALID_CREDENTIALS), any());
  }

  @Test
  void anUnconfirmedSendStartsTheResendTimer() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "UNKNOWN", "timeout"));
    authenticator.authenticate(context);
    verify(form).setAttribute("codeJustSent", true);
    String code = notes.get(Utils.CODE);

    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sender, times(1)).send(any());
    assertEquals(code, notes.get(Utils.CODE));
  }

  @Test
  void attemptsAreCountedForTheSessionAcrossCodesOnTheChannel() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "wrong");
      authenticator.action(context);
    }
    codeIssuedSecondsAgo(61);
    parameters.clear();
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sender, times(2)).send(any());

    parameters.clear();
    parameters.putSingle(Utils.CODE, "wrong");
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));
    verify(form, atLeastOnce()).setAttribute("attemptsExhausted", true);

    codeIssuedSecondsAgo(61);
    parameters.clear();
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sender, times(2)).send(any());
  }
}
