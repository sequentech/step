// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.FakePace;
import sequent.keycloak.authenticator.NeutralPace;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.SendMessageRequest;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

/** R3-01 and R3-10: the temporary password replaces the old one only once it was handed over. */
class SendNewPasswordTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String WHATSAPP = "+15550199999";

  private final SendNewPassword authenticator = new SendNewPassword();
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserModel user = mock(UserModel.class);
  private final EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
  private final SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
  private final MessageSenderProvider sender = mock(MessageSenderProvider.class);
  private final SmsSenderProvider sms = mock(SmsSenderProvider.class);
  private final EmailTemplateProvider email = mock(EmailTemplateProvider.class, RETURNS_SELF);
  private final Map<String, List<String>> attributes = new HashMap<>();
  private final Map<String, String> configMap = new HashMap<>();

  @BeforeEach
  void setup() throws Exception {
    when(context.getUser()).thenReturn(user);
    when(context.getRealm()).thenReturn(realm);
    when(context.getSession()).thenReturn(session);
    when(context.getEvent()).thenReturn(event);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(configMap);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    RootAuthenticationSessionModel root = mock(RootAuthenticationSessionModel.class);
    when(root.getId()).thenReturn("synthetic-session");
    when(authSession.getParentSession()).thenReturn(root);
    when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-synthetic");
    when(realm.getDisplayName()).thenReturn("Synthetic Commission");
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE))
        .thenReturn(
            """
            {"version": 1, "channels": [{"channel": "WHATSAPP", "purposes": ["OTP"]}]}
            """);
    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(keycloakContext);
    when(keycloakContext.getRealm()).thenReturn(realm);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    ThemeManager themes = mock(ThemeManager.class);
    Theme theme = mock(Theme.class);
    when(session.theme()).thenReturn(themes);
    when(themes.getTheme(Theme.Type.LOGIN)).thenReturn(theme);
    when(theme.getEnhancedMessages(any(), any())).thenReturn(new Properties());
    PasswordPolicyManagerProvider policies = mock(PasswordPolicyManagerProvider.class);
    when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(policies);
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(session.getProvider(EmailTemplateProvider.class)).thenReturn(email);
    when(sender.getChannels())
        .thenReturn(
            Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER));
    when(sender.delivers(any())).thenCallRealMethod();

    when(user.getId()).thenReturn("voter-1");
    when(user.getEmail()).thenReturn("voter@example.test");
    when(user.credentialManager()).thenReturn(credentials);
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), List.of()).stream());
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of(WHATSAPP));
  }

  private void harvestAnswers(String state, String reason) {
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-1", state, reason));
  }

  @Test
  void aRefusedTemporaryPasswordLeavesThePasswordUnchangedAndRecordsTheReason() {
    harvestAnswers("FAILED", "Channel not offered for notices");

    authenticator.authenticate(context);

    verify(sender).send(any());
    verify(credentials, never()).updateCredential(any());
    verify(user, never()).setSingleAttribute(anyString(), anyString());
    verify(event).detail("reason", "Channel not offered for notices");
    verify(event).error(Errors.EMAIL_SEND_FAILED);
    verify(event, never()).success();
    ArgumentCaptor<FormMessage> page = ArgumentCaptor.forClass(FormMessage.class);
    verify(context).forkWithSuccessMessage(page.capture());
    assertEquals(SendNewPassword.NEUTRAL_SENT_MESSAGE_KEY, page.getValue().getMessage());
  }

  @Test
  void aFailedTemporaryPasswordLeavesThePasswordUnchanged() {
    harvestAnswers("FAILED", "rejected-422");
    authenticator.authenticate(context);
    verify(credentials, never()).updateCredential(any());
    verify(event).error(Errors.EMAIL_SEND_FAILED);
  }

  @Test
  void aTemporaryPasswordHandedOverReplacesThePassword() {
    for (String state : List.of("ACCEPTED", "QUEUED", "UNKNOWN")) {
      reset(credentials, event);
      when(event.clone()).thenReturn(event);
      harvestAnswers(state, null);
      authenticator.authenticate(context);
      verify(credentials, times(1)).updateCredential(any());
      verify(event, never()).error(anyString());
    }
  }

  @Test
  void thePasswordSentIsThePasswordSet() {
    harvestAnswers("ACCEPTED", null);
    authenticator.authenticate(context);
    ArgumentCaptor<SendMessageRequest> request = ArgumentCaptor.forClass(SendMessageRequest.class);
    verify(sender).send(request.capture());
    ArgumentCaptor<org.keycloak.credential.CredentialInput> input =
        ArgumentCaptor.forClass(org.keycloak.credential.CredentialInput.class);
    verify(credentials).updateCredential(input.capture());
    assertTrue(
        request
            .getValue()
            .content()
            .templateParameters()
            .contains(input.getValue().getChallengeResponse()));
  }

  /** R3-10: a voter without an email still gets the temporary password on their channel. */
  @Test
  void aVoterWithoutAnEmailGetsTheTemporaryPasswordOnTheirChannel() {
    when(user.getEmail()).thenReturn(null);
    harvestAnswers("ACCEPTED", null);
    authenticator.authenticate(context);
    verify(sender).send(any());
    verify(credentials).updateCredential(any());
    verify(event, never()).error(Errors.INVALID_EMAIL);
  }

  /**
   * R7-06: the page answers no sooner than its floor, for an account (whose send takes time) and
   * for no account alike.
   */
  @Test
  void anUnknownAccountWaitsAsLongAsAnAccount() {
    try (FakePace pace = new FakePace()) {
      when(sender.send(any()))
          .thenAnswer(
              i -> {
                pace.advance(700);
                return new SendMessageResponse("m-1", "ACCEPTED", null);
              });
      long start = pace.now();
      authenticator.authenticate(context);
      long withAccount = pace.now() - start;

      when(context.getUser()).thenReturn(null);
      start = pace.now();
      authenticator.authenticate(context);
      long withoutAccount = pace.now() - start;

      assertEquals(NeutralPace.DEFAULT_MILLIS, withAccount);
      assertEquals(withAccount, withoutAccount);
      configMap.put(NeutralPace.CONFIG, "500");
      authenticator.authenticate(context);
      assertEquals(
          List.of(NeutralPace.DEFAULT_MILLIS - 700, NeutralPace.DEFAULT_MILLIS, 500L), pace.held());
    }
    assertTrue(
        authenticator.getConfigProperties().stream()
            .anyMatch(p -> NeutralPace.CONFIG.equals(p.getName())));
  }

  /**
   * R7-08: an SMS provider that sends only codes (Twilio Verify) can't carry a temporary password:
   * it isn't sent, the password is unchanged, and the reason is recorded.
   */
  @Test
  void anSmsProviderThatSendsOnlyCodesNeitherSendsNorSetsTheTemporaryPassword() throws Exception {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("SMS"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of("+15550100777"));
    when(sms.sendsOnlyCodes()).thenReturn(true);

    authenticator.authenticate(context);

    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    verify(sms, never()).send(anyString(), anyString());
    verify(credentials, never()).updateCredential(any());
    verify(event).detail("reason", MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
    verify(event).error(Errors.EMAIL_SEND_FAILED);
  }

  /** The SMS provider still sends the temporary password when it sends messages. */
  @Test
  void anSmsVoterGetsTheTemporaryPasswordByTheSmsProvider() throws Exception {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("SMS"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of("+15550100777"));

    authenticator.authenticate(context);

    verify(sms).send(eq("+15550100777"), anyString(), anyList(), any(), any(), any());
    verify(credentials).updateCredential(any());
  }

  /** R7-13: the identifier the visitor typed (an email or a number) never reaches the event. */
  @Test
  void theTypedIdentifierNeverReachesTheEvent() {
    when(authSession.getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME))
        .thenReturn("voter@example.test");
    harvestAnswers("ACCEPTED", null);
    authenticator.authenticate(context);
    harvestAnswers("FAILED", "unreachable");
    authenticator.authenticate(context);
    verify(event, never()).detail(eq(Details.USERNAME), anyString());
    verify(event, never()).detail(anyString(), eq("voter@example.test"));
  }

  /**
   * R7-16: a temporary password handed over but not set is recorded with its reason, and the page
   * is the same.
   */
  @Test
  void aPasswordThatCouldNotBeSetIsRecordedWithItsReason() {
    harvestAnswers("ACCEPTED", null);
    doThrow(new org.keycloak.models.ModelException("synthetic storage failure"))
        .when(credentials)
        .updateCredential(any());

    authenticator.authenticate(context);

    verify(user, never()).setSingleAttribute(anyString(), anyString());
    verify(event).detail("reason", MessageReason.PASSWORD_NOT_SET.reason());
    verify(event).error(Errors.PASSWORD_REJECTED);
    verify(event, never()).success();
    ArgumentCaptor<FormMessage> page = ArgumentCaptor.forClass(FormMessage.class);
    verify(context).forkWithSuccessMessage(page.capture());
    assertEquals(SendNewPassword.NEUTRAL_SENT_MESSAGE_KEY, page.getValue().getMessage());
  }
}
