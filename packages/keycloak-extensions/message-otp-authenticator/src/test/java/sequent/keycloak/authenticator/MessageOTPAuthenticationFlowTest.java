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
import java.util.Map;
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
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;

class MessageOTPAuthenticationFlowTest {
  private static final String CODE = "482619";
  private static final String PHONE = "+15550123456";
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
  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, String> configMap = new HashMap<>();
  private final MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();

  @BeforeEach
  void setup() {
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    when(rootSession.getId()).thenReturn("synthetic-session");
    when(authSession.getParentSession()).thenReturn(rootSession);
    when(authSession.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("synthetic-realm");
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
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setRequirement(AuthenticationExecutionModel.Requirement.REQUIRED);
    when(context.getExecution()).thenReturn(execution);
    UserProfileProvider profiles = mock(UserProfileProvider.class);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profiles);
    UPConfig profileConfig = new UPConfig();
    profileConfig.setAttributes(java.util.List.of());
    when(profiles.getConfiguration()).thenReturn(profileConfig);
    when(user.getAttributes()).thenReturn(Map.of());
    when(user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD)).thenReturn(PHONE);
    SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
    when(user.credentialManager()).thenReturn(credentials);
    when(credentials.getStoredCredentialsByTypeStream(MessageOTPCredentialModel.TYPE))
        .thenAnswer(i -> Stream.of(MessageOTPCredentialModel.create(true)));
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    try {
      when(sms.send(anyString(), anyString(), anyList(), any(), any(), any()))
          .thenReturn("delivered");
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
    configMap.putAll(MessageOTPAuthenticatorFactory.getConfigMap(null));
    configMap.put(Utils.MESSAGE_COURIER_ATTRIBUTE, "SMS");
    configMap.put(Utils.TEL_USER_ATTRIBUTE, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(configMap);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    notes.put(Utils.CODE, CODE);
    notes.put(Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + 300_000));
  }

