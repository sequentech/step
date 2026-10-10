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
    // A sign-in step after the password: the realm names the channel there.
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, Utils.CodePageDisclosure.NAMED.name());
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
    // A form that asks for the channel records what it offered.
    notes.put(MessagingAttributes.NOTE_OFFERED_CHANNELS, channel.name());
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
    verify(form).setAttribute("address", "+1 ••• ••• 999");
    verify(form, never()).setAttribute(eq("otpView"), any());
    verify(form, never()).setAttribute(eq("otherWayChannels"), any());
    verify(form, never()).setAttribute(eq("channelAddresses"), any());
    verify(event, never()).detail(eq("msgBody"), contains(code));
    // R2-07: harvest writes the one electoral-log entry of a send it makes; Keycloak none.
    verify(event, never()).detail(eq("msgBody"), anyString());
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

  /**
   * R3-03: attempts are counted per challenge (per code step). A second message OTP step of the
   * same flow starts with its own count; a resend in either never resets it.
   */
  @Test
  void attemptsAreCountedPerCodeStep() throws Exception {
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    execution.setId("email-step");
    authenticator.authenticate(context);
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "wrong");
      authenticator.action(context);
    }
    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();

    // The next code step of the flow.
    execution.setId("sms-step");
    authenticator.authenticate(context);
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "wrong");
      authenticator.action(context);
    }
    verify(form, never()).setAttribute("attemptsExhausted", true);
    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context, times(2)).success();
  }

  /**
   * R3-04: with the NEUTRAL disclosure policy (the reset flow), the code page tells nothing about
   * the account: no channel, no masked destination, no sender, no Messenger link.
   */
  @Test
  void aNeutralCodePageNamesNoChannelAndNoDestination() throws Exception {
    configMap.put("code-page-disclosure", "NEUTRAL");
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);

    verify(sender).send(any());
    verify(form).setAttribute("codeDisclosure", "NEUTRAL");
    verify(form, never()).setAttribute(eq("channel"), any());
    verify(form, never()).setAttribute(eq("address"), any());
    verify(form, never()).setAttribute(eq("senderLabel"), any());
    verify(form, never()).setAttribute(eq("deliveryState"), any());
    verify(form, never()).setAttribute(eq("messengerState"), any());
    // The event offers Messenger for codes: the box is there, with a link that binds to nothing.
    verify(form).setAttribute("messengerConnect", true);
    verify(form)
        .setAttribute(
            eq("messengerLink"),
            argThat((String link) -> link.matches("https://m\\.me/page-1\\?ref=[0-9a-f]{64}")));
    verify(sender, never()).createMessengerLink(any());
  }

  private java.util.Set<String> pageAttributes() {
    ArgumentCaptor<String> names = ArgumentCaptor.forClass(String.class);
    verify(form, atLeastOnce()).setAttribute(names.capture(), any());
    return new java.util.TreeSet<>(names.getAllValues());
  }

  /** Lead decision: a neutral page is the same for an account, any channel, and no account. */
  @Test
  void aNeutralPageIsTheSameWithAndWithoutAnAccount() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);
    java.util.Set<String> withAccount = pageAttributes();

    clearInvocations(form);
    notes.clear();
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);
    assertEquals(withAccount, pageAttributes());
    assertTrue(withAccount.contains("messengerLink"));
  }

  /** A Messenger voter's neutral page carries the live link, so they can reset their password. */
  @Test
  void aMessengerVotersNeutralPageCarriesTheLiveLink() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    recordedMessenger("psid-7", "page-1");
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-02T10:05:00Z"));
    authenticator.authenticate(context);
    verify(form).setAttribute("messengerLink", "https://m.me/synthetic?ref=ref-1");
    verify(form).setAttribute("messengerConnect", true);
    verify(form, never()).setAttribute(eq("channel"), any());
    verify(form, never()).setAttribute(eq("messengerState"), any());
    // Only the voter's own account may get the code.
    ArgumentCaptor<CreateMessengerLinkRequest> request =
        ArgumentCaptor.forClass(CreateMessengerLinkRequest.class);
    verify(sender).createMessengerLink(request.capture());
    assertEquals("psid-7", request.getValue().expectedRecipient());
  }

  @Test
  void anEnrollingVotersLinkHasNoExpectedRecipient() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-02T10:05:00Z"));
    authenticator.authenticate(context);
    ArgumentCaptor<CreateMessengerLinkRequest> request =
        ArgumentCaptor.forClass(CreateMessengerLinkRequest.class);
    verify(sender).createMessengerLink(request.capture());
    assertNull(request.getValue().expectedRecipient());
  }

  /**
   * R7-09: a channel in the notes without the list the form offered didn't come from a form that
   * asks for one: it is refused, and no code goes to the number it names.
   */
  @Test
  void aFormChannelWithoutTheOfferedListIsRefused() throws Exception {
    enrolling(MessageChannel.WHATSAPP);
    notes.remove(MessagingAttributes.NOTE_OFFERED_CHANNELS);
    authenticator.authenticate(context);
    verify(sender, never()).send(any());
    verify(event).detail("reason", "Channel not offered for codes");
  }

  /** The default policy reveals nothing; enrollment always names the channel just entered. */
  @Test
  void theDefaultPolicyIsNeutralAndEnrollmentAlwaysNamesTheChannel() throws Exception {
    assertEquals(Utils.CodePageDisclosure.NEUTRAL, Utils.CodePageDisclosure.fromConfig(Map.of()));
    assertEquals(
        Utils.CodePageDisclosure.NAMED,
        Utils.CodePageDisclosure.of(Map.of(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL"), true));
    configMap.remove(Utils.CODE_PAGE_DISCLOSURE);
    enrolling(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);
    verify(form).setAttribute("channel", "WHATSAPP");
  }

  /** An optional step evaluated before the user is known still lets Keycloak move on. */
  @Test
  void anOptionalNeutralStepWithoutAUserIsSkipped() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    execution.setRequirement(AuthenticationExecutionModel.Requirement.ALTERNATIVE);
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);
    verify(context).attempted();
    verify(context, never()).challenge(any());
  }

  /** R3-04: an unknown account gets the same neutral page, and nothing is sent. */
  @Test
  void anUnknownAccountGetsTheSameNeutralPageAndNothingIsSent() throws Exception {
    when(form.getMessage("messageOtp.auth.codeInvalid")).thenReturn("messageOtp.auth.codeInvalid");
    configMap.put("code-page-disclosure", "NEUTRAL");
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);

    verify(context, never()).attempted();
    verify(context).challenge(response);
    verify(form).setAttribute("codeDisclosure", "NEUTRAL");
    verify(form).createForm("message-otp.login.ftl");
    verify(sender, never()).send(any());
    verifyNoInteractions(sms);

    parameters.putSingle(Utils.CODE, "123456");
    authenticator.action(context);
    verify(context, never()).success();
    verify(event).error(MessageOTPAuthenticator.INVALID_CODE);
    verify(form, atLeastOnce()).setError(contains("messageOtp.auth.codeInvalid"));
  }

  /** R3-04: a known account's wrong code reads exactly like an unknown account's. */
  @Test
  void aKnownAccountsWrongCodeReadsLikeAnUnknownOnesOnANeutralPage() throws Exception {
    when(form.getMessage("messageOtp.auth.codeInvalid")).thenReturn("messageOtp.auth.codeInvalid");
    configMap.put("code-page-disclosure", "NEUTRAL");
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);
    parameters.putSingle(Utils.CODE, "wrong");
    authenticator.action(context);
    verify(form, atLeastOnce()).setError(contains("messageOtp.auth.codeInvalid"));
    verify(form, never()).setAttribute(eq("channel"), any());
  }

  /** R3-12: a test-mode code proves nothing about the destination: nothing is recorded. */
  @Test
  void aTestModeCodeDoesNotVerifyTheDestination() throws Exception {
    configMap.put(Utils.TEST_MODE_ATTRIBUTE, "true");
    configMap.put(Utils.TEST_MODE_CODE_ATTRIBUTE, "000000");
    enrolling(MessageChannel.WHATSAPP);
    authenticator.authenticate(context);

    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    verify(context).success();
    assertNull(notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
    assertNull(notes.get(MessagingAttributes.NOTE_VERIFIED_DESTINATION));
  }

  /** R3-17: only an https link becomes the page's Messenger link. */
  @Test
  void aMessengerLinkThatIsNotHttpsIsNotShown() throws Exception {
    enrolling(MessageChannel.MESSENGER);
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "javascript:alert(1)", null, "2026-10-02T10:05:00Z"));
    authenticator.authenticate(context);
    verify(form, never()).setAttribute(eq("messengerLink"), any());
    verify(form, never()).setAttribute(eq("messengerQr"), any());
  }

  /** R3-19: a realm whose courier is EMAIL or SMS records no messaging channel. */
  @Test
  void aConfiguredCourierRecordsNoChannel() throws Exception {
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, "EMAIL");
    configMap.put(Utils.DEFERRED_USER_ATTRIBUTE, "true");
    when(context.getUser()).thenReturn(null);
    notes.put("email", EMAIL);
    notes.put(Utils.CODE, "482619");
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, "482619");
    authenticator.action(context);
    verify(context).success();
    assertNull(notes.get(MessagingAttributes.NOTE_VERIFIED_CHANNEL));
  }

  /**
   * R7-01 (lead's regression): a sign-in step configured with BOTH sends a WhatsApp-recorded
   * voter's code only to WhatsApp: no SMS, no email.
   */
  @Test
  void aBothSignInStepSendsAWhatsAppVotersCodeOnlyToWhatsApp() throws Exception {
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.BOTH.name());
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);

    SendMessageRequest request = sentRequest();
    assertEquals(MessageChannel.WHATSAPP, request.channel());
    assertEquals(WHATSAPP, request.destination());
    verifyNoInteractions(sms);
    verify(session, never()).getProvider(org.keycloak.email.EmailTemplateProvider.class);
    verify(session, never()).getProvider(org.keycloak.email.EmailSenderProvider.class);
    verify(form).setAttribute("channel", "WHATSAPP");

    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();
    verifyNoInteractions(sms);
  }

  /** R7-01: every configured courier yields to a recorded or first-release channel. */
  @Test
  void everyConfiguredCourierYieldsToTheVotersRecordedChannel() throws Exception {
    for (String courier : List.of("EMAIL", "SMS", "BOTH")) {
      clearInvocations(sender, sms, form);
      notes.clear();
      attributes.clear();
      attributes.put(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD, List.of(PHONE));
      configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, courier);
      attributes.put(MessagingAttributes.VERIFIED_CHANNELS, List.of("VIBER"));
      attributes.put(MessagingAttributes.VIBER_NUMBER, List.of(WHATSAPP));
      authenticator.authenticate(context);
      assertEquals(MessageChannel.VIBER, sentRequest().channel(), courier);
      verifyNoInteractions(sms);
    }
  }

  /**
   * R7-01: a recorded channel that can't be used is refused with its reason on a configured courier
   * step too; the step's courier never sends to the email or the mobile number instead.
   */
  @Test
  void aRecordedChannelWithoutItsNumberIsRefusedOnAConfiguredCourierStep() throws Exception {
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.BOTH.name());
    recorded(MessageChannel.WHATSAPP, null);
    authenticator.authenticate(context);

    verify(sender, never()).send(any());
    verifyNoInteractions(sms);
    verify(session, never()).getProvider(org.keycloak.email.EmailTemplateProvider.class);
    verify(event).detail("reason", "No contact channel");
    verify(form).setError(eq("messageOtp.auth.noChannel"), any());
    assertNull(notes.get(Utils.CODE));
  }

  /** R3-11 kept: a voter without a messaging channel keeps the step's configured courier. */
  @Test
  void aLegacyVoterKeepsTheStepsConfiguredCourier() throws Exception {
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.SMS.name());
    authenticator.authenticate(context);
    verify(sms).send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    verify(sender, never()).send(any());
  }

  /** What a neutral page told: whether a code was sent now and whether the timer started. */
  private List<Object> neutralAnswer() {
    ArgumentCaptor<Object> sent = ArgumentCaptor.forClass(Object.class);
    ArgumentCaptor<Object> timer = ArgumentCaptor.forClass(Object.class);
    verify(form, atLeastOnce()).setAttribute(eq("codeJustSent"), sent.capture());
    verify(form, atLeastOnce()).setAttribute(eq("resendTimerStarted"), timer.capture());
    return List.of(sent.getValue(), timer.getValue());
  }

  /**
   * R7-07: on a neutral page, an account whose channel is refused starts the same resend timer as
   * an unknown account: a resend within 60 s says "wait" for both.
   */
  @Test
  void aRefusedChannelOnANeutralPageStartsTheResendTimerLikeNoAccount() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    recorded(MessageChannel.WHATSAPP, null);
    authenticator.authenticate(context);
    assertNotNull(notes.get(Utils.CODE_TTL));
    assertEquals(List.of(true, true), neutralAnswer());
    clearInvocations(form);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    List<Object> refused = neutralAnswer();

    clearInvocations(form);
    notes.clear();
    parameters.clear();
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);
    clearInvocations(form);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    assertEquals(neutralAnswer(), refused);
    assertEquals(List.of(false, false), refused);
    verify(sender, never()).send(any());
  }

  /** A provider exception must not reveal that a neutral reset identifier has an account. */
  @Test
  void aNeutralSendExceptionAnswersLikeNoAccountAndKeepsTheResendTimer() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    recorded(MessageChannel.SMS, PHONE);
    when(sms.send(anyString(), anyString(), anyList(), any(), any(), any()))
        .thenThrow(new java.io.IOException("synthetic provider refusal"));
    authenticator.authenticate(context);
    verify(context).challenge(response);
    verify(context, never()).failureChallenge(any(), any());
    assertEquals(List.of(true, true), neutralAnswer());
    assertNotNull(notes.get(Utils.CODE_TTL));
    assertNull(notes.get(Utils.CODE));
    clearInvocations(form);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    List<Object> refused = neutralAnswer();
    verify(sms, times(1)).send(anyString(), anyString(), anyList(), any(), any(), any());

    clearInvocations(form);
    notes.clear();
    parameters.clear();
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);
    assertEquals(List.of(true, true), neutralAnswer());
    clearInvocations(form);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    assertEquals(neutralAnswer(), refused);
    assertEquals(List.of(false, false), refused);
  }

  /** A code-only SMS provider refuses links before generating a usable token. */
  @Test
  void aNeutralOneTimeLinkWithACodeOnlyProviderAnswersLikeNoAccount() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    configMap.put(Utils.ONE_TIME_LINK, "true");
    recorded(MessageChannel.SMS, PHONE);
    when(sms.sendsOnlyCodes()).thenReturn(true);
    authenticator.authenticate(context);
    verify(sms).sendsOnlyCodes();
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    assertNull(notes.get(MessagingAttributes.NOTE_CODE_ID));
    assertNull(notes.get(Utils.CODE));
    assertEquals(List.of(true, true), neutralAnswer());
    verify(context, never()).failureChallenge(any(), any());
    assertNotNull(notes.get(Utils.CODE_TTL));
  }

  /** A fake clock: a send that takes {@code sendMillis} moves it on. */
  private final long[] now = {0L};

  private final List<Long> held = new java.util.ArrayList<>();

  private void pacedClock() {
    NeutralPace.clock = () -> now[0] * 1_000_000L;
    NeutralPace.sleeper =
        millis -> {
          held.add(millis);
          now[0] += millis;
        };
  }

  @org.junit.jupiter.api.AfterEach
  void realClock() {
    NeutralPace.clock = System::nanoTime;
    NeutralPace.sleeper = millis -> {};
  }

  /**
   * R7-06: a neutral code page answers no sooner than its floor after the request began, for an
   * account (whose send takes time) and for no account alike, so timing tells them apart only when
   * a send outlasts the floor.
   */
  @Test
  void aNeutralPageTakesTheSameTimeWithAndWithoutAnAccount() throws Exception {
    pacedClock();
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    when(sender.send(any()))
        .thenAnswer(
            i -> {
              now[0] += 400;
              return new SendMessageResponse("m-1", "ACCEPTED", null);
            });
    long start = now[0];
    authenticator.authenticate(context);
    long withAccount = now[0] - start;
    verify(sender).send(any());

    notes.clear();
    when(context.getUser()).thenReturn(null);
    start = now[0];
    authenticator.authenticate(context);
    long withoutAccount = now[0] - start;

    assertEquals(NeutralPace.DEFAULT_MILLIS, withAccount);
    assertEquals(withAccount, withoutAccount);
    assertEquals(List.of(NeutralPace.DEFAULT_MILLIS - 400, NeutralPace.DEFAULT_MILLIS), held);
  }

  /** R7-06: the floor is the step's {@code neutral-response-ms}; a named page isn't held. */
  @Test
  void onlyANeutralPageIsHeldAndItsFloorIsConfigurable() throws Exception {
    pacedClock();
    recorded(MessageChannel.WHATSAPP, WHATSAPP);
    authenticator.authenticate(context);
    assertEquals(List.of(), held);

    notes.clear();
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    configMap.put(NeutralPace.CONFIG, "1200");
    authenticator.authenticate(context);
    assertEquals(List.of(1200L), held);
  }

  // R7-10: a Messenger voter whose saved Page isn't the event's Page any more.

  private void linkCreated() throws Exception {
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-02T10:05:00Z"));
  }

  private CreateMessengerLinkRequest linkRequest() throws Exception {
    ArgumentCaptor<CreateMessengerLinkRequest> request =
        ArgumentCaptor.forClass(CreateMessengerLinkRequest.class);
    verify(sender).createMessengerLink(request.capture());
    return request.getValue();
  }

  /** Other voters who use a Page-scoped ID. */
  private final List<UserModel> othersWithTheAccount = new java.util.ArrayList<>();

  private void signedInWithAPasswordAfterAPageChange() {
    recordedMessenger("psid-old", "page-0");
    attributes.put(MessagingAttributes.MESSAGE_CONSENT, List.of("{\"version\":\"1\"}"));
    notes.put("PASSWORD_VALIDATED", "true");
    org.keycloak.models.UserProvider users = mock(org.keycloak.models.UserProvider.class);
    when(session.users()).thenReturn(users);
    when(users.searchForUserByUserAttributeStream(any(), anyString(), anyString()))
        .thenAnswer(i -> othersWithTheAccount.stream());
  }

  /** A saved voter's link names their Page as well as their account; harvest checks both. */
  @Test
  void aSavedVotersLinkNamesTheirPage() throws Exception {
    recordedMessenger("psid-7", "page-1");
    linkCreated();
    authenticator.authenticate(context);
    CreateMessengerLinkRequest request = linkRequest();
    assertEquals("psid-7", request.expectedRecipient());
    assertEquals("page-1", request.expectedPage());
    verify(form, never()).setAttribute(eq("messengerReconnect"), any());
  }

  /**
   * After a Page change, a voter who signed in with their password connects Messenger again in the
   * same session: the link binds to no account, the page says why, and once the code is verified
   * the new account and Page replace the old ones (the contact change's writes; the consent stays).
   */
  @Test
  void afterAPageChangeAVoterReconnectsMessengerAtSignIn() throws Exception {
    signedInWithAPasswordAfterAPageChange();
    linkCreated();
    authenticator.authenticate(context);
    CreateMessengerLinkRequest request = linkRequest();
    assertNull(request.expectedRecipient());
    assertNull(request.expectedPage());
    verify(form).setAttribute("messengerReconnect", true);
    verify(form).setAttribute("messengerLink", "https://m.me/synthetic?ref=ref-1");
    verify(event).detail("messenger", "reconnect");

    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-new", "page-1"));
    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);

    verify(context).success();
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, "MESSENGER");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_ID, "psid-new");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, "page-1");
    verify(user).removeAttribute(MessagingAttributes.MESSAGE_NUMBER);
    verify(user).removeAttribute(MessagingAttributes.VERIFIED_CHANNELS);
    verify(user, never()).removeAttribute(MessagingAttributes.MESSAGE_CONSENT);
    verify(event).detail("messenger", "reconnected");
  }

  /** A reconnect confirmed on another Page than the event's, or not confirmed, saves nothing. */
  @Test
  void aReconnectConfirmedOnAnotherPageSavesNothing() throws Exception {
    signedInWithAPasswordAfterAPageChange();
    linkCreated();
    authenticator.authenticate(context);
    String code = notes.get(Utils.CODE);
    for (MessengerLinkStatus wrong :
        List.of(
            new MessengerLinkStatus("CONFIRMED", "psid-new", "page-0"),
            new MessengerLinkStatus("CONFIRMED", "psid-new", null),
            new MessengerLinkStatus("PENDING", null, null))) {
      liveCode(MessageChannel.MESSENGER, code);
      notes.put(MessagingAttributes.NOTE_MESSENGER_REFERENCE, "ref-1");
      notes.put(MessagingAttributes.NOTE_MESSENGER_RECONNECT, "page-1");
      when(sender.confirmMessengerLink(any())).thenReturn(wrong);
      parameters.putSingle(Utils.CODE, code);
      authenticator.action(context);
    }
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
  }

  /** The new account already belongs to another voter: refused with the reason, nothing saved. */
  @Test
  void aReconnectToAnAccountAnotherVoterUsesIsRefused() throws Exception {
    signedInWithAPasswordAfterAPageChange();
    UserModel other = mock(UserModel.class);
    when(other.getId()).thenReturn("voter-2");
    othersWithTheAccount.add(other);
    when(form.getMessage(anyString())).thenAnswer(i -> i.getArgument(0));
    linkCreated();
    authenticator.authenticate(context);
    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-new", "page-1"));
    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);

    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
    verify(form).setError(startsWith("resetAppOtp.auth.error.maxReceiverReuse"));
  }

  /**
   * Without a password in this session (a code-only step, a reset) there is no reconnect: the code
   * is refused with the reason and the page says how to reconnect.
   */
  @Test
  void withoutAPasswordThereIsNoReconnect() throws Exception {
    signedInWithAPasswordAfterAPageChange();
    notes.remove("PASSWORD_VALIDATED");
    linkCreated();
    authenticator.authenticate(context);
    verify(sender, never()).createMessengerLink(any());
    verify(event)
        .detail(
            "reason",
            sequent.keycloak.authenticator.messaging.MessageReason
                .MESSENGER_RECONNECT_NEEDS_PASSWORD
                .reason());
    verify(form).setError("messageOtp.messenger.reconnectNeedsPassword");
  }

  /** A neutral page never reconnects nor tells: nothing is sent, like any refused channel. */
  @Test
  void aNeutralPageDoesNotReconnect() throws Exception {
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, "NEUTRAL");
    signedInWithAPasswordAfterAPageChange();
    linkCreated();
    authenticator.authenticate(context);
    verify(sender, never()).createMessengerLink(any());
    verify(form, never()).setAttribute(eq("messengerReconnect"), any());
    verify(event)
        .detail(
            "reason",
            sequent.keycloak.authenticator.messaging.MessageReason.MESSENGER_PAGE_CHANGED.reason());
    assertNotNull(notes.get(Utils.CODE_TTL));
  }

  /**
   * Lead decision: a NONE step stays NONE. It sends nothing on any path: no sender call, no
   * Messenger link, no SMS, no email; for a legacy voter, a voter with a recorded channel, while
   * enrolling, and on a resend. So it can't reach any other channel than the voter's.
   */
  @Test
  void aNoneStepSendsNoCodeAnywhere() throws Exception {
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.NONE.name());
    configMap.put(Utils.RESEND_ACTIVATION_TIMER, "0");
    Runnable[] voters = {
      () -> {},
      () -> recorded(MessageChannel.WHATSAPP, WHATSAPP),
      () -> recordedMessenger("psid-7", "page-1"),
      () -> enrolling(MessageChannel.WHATSAPP),
    };
    for (Runnable voter : voters) {
      notes.clear();
      voter.run();
      authenticator.authenticate(context);
      // A code was issued (and its timer started): the send path ran and sent nothing.
      assertNotNull(notes.get(Utils.CODE));
      notes.remove(Utils.CODE_TTL);
      parameters.putSingle("resend", "true");
      authenticator.action(context);
      parameters.clear();
    }
    verify(sender, never()).send(any());
    verify(sender, never()).createMessengerLink(any());
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    verify(sms, never()).send(anyString(), anyString());
    verify(session, never()).getProvider(org.keycloak.email.EmailTemplateProvider.class);
    verify(session, never()).getProvider(org.keycloak.email.EmailSenderProvider.class);
  }
}
