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
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
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
  private final UserProvider users = mock(UserProvider.class);
  private final SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, String> configMap = new HashMap<>();
  private final MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();
  private final List<String> verifiedChannels = new ArrayList<>(List.of("SMS"));

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
    when(user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD))
        .thenReturn("+15550123456");
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> verifiedChannels.stream());
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

    configMap.putAll(MessageOTPAuthenticatorFactory.getConfigMap(null));
    configMap.put(Utils.TEL_USER_ATTRIBUTE, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId("flow");
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    execution.setAuthenticatorConfig("config");
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(configMap);
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

  private void resendTimerElapsed() {
    long ttlMillis = Long.parseLong(configMap.get(Utils.CODE_TTL)) * 1000L;
    notes.put(Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + ttlMillis - 61_000L));
  }

  @Test
  void onlyTheMessagingAppsTheRealmUsesForCodesAreOffered() {
    action.requiredActionChallenge(context);

    verify(form).setAttribute("messagingChannels", List.of("WHATSAPP", "MESSENGER"));
    verify(form).setAttribute("messagingPage", "Synthetic Page");
    verify(form).createForm("message-otp.enter-app-contact.ftl");
    verify(context).challenge(response);
    verify(sender, never()).send(any());
  }

  @Test
  void aRealmWithoutMessagingAppsSkipsTheAction() {
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
  void aSenderThatDoesNotDeliverTheAppsSkipsTheAction() {
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(null);
    action.requiredActionChallenge(context);
    verify(context).ignore();
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
  void aNewWhatsAppNumberIsVerifiedOnWhatsAppBeforeItIsSaved() {
    parameters.putSingle(MessagingAttributes.FORM_NOTICE_CHANNEL, "WHATSAPP");
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
    assertEquals("en", request.language());
    assertEquals(TENANT, request.tenantId());
    String code = notes.get(Utils.CODE);
    assertEquals(code, request.content().code());
    verify(form).createForm("message-otp.enter-otp.ftl");
    verify(form).setAttribute("channel", "WHATSAPP");
    verify(form).setAttribute("contact", NEW_NUMBER);
    verify(user, never()).setSingleAttribute(anyString(), anyString());

    submitCode(code);

    verify(user).setSingleAttribute(MessagingAttributes.WHATSAPP_NUMBER, NEW_NUMBER);
    verify(user).setAttribute(MessagingAttributes.VERIFIED_CHANNELS, List.of("SMS", "WHATSAPP"));
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, "WHATSAPP");
    verify(user)
        .setSingleAttribute(
            eq(MessagingAttributes.MESSAGE_CONSENT),
            argThat(
                consent ->
                    consent.contains("\"wording_version\":\"1\"")
                        && consent.contains("\"channel\":\"WHATSAPP\"")));
    verify(user).removeRequiredAction(ResetMessagingAppOTPRequiredAction.PROVIDER_ID);
    verify(context).success();
    verify(credentials, never()).createStoredCredential(any());
    assertNull(notes.get(Utils.CODE));
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));
    assertNull(notes.get(MessagingAttributes.FORM_WHATSAPP_NUMBER));
  }

  @Test
  void noticesStayWhereTheyWereUnlessTheVoterAsks() {
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));
    verify(user).setSingleAttribute(MessagingAttributes.WHATSAPP_NUMBER, NEW_NUMBER);
    verify(user, never()).setSingleAttribute(eq(MessagingAttributes.MESSAGE_CHANNEL), any());
  }

  @Test
  void consentToTheAppIsRequired() {
    parameters.putSingle("channel", "WHATSAPP");
    parameters.putSingle("contact", NEW_NUMBER);
    parameters.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "1:VIBER");
    action.processAction(context);

    verify(sender, never()).send(any());
    verify(form).setError(eq("messaging.consent.required"), any());
    verify(form).createForm("message-otp.enter-app-contact.ftl");
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
  void anAppTheRealmDoesNotUseForCodesCannotBeChosen() {
    enter(MessageChannel.VIBER, NEW_NUMBER);
    enter(MessageChannel.SMS, NEW_NUMBER);
    verify(sender, never()).send(any());
    verify(form, times(2)).setError("messaging.channelChoice.required");
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void wrongCodesSaveNothingAndAreLimited() {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "2");
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    String code = notes.get(Utils.CODE);
    String wrong = code.equals("000000") ? "111111" : "000000";

    submitCode(wrong);
    verify(form).setError("resetAppOtp.auth.error.codeInvalid");
    submitCode(wrong);
    verify(form).setError("messageOtp.auth.tooManyAttempts");
    assertNull(notes.get(Utils.CODE));

    submitCode(code);
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
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

  @Test
  void aFailedSendCanBeRetriedAtOnce() {
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "rejected-422"));
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    verify(form).setError("resetAppOtp.auth.error.sendError");
    assertNull(notes.get(Utils.CODE));
    assertNull(notes.get(MessagingAttributes.NOTE_RESET_CHANNEL));

    when(sender.send(any())).thenReturn(new SendMessageResponse("m-2", "ACCEPTED", null));
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
            realm, MessagingAttributes.WHATSAPP_NUMBER, NEW_NUMBER))
        .thenAnswer(i -> Stream.of(user, other));
    enter(MessageChannel.WHATSAPP, NEW_NUMBER);
    submitCode(notes.get(Utils.CODE));

    verify(form).setError("resetAppOtp.auth.error.maxReceiverReuse");
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void aMessengerConnectionIsConfirmedByHarvestBeforeItIsSaved() throws Exception {
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", "MAPLE", "2026-10-03T10:05:00Z"));
    enter(MessageChannel.MESSENGER, null);

    verify(sender, never()).send(any());
    verify(form).setAttribute("messengerLink", "https://m.me/synthetic?ref=ref-1");
    verify(form).setAttribute("messengerPage", "Synthetic Page");
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

    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_ID, "psid-9");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, "page-1");
    verify(user).setAttribute(MessagingAttributes.VERIFIED_CHANNELS, List.of("SMS", "MESSENGER"));
    verify(context).success();
  }

  @Test
  void anUnconfirmedMessengerLinkSavesNothing() throws Exception {
    when(sender.createMessengerLink(any()))
        .thenReturn(
            new CreateMessengerLinkResponse(
                "ref-1", "https://m.me/synthetic?ref=ref-1", "MAPLE", "2026-10-03T10:05:00Z"));
    enter(MessageChannel.MESSENGER, null);
    when(sender.confirmMessengerLink(any()))
        .thenReturn(new MessengerLinkStatus("REPLACED", null, null));
    submitCode(notes.get(Utils.CODE));

    verify(form).setError("messageOtp.messenger.notConfirmed");
    verify(context, never()).success();
    verify(user, never()).setSingleAttribute(anyString(), anyString());
    assertNull(notes.get(Utils.CODE));
  }
}