  @Test
  void incorrectCodeProducesOnlyTheStableErrorAndAllowsRetry() throws Exception {
    parameters.putSingle(Utils.CODE, "190283\nforged-log-entry");
    authenticator.action(context);
    verify(event).error(MessageOTPAuthenticator.INVALID_CODE);
    verify(context).failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, response);
    verify(context, never()).success();
    verify(sms).sendFeedback(PHONE, false, realm, user, session);
    assertEquals(CODE, notes.get(Utils.CODE));
  }

  @Test
  void missingCodeIsRejectedWithoutAnException() {
    authenticator.action(context);
    verify(event).error(MessageOTPAuthenticator.INVALID_CODE);
    verify(context).failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, response);
    verify(context, never()).success();
  }

  @Test
  void validCodeIsConsumedAndReportsSuccessfulDeliveryFeedback() throws Exception {
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context).success();
    verify(event).success();
    verify(sms).sendFeedback(PHONE, true, realm, user, session);
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void expiredCodeRemainsRejected() throws Exception {
    notes.put(Utils.CODE_TTL, "1");
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(event).error(MessageOTPAuthenticator.EXPIRED_CODE);
    verify(context).failureChallenge(AuthenticationFlowError.EXPIRED_CODE, response);
    verify(sms).sendFeedback(PHONE, false, realm, user, session);
    verify(context, never()).success();
  }

  @Test
  void missingTestModeCodeDoesNotAcceptMissingInput() {
    configMap.put(Utils.TEST_MODE_ATTRIBUTE, "true");
    configMap.remove(Utils.TEST_MODE_CODE_ATTRIBUTE);
    authenticator.action(context);
    verify(context).failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, response);
    verify(context, never()).success();
  }

  @Test
  void emptyTestModeCodeDoesNotAcceptEmptyInput() {
    configMap.put(Utils.TEST_MODE_ATTRIBUTE, "true");
    configMap.put(Utils.TEST_MODE_CODE_ATTRIBUTE, "");
    parameters.putSingle(Utils.CODE, "");
    authenticator.action(context);
    verify(context).failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, response);
    verify(context, never()).success();
  }

  @Test
  void alternativeOtlCannotFallThroughToOtpAuthentication() {
    configMap.put(Utils.ONE_TIME_LINK, "true");
    context.getExecution().setRequirement(AuthenticationExecutionModel.Requirement.ALTERNATIVE);
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context).attempted();
    verify(context, never()).success();
    assertEquals(CODE, notes.get(Utils.CODE));
  }

  @Test
  void profileAttributeNamedCodeCannotCopyTheSessionOtpIntoAnEvent() {
    UPAttribute attribute = new UPAttribute();
    attribute.setName(Utils.CODE);
    session
        .getProvider(UserProfileProvider.class)
        .getConfiguration()
        .setAttributes(java.util.List.of(attribute));
    parameters.putSingle(Utils.CODE, "190283");
    authenticator.action(context);
    verify(event, never()).detail(eq(Utils.CODE), anyString());
    verify(event).error(MessageOTPAuthenticator.INVALID_CODE);
  }

  @Test
  void configuredTestModeCodeStillAuthenticates() {
    configMap.put(Utils.TEST_MODE_ATTRIBUTE, "true");
    configMap.put(Utils.TEST_MODE_CODE_ATTRIBUTE, "190283");
    parameters.putSingle(Utils.CODE, "190283");
    authenticator.action(context);
    verify(context).success();
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void deliveryFailureDoesNotLogSecretsFromTheProviderException() throws Exception {
    notes.remove(Utils.CODE);
    when(sms.send(eq(PHONE), anyString(), anyList(), eq(realm), eq(user), eq(session)))
        .thenThrow(new java.io.IOException("provider echoed synthetic-provider-secret"));
    try (CapturedLogs logs = new CapturedLogs(MessageOTPAuthenticator.class)) {
      authenticator.authenticate(context);
      assertFalse(logs.text().contains("synthetic-provider-secret"));
    }
    verify(context).failureChallenge(AuthenticationFlowError.INTERNAL_ERROR, response);
    verify(context, never()).success();
  }

  @Test
  void initialAndThrottledResendFormsDoNotLogTheStoredCode() throws Exception {
    try (CapturedLogs logs = new CapturedLogs(MessageOTPAuthenticator.class)) {
      authenticator.authenticate(context);
      parameters.putSingle("resend", "true");
      authenticator.action(context);
      assertFalse(logs.text().isEmpty());
      assertFalse(logs.text().contains(CODE));
    }
    verify(context, times(2)).challenge(response);
    verifyNoInteractions(sms);
    assertEquals(CODE, notes.get(Utils.CODE));
  }

  private void codeIssuedSecondsAgo(long seconds) {
    long ttlMillis = Long.parseLong(configMap.get(Utils.CODE_TTL)) * 1000L;
    notes.put(
        Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + ttlMillis - seconds * 1000L));
  }

  @Test
  void resendIsRefusedUntilTheResendTimerInSecondsHasElapsed() throws Exception {
    codeIssuedSecondsAgo(1);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    assertEquals(CODE, notes.get(Utils.CODE));
  }

  @Test
  void resendIsAllowedOnceTheResendTimerHasElapsed() throws Exception {
    codeIssuedSecondsAgo(61);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sms)
        .send(
            eq(PHONE),
            eq(Utils.SEND_CODE_SMS_I18N_KEY),
            anyList(),
            eq(realm),
            eq(user),
            eq(session));
    assertNotEquals(CODE, notes.get(Utils.CODE));
  }

  @Test
  void fifthWrongCodeInvalidatesTheCode() throws Exception {
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "000000");
      authenticator.action(context);
      assertEquals(CODE, notes.get(Utils.CODE));
    }
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));
    verify(event).error(MessageOTPAuthenticator.TOO_MANY_ATTEMPTS);

    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context, never()).success();
  }

  @Test
  void reloadingAfterExhaustingTheCodeDoesNotBypassTheResendTimer() throws Exception {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "1");
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));

    parameters.clear();
    authenticator.authenticate(context);
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void configuredMaxAttemptsIsHonoured() throws Exception {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "2");
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    assertEquals(CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void aNewCodeStartsWithAFreshAttemptCount() throws Exception {
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "000000");
      authenticator.action(context);
    }
    codeIssuedSecondsAgo(61);
    parameters.clear();
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    String newCode = notes.get(Utils.CODE);
    assertNotEquals(CODE, newCode);

    parameters.clear();
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    assertEquals(newCode, notes.get(Utils.CODE));
  }

  @Test
  void allowedResendDeliversANewCodeAndKeepsItOutOfEventsAndLogs() throws Exception {
    notes.put(Utils.CODE_TTL, "1");
    parameters.putSingle("resend", "true");
    when(sms.send(eq(PHONE), anyString(), anyList(), eq(realm), eq(user), eq(session)))
        .thenAnswer(i -> "Your code is " + ((java.util.List<?>) i.getArgument(2)).get(1));
    try (CapturedLogs logs = new CapturedLogs(MessageOTPAuthenticator.class)) {
      authenticator.action(context);
      String generatedCode = notes.get(Utils.CODE);
      assertNotNull(generatedCode);
      assertTrue(generatedCode.matches("[0-9]{6}"));
      assertFalse(logs.text().contains(CODE));
      assertFalse(logs.text().contains(generatedCode));
      verify(event).detail("msgBody", "Your code is ******");
    }
    verify(context).challenge(response);
    verify(sms)
        .send(
            eq(PHONE),
            eq(Utils.SEND_CODE_SMS_I18N_KEY),
            anyList(),
            eq(realm),
            eq(user),
            eq(session));
    parameters.clear();
    parameters.putSingle(Utils.CODE, notes.get(Utils.CODE));
    authenticator.action(context);
    verify(context).success();
    assertNull(notes.get(Utils.CODE));
  }
}
