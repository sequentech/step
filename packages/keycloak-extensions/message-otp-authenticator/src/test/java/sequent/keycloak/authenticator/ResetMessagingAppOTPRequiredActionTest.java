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
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RequiredActionConfigModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.CreateMessengerLinkResponse;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagePurpose;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.SendMessageRequest;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

class ResetMessagingAppOTPRequiredActionTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String EVENT = "11111111-2222-3333-4444-555555555555";
  private static final String NEW_NUMBER = "+15550177777";
  private static final String MASKED_NEW_NUMBER = "+1 ••• ••• 777";
  private static final String PROJECTION =
      """
      {"version": 1, "channels": [
        {"channel": "EMAIL", "purposes": ["OTP"]},
        {"channel": "SMS", "purposes": ["OTP"]},
        {"channel": "WHATSAPP", "purposes": ["OTP"], "sender_label": "Synthetic Commission"},
        {"channel": "VIBER", "purposes": ["NOTICE"]},
        {"channel": "MESSENGER", "purposes": ["OTP"],
         "messenger_page": {"page_id": "page-1", "name": "Synthetic Page"}}]}
      """;

  private final ResetMessagingAppOTPRequiredAction action =
      new ResetMessagingAppOTPRequiredAction();
  private final RequiredActionContext context = mock(RequiredActionContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final UserModel user = mock(UserModel.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
  private final Response response = mock(Response.class);
  private final MessageSenderProvider sender = mock(MessageSenderProvider.class);
  private final SmsSenderProvider sms = mock(SmsSenderProvider.class);
  private final UserProvider users = mock(UserProvider.class);
  private final SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, String> configMap = new HashMap<>();
  private final Map<String, String> signInStepConfig = new HashMap<>();
  private final MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();

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
    when(context.getEvent()).thenReturn(mock(EventBuilder.class, RETURNS_SELF));
    when(context.form()).thenReturn(form);
    when(form.createForm(anyString())).thenReturn(response);
    HttpRequest request = mock(HttpRequest.class);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(parameters);
    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(keycloakContext);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    ThemeManager themes = mock(ThemeManager.class);
    Theme theme = mock(Theme.class);
    when(session.theme()).thenReturn(themes);
    when(themes.getTheme(Theme.Type.LOGIN)).thenReturn(theme);
    when(theme.getEnhancedMessages(any(), any())).thenReturn(new Properties());
    when(session.users()).thenReturn(users);
    when(users.searchForUserByUserAttributeStream(any(), anyString(), anyString()))
        .thenAnswer(i -> Stream.of());

    when(user.getId()).thenReturn("voter-1");
    when(user.getEmail()).thenReturn("maria.cruz@example.test");
    when(user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD))
        .thenReturn("+15550123456");
    when(user.getAttributeStream(MessagingAttributes.AUTHORIZED_ELECTIONS))
        .thenAnswer(i -> Stream.of());
    when(user.credentialManager()).thenReturn(credentials);
    when(credentials.getStoredCredentialsByTypeStream(MessageOTPCredentialModel.TYPE))
        .thenAnswer(i -> Stream.of(MessageOTPCredentialModel.create(true)));

    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    when(sender.getChannels())
        .thenReturn(
            Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER));
    when(sender.delivers(any())).thenCallRealMethod();
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", "ACCEPTED", null));
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(sms.send(anyString(), anyString(), anyList(), any(), any(), any())).thenReturn("sent");

    // The action's own configuration in the realm.
    RequiredActionConfigModel own = new RequiredActionConfigModel();
    own.setAlias(ResetMessagingAppOTPRequiredAction.PROVIDER_ID);
    own.setConfig(configMap);
    when(realm.getRequiredActionConfigByAlias(ResetMessagingAppOTPRequiredAction.PROVIDER_ID))
        .thenReturn(own);

    // A sign-in step with other settings, which the action must not use.
    signInStepConfig.putAll(MessageOTPAuthenticatorFactory.getConfigMap(null));
    signInStepConfig.put(Utils.CODE_TTL, "900");
    signInStepConfig.put(Utils.MAX_CODE_ATTEMPTS, "50");
    // A messaging realm: its code steps send to the voter's channel (R7-01).
    signInStepConfig.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.CHOSEN.name());
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId("flow");
    flow.setTopLevel(true);
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    execution.setAuthenticatorConfig("config");
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(signInStepConfig);
    when(realm.getAuthenticationFlowsStream()).thenAnswer(i -> Stream.of(flow));
    when(realm.getAuthenticationExecutionsStream("flow")).thenAnswer(i -> Stream.of(execution));
    when(realm.getAuthenticatorConfigById("config")).thenReturn(config);
  }

  private void enter(MessageChannel channel, String number) {
    parameters.clear();
    parameters.putSingle("channel", channel.name());
    if (number != null) {
      parameters.putSingle("contact", number);
    }
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:" + channel.name());
    action.processAction(context);
  }

  private void submitCode(String code) {
    parameters.clear();
    parameters.putSingle(Utils.CODE, code);
    action.processAction(context);
  }

  private static String wrong(String code) {
    return code.equals("000000") ? "111111" : "000000";
  }

  private void resendTimerElapsed() {
    long ttlMillis =
        Long.parseLong(configMap.getOrDefault(Utils.CODE_TTL, Utils.CODE_TTL_DEFAULT)) * 1000L;
    notes.put(Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + ttlMillis - 61_000L));
  }

  private void verifyReplacedWith(MessageChannel channel) {
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, channel.name());
    verify(user).removeAttribute(MessagingAttributes.VERIFIED_CHANNELS);
    verify(user).removeAttribute(MessagingAttributes.WHATSAPP_NUMBER);
    verify(user).removeAttribute(MessagingAttributes.VIBER_NUMBER);
  }

  @Test
  void theIdIsMessageChannelRaAndTheFirstReleaseIdStillWorks() {
    assertEquals("message-channel-ra", action.getId());
    assertEquals("message-channel-ra", action.getProviderId());
    LegacyMessagingAppOTPRequiredAction legacy = new LegacyMessagingAppOTPRequiredAction();
    assertEquals("messaging-app-otp-ra", legacy.getId());
    assertEquals("messaging-app-otp-ra", legacy.getProviderId());
    assertEquals(legacy, legacy.create(session));
  }

  @Test
  void theContactChangeNeedsARecentSignIn() {
    assertEquals(300, action.getMaxAuthAge(session));
    assertTrue(action.isConfigurable());
    assertTrue(
        action.getConfigMetadata().stream()
            .map(ProviderConfigProperty::getName)
            .toList()
            .containsAll(List.of("max_auth_age", Utils.CODE_TTL, Utils.RESEND_ACTIVATION_TIMER)));
    assertEquals(InitiatedActionSupport.SUPPORTED, action.initiatedActionSupport());
  }

  @Test
  void thePostsChannelsAreOfferedInTheTicketsOrder() {
    action.requiredActionChallenge(context);

    verify(form)
        .setAttribute("messagingChannels", List.of("WHATSAPP", "MESSENGER", "SMS", "EMAIL"));
    verify(form).setAttribute("messagingPage", "Synthetic Page");
    verify(form).setAttribute("messagingEmail", "ma••••••••@example.test");
    verify(form).createForm("message-otp.enter-app-contact.ftl");
    verify(context).challenge(response);
    verify(sender, never()).send(any());
  }

  @Test
  void aRealmWithoutMessagingSkipsTheAction() {
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(null);
    action.requiredActionChallenge(context);
    verify(context).ignore();
    verify(context, never()).challenge(any());

    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", NEW_NUMBER);
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:WHATSAPP");
    action.processAction(context);
    verify(sender, never()).send(any());
  }

  @Test
  void withoutAMessageSenderOnlySmsAndEmailAreOffered() {
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(null);
    action.requiredActionChallenge(context);
    verify(form).setAttribute("messagingChannels", List.of("SMS", "EMAIL"));
  }

  @Test
  void itOnlyRunsForAnAuthenticatedVoter() {
    when(context.getUser()).thenReturn(null);
    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", NEW_NUMBER);
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:WHATSAPP");
    action.processAction(context);

    verify(context).failure();
    verify(sender, never()).send(any());
    assertTrue(notes.isEmpty());
  }

  @Test
  void aNewWhatsAppNumberIsVerifiedAndThenReplacesTheVotersChannel() {
    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", " " + NEW_NUMBER + " ");
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:WHATSAPP");
    action.processAction(context);

    ArgumentCaptor<SendMessageRequest> captor = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(captor.capture());
    SendMessageRequest request = captor.getValue();
    assertEquals(MessageChannel.WHATSAPP, request.channel());
    assertEquals(MessagePurpose.OTP, request.purpose());
    assertEquals(NEW_NUMBER, request.destination());
    assertEquals("voter-1", request.voterId());
    assertEquals("otp", request.templateKey());
    assertNull(request.recipient());
    assertEquals("en", request.language());
    assertEquals(TENANT, request.tenantId());
    String code = notes.get(Utils.CODE);
    assertEquals(code, request.content().code());
    verify(form).createForm("message-otp.enter-otp.ftl");
    verify(form).setAttribute("channel", "WHATSAPP");
    verify(form).setAttribute("contact", MASKED_NEW_NUMBER);
    verify(form, never()).setAttribute("contact", NEW_NUMBER);
    verify(user, never()).setSingleAttribute(anyString(), anyString());

    submitCode(code);

    verifyReplacedWith(MessageChannel.WHATSAPP);
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_NUMBER, NEW_NUMBER);
    verify(user).removeAttribute(MessagingAttributes.MESSENGER_ID);
    verify(user).removeAttribute(MessagingAttributes.MESSENGER_PAGE);
    verify(user)
        .setSingleAttribute(
            eq(MessagingAttributes.MESSAGE_CONSENT),
            argThat(
                consent ->
                    consent.contains("\"version\":\"1\"")
                        && consent.contains("\"organization\":\"Synthetic Commission\"")
                        && consent.contains("\"channel\":\"WHATSAPP\"")
                        && consent.contains("\"at\":")));
    verify(user).removeRequiredAction(ResetMessagingAppOTPRequiredAction.PROVIDER_ID);
    verify(context).success();
    verify(credentials, never()).createStoredCredential(any());
    assertNull(notes.get(Utils.CODE));
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));
    assertNull(notes.get(MessagingAttributes.FORM_WHATSAPP_NUMBER));
  }

  @Test
  void anSmsNumberIsVerifiedBySmsAndRecordedAsTheMessageNumber() throws Exception {
    enter(MessageChannel.SMS, NEW_NUMBER);
    verify(sms)
        .send(eq(NEW_NUMBER), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    verify(sender, never()).send(any());

    submitCode(notes.get(Utils.CODE));
    verifyReplacedWith(MessageChannel.SMS);
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_NUMBER, NEW_NUMBER);
    verify(user, never())
        .setSingleAttribute(eq(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD), anyString());
    verify(context).success();
  }

  @Test
  void consentToTheChannelIsRequired() {
    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", NEW_NUMBER);
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:VIBER");
    action.processAction(context);
    parameters.clear();
    parameters.putSingle("channel", "SMS");
    parameters.putSingle("contact", NEW_NUMBER);
    action.processAction(context);

    verify(sender, never()).send(any());
    verifyNoInteractions(sms);
    verify(form, times(2)).setError(eq("messaging.consent.required"), any());
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));
  }

  @Test
  void aNumberMustBeInInternationalFormat() {
    enter(MessageChannel.WHATSAPP, "0917 555 0100");
    verify(sender, never()).send(any());
    verify(form).setError("messaging.number.invalid");
  }

  @Test
  void aNumberOutsideTheAllowedCountriesIsRefused() {
    configMap.put(Utils.VALID_COUNTRY_CODES, "+63,+34");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    verify(sender, never()).send(any());
    verify(form).setError("resetAppOtp.auth.error.invalidCountry");
  }

  @Test
  void aChannelThePostDoesNotOfferCannotBeChosen() {
    enter(MessageChannel.VIBER, NEW_NUMBER);
    parameters.putSingle("channel", "FAX");
    action.processAction(context);
    verify(sender, never()).send(any());
    verify(form, times(2)).setError("messaging.channelChoice.required");
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void theCodeSettingsAreTheActionsOwn() {
    configMap.put(Utils.CODE_TTL, "120");
    long before = System.currentTimeMillis();
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    long expiry = Long.parseLong(notes.get(Utils.CODE_TTL));
    assertTrue(expiry >= before + 120_000 && expiry <= System.currentTimeMillis() + 120_000);
    verify(form).setAttribute("ttl", "120");
  }

  @Test
  void wrongCodesSaveNothingAndAreLimited() {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "2");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    String code = notes.get(Utils.CODE);

    submitCode(wrong(code));
    verify(form).setError("resetAppOtp.auth.error.codeInvalid");
    submitCode(wrong(code));
    verify(form).setError("messageOtp.auth.tooManyAttempts");
    verify(form, atLeastOnce()).setAttribute("attemptsExhausted", true);
    assertNull(notes.get(Utils.CODE));

    submitCode(code);
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void fiveWrongCodesFinishTheSession() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    String code = notes.get(Utils.CODE);
    for (int attempt = 0; attempt < 5; attempt++) {
      submitCode(wrong(code));
    }
    resendTimerElapsed();
    enter(MessageChannel.WHATSAPP, "+15550188888");
    parameters.clear();
    parameters.putSingle("resend", "true");
    action.processAction(context);
    verify(sender, times(1)).send(any());
    verify(form, atLeast(3)).setAttribute("attemptsExhausted", true);
  }

  /** KC-R3 for the contact change: asking for a new code keeps the attempt count. */
  @Test
  void attemptsSurviveAResend() {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "2");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    String code = notes.get(Utils.CODE);
    submitCode(wrong(code));

    resendTimerElapsed();
    parameters.clear();
    parameters.putSingle("resend", "true");
    action.processAction(context);
    String second = notes.get(Utils.CODE);
    assertNotNull(second);

    submitCode(wrong(second));
    verify(form, atLeastOnce()).setError("messageOtp.auth.tooManyAttempts");
    assertNull(notes.get(Utils.CODE));
    submitCode(second);
    verify(context, never()).success();
  }

  @Test
  void aNewCodeWaitsForTheResendTimer() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    String code = notes.get(Utils.CODE);

    parameters.clear();
    parameters.putSingle("resend", "true");
    action.processAction(context);
    verify(sender, times(1)).send(any());
    verify(form).setError("resetAppOtp.auth.error.resendTimer");
    assertEquals(code, notes.get(Utils.CODE));

    resendTimerElapsed();
    action.processAction(context);
    verify(sender, times(2)).send(any());
    assertNotEquals(null, notes.get(Utils.CODE));
  }

  @Test
  void changingTheContactDoesNotBypassTheResendTimer() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    parameters.clear();
    parameters.putSingle("changeValue", "true");
    action.processAction(context);
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));

    enter(MessageChannel.WHATSAPP, "+15550188888");
    verify(sender, times(1)).send(any());
    verify(form).setError("resetAppOtp.auth.error.resendTimer");
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));
  }

  /** KC-R4: a failed send starts the resend timer, here too. */
  @Test
  void aFailedSendStartsTheResendTimer() {
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "rejected-429"));
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    verify(form).setError("resetAppOtp.auth.error.sendError");
    assertNull(notes.get(Utils.CODE));
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));

    when(sender.send(any())).thenReturn(new SendMessageResponse("m-2", "ACCEPTED", null));
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    verify(sender, times(1)).send(any());
    verify(form).setError("resetAppOtp.auth.error.resendTimer");

    resendTimerElapsed();
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    verify(sender, times(2)).send(any());
    assertNotNull(notes.get(Utils.CODE));
    assertEquals("WHATSAPP", notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));
  }

  @Test
  void aNumberAnotherVoterUsesIsRefused() {
    UserModel other = mock(UserModel.class);
    when(other.getId()).thenReturn("voter-2");
    when(users.searchForUserByUserAttributeStream(
            realm, MessagingAttributes.MESSAGE_NUMBER, NEW_NUMBER))
        .thenAnswer(i -> Stream.of(user, other));
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));

    verify(form).setError("resetAppOtp.auth.error.maxReceiverReuse");
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void aMessengerConnectionIsConfirmedByHarvestBeforeItReplacesTheChannel() throws Exception {
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-03T10:05:00Z"));
    enter(MessageChannel.MESSENGER, null);

    verify(sender, never()).send(any());
    verify(form).setAttribute("messengerLink", "https://m.me/synthetic?ref=ref-1");
    verify(form).setAttribute("messengerPage", "Synthetic Page");
    verify(form)
        .setAttribute(
            eq("messengerQr"), argThat((String qr) -> qr.startsWith("data:image/svg+xml;base64,")));
    String code = notes.get(Utils.CODE);

    when(sender.messengerLinkStatus(any()))
        .thenReturn(new MessengerLinkStatus("CODE_SENT", null, null));
    parameters.clear();
    parameters.putSingle("messengerStatus", "true");
    action.processAction(context);
    verify(form).setAttribute("messengerState", "CODE_SENT");
    assertEquals(code, notes.get(Utils.CODE));

    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("CONFIRMED", "psid-9", "page-1"));
    submitCode(code);

    verifyReplacedWith(MessageChannel.MESSENGER);
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_ID, "psid-9");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, "page-1");
    verify(user).removeAttribute(MessagingAttributes.MESSAGE_NUMBER);
    verify(context).success();
  }

  @Test
  void anUnconfirmedMessengerLinkSavesNothing() throws Exception {
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", null, "2026-10-03T10:05:00Z"));
    enter(MessageChannel.MESSENGER, null);
    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("REPLACED", null, null));
    submitCode(notes.get(Utils.CODE));

    verify(form).setError("messageOtp.messenger.notConfirmed");
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
    assertNull(notes.get(Utils.CODE));
  }

  /**
   * R3-03: the contact change has its own resend timer and attempt counter. A sign-in in the same
   * authentication session (the voter re-authenticated for max_auth_age) leaves its code timer and
   * wrong codes behind; neither applies to the new contact's code.
   */
  @Test
  void theContactChangeDoesNotInheritTheSignInsTimerOrAttempts() {
    configMap.put(Utils.CODE_TTL, "120");
    // What the sign-in's message OTP step left in this session: a code sent 15 s ago with a
    // 300 s lifetime, delivered by SMS, and four wrong codes.
    notes.put(Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + 300_000L - 15_000L));
    notes.put(Utils.CODE_ATTEMPTS, "4");
    notes.put(MessagingAttributes.NOTE_DELIVERY_STATE, "ACCEPTED");
    notes.put(MessagingAttributes.NOTE_CODE_CHANNEL, "SMS");
    notes.put(MessagingAttributes.NOTE_CODE_DESTINATION, "+15550123456");

    action.requiredActionChallenge(context);
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);

    verify(form, never()).setError("resetAppOtp.auth.error.resendTimer");
    verify(sender).send(any());
    String code = notes.get(Utils.CODE);
    for (int attempt = 0; attempt < 4; attempt++) {
      submitCode(wrong(code));
    }
    verify(form, never()).setAttribute("attemptsExhausted", true);
    submitCode(code);
    verify(context).success();
  }

  /** R3-13: a code of the wrong length is a wrong code, never an aborted action. */
  @Test
  void aCodeOfTheWrongLengthIsShownAsInvalid() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode("12");
    verify(context, never()).failure();
    verify(form).setError("resetAppOtp.auth.error.codeInvalid");
  }

  /** R3-13: without a live code the voter is told to ask for a new one. */
  @Test
  void noLiveCodeAsksForANewOneInsteadOfAborting() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    notes.remove(Utils.CODE);
    submitCode("123456");
    verify(context, never()).failure();
    verify(form).setError("messageOtp.auth.noCode");
  }

  /** R3-20: changing the contact forgets the number under the configured attribute. */
  @Test
  void changingTheContactForgetsTheNumberUnderTheConfiguredAttribute() {
    configMap.put(Utils.TEL_USER_ATTRIBUTE, "custom.mobile");
    enter(MessageChannel.SMS, NEW_NUMBER);
    assertEquals(NEW_NUMBER, notes.get("custom.mobile"));
    parameters.clear();
    parameters.putSingle("changeValue", "true");
    action.processAction(context);
    assertNull(notes.get("custom.mobile"));
  }

  private void currentChannel(MessageChannel channel, String number) {
    when(user.getAttributeStream(MessagingAttributes.MESSAGE_CHANNEL))
        .thenAnswer(i -> Stream.of(channel.name()));
    when(user.getAttributeStream(MessagingAttributes.MESSAGE_NUMBER))
        .thenAnswer(i -> Stream.of(number));
  }

  /**
   * R3-05: after a change, the old channel is told about it, so a voter whose password was used to
   * take over their codes finds out. Sent through harvest like every notice.
   */
  @Test
  void theOldChannelIsToldAboutTheChange() {
    currentChannel(MessageChannel.WHATSAPP, "+15550100001");
    enter(MessageChannel.SMS, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));
    verify(context).success();

    ArgumentCaptor<SendMessageRequest> captor = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(captor.capture());
    SendMessageRequest notice = captor.getValue();
    assertEquals(MessagePurpose.NOTICE, notice.purpose());
    assertEquals(MessageChannel.WHATSAPP, notice.channel());
    assertEquals("+15550100001", notice.destination());
    assertEquals("contactChanged.notice.text", notice.templateKey());
    // Harvest sends it to the channel just replaced, not the voter's (new) channel.
    assertEquals(
        sequent.keycloak.authenticator.messaging.MessageRecipient.PREVIOUS_CHANNEL,
        notice.recipient());
    assertFalse(notice.content().text().contains(NEW_NUMBER));
  }

  /** R3-05: an old SMS channel gets the notice by Keycloak's SMS provider, on the old number. */
  @Test
  void anOldSmsChannelIsToldBySms() throws Exception {
    currentChannel(MessageChannel.SMS, "+15550100002");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));
    verify(sms)
        .send(eq("+15550100002"), eq("contactChanged.notice.text"), anyList(), any(), any(), any());
  }

  /**
   * R7-08: an SMS provider that sends only codes can't tell the old SMS channel; the notice isn't
   * sent and isn't recorded as sent. The change itself goes ahead.
   */
  @Test
  void anSmsProviderThatSendsOnlyCodesDoesNotTellTheOldChannel() throws Exception {
    when(sms.sendsOnlyCodes()).thenReturn(true);
    currentChannel(MessageChannel.SMS, "+15550100002");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));
    verify(context).success();
    verify(sms, never())
        .send(anyString(), eq("contactChanged.notice.text"), anyList(), any(), any(), any());
  }

  /** Verifying the same contact again changes nothing, so no notice is sent. */
  @Test
  void noNoticeWhenTheContactDidNotChange() {
    currentChannel(MessageChannel.WHATSAPP, NEW_NUMBER);
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));
    verify(sender, times(1)).send(any());
  }

  /**
   * R3-09: Keycloak's own re-authentication check for an application-initiated action asks this
   * action for its max auth age. A sign-in older than 300 s must sign in again; the realm entry's
   * {@code max_auth_age} changes the limit.
   */
  @Test
  void keycloakAsksForARecentSignInBeforeTheContactChange() {
    org.keycloak.models.KeycloakContext keycloakContext = session.getContext();
    when(keycloakContext.getRealm()).thenReturn(realm);
    when(keycloakContext.getAuthenticationSession()).thenReturn(authSession);
    when(session.getProvider(
            org.keycloak.authentication.RequiredActionProvider.class,
            ResetMessagingAppOTPRequiredAction.PROVIDER_ID))
        .thenReturn(action);
    when(authSession.getClientNote("kc_action"))
        .thenReturn(ResetMessagingAppOTPRequiredAction.PROVIDER_ID);
    org.keycloak.models.UserSessionModel userSession =
        mock(org.keycloak.models.UserSessionModel.class);
    org.keycloak.protocol.oidc.OIDCLoginProtocol protocol =
        new org.keycloak.protocol.oidc.OIDCLoginProtocol(session, realm, null, null, null);
    int now = org.keycloak.common.util.Time.currentTime();

    when(userSession.getNote("AUTH_TIME")).thenReturn(Integer.toString(now - 301));
    assertTrue(protocol.requireReauthentication(userSession, authSession));
    when(userSession.getNote("AUTH_TIME")).thenReturn(Integer.toString(now - 100));
    assertFalse(protocol.requireReauthentication(userSession, authSession));

    configMap.put("max_auth_age", "60");
    assertTrue(protocol.requireReauthentication(userSession, authSession));
  }

  /**
   * R7-01: while a code step of the realm doesn't send to the voter's channel (a configured
   * courier: BOTH, EMAIL, SMS, or no configuration), the contact change is refused with its reason,
   * logged and in the event: a new channel would not get the sign-in codes.
   */
  @Test
  void theContactChangeIsRefusedWhileACodeStepUsesAConfiguredCourier() {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    signInStepConfig.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.BOTH.name());
    action.requiredActionChallenge(context);

    verify(context).ignore();
    verify(context, never()).challenge(any());
    verify(event).detail("reason", "A code step doesn't send to the voter's channel");
    verify(authSession).removeRequiredAction(ResetMessagingAppOTPRequiredAction.PROVIDER_ID);

    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", NEW_NUMBER);
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:WHATSAPP");
    action.processAction(context);
    verify(sender, never()).send(any());
    verify(user, never()).setSingleAttribute(eq(MessagingAttributes.MESSAGE_CHANNEL), any());
  }

  /** R7-01: a code step without its own configuration uses the default courier, BOTH. */
  @Test
  void aCodeStepWithoutConfigurationAlsoRefusesTheContactChange() {
    when(realm.getAuthenticatorConfigById("config")).thenReturn(null);
    action.requiredActionChallenge(context);
    verify(context).ignore();
    verify(context, never()).challenge(any());
  }

  /** A disabled code step sends nothing, so it doesn't block the contact change. */
  @Test
  void aDisabledCodeStepDoesNotRefuseTheContactChange() {
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId("flow");
    flow.setTopLevel(true);
    AuthenticationExecutionModel chosen = new AuthenticationExecutionModel();
    chosen.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    chosen.setAuthenticatorConfig("config");
    chosen.setRequirement(AuthenticationExecutionModel.Requirement.REQUIRED);
    AuthenticationExecutionModel disabled = new AuthenticationExecutionModel();
    disabled.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    disabled.setAuthenticatorConfig("other");
    disabled.setRequirement(AuthenticationExecutionModel.Requirement.DISABLED);
    when(realm.getAuthenticationExecutionsStream("flow"))
        .thenAnswer(i -> Stream.of(chosen, disabled));
    when(realm.getAuthenticatorConfigById("other")).thenReturn(null);
    action.requiredActionChallenge(context);
    verify(context).challenge(response);
  }
}
