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
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;

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
    configMap.put(Utils.CODE_PAGE_DISCLOSURE, Utils.CodePageDisclosure.NAMED.name());
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

  /** KC-R3: a resend never resets the session's attempt count. */
  @Test
  void attemptsSurviveAResend() throws Exception {
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
    parameters.putSingle(Utils.CODE, newCode.equals("000000") ? "111111" : "000000");
    authenticator.action(context);
    assertNull(notes.get(Utils.CODE));
    verify(event).error(MessageOTPAuthenticator.TOO_MANY_ATTEMPTS);
  }

  /** The enrollment's details form, done, then this code step. */
  private void afterTheDetailsForm() {
    statuses = StepBackFlow.afterTheDetailsForm(context, realm, authSession);
  }

  private Map<String, org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus> statuses;

  /** The voter submits the details form again after going back: the flow runs this step next. */
  private void detailsFormSubmitted() {
    statuses.put("details", org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus.SUCCESS);
  }

  @Test
  void backReturnsToTheDetailsFormAndDropsTheCode() {
    afterTheDetailsForm();
    parameters.putSingle(StepBack.PARAM, "true");
    authenticator.action(context);

    verify(context).forceChallenge(any());
    verify(context, never()).success();
    // The code must not verify a contact the voter changes on the details form.
    assertNull(notes.get(Utils.CODE));
  }

  @Test
  void theCodePageOffersBackAfterTheDetailsForm() {
    afterTheDetailsForm();
    authenticator.authenticate(context);
    verify(form).setAttribute(StepBack.ATTRIBUTE, true);
  }

  @Test
  void withoutAPreviousStepBackShowsTheCodePageAgain() {
    parameters.putSingle(StepBack.PARAM, "true");
    authenticator.action(context);

    verify(context, never()).forceChallenge(any());
    verify(form).setAttribute(StepBack.ATTRIBUTE, false);
    assertEquals(CODE, notes.get(Utils.CODE));
  }

  @Test
  void backIsRefusedOnceTheAttemptsAreUsedUp() {
    afterTheDetailsForm();
    fiveWrongCodes();
    parameters.putSingle(StepBack.PARAM, "true");
    authenticator.action(context);

    verify(context, never()).forceChallenge(any());
  }

  private void back() {
    parameters.clear();
    parameters.putSingle(StepBack.PARAM, "true");
    authenticator.action(context);
    parameters.clear();
  }

  /**
   * Back to the details form forgets the code and everything a code proved: the contact may change
   * there. The resend timer and the attempt count are kept.
   */
  @Test
  void backForgetsWhatTheCodeVerifiedAndKeepsTheTimerAndTheAttempts() {
    afterTheDetailsForm();
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    notes.put(Utils.EMAIL_VERIFIED, "true");
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "EMAIL");
    notes.put(MessagingAttributes.NOTE_VERIFIED_DESTINATION, "a@example.test");
    notes.put(MessagingAttributes.NOTE_CODE_CHANNEL, "EMAIL");
    notes.put(MessagingAttributes.NOTE_CODE_DESTINATION, "a@example.test");
    notes.put(Utils.OTL_VISITED, "false");
    String timer = notes.get(Utils.CODE_TTL);
    Map<String, String> attempts = new HashMap<>(notes);
    attempts.keySet().removeIf(key -> !key.startsWith(Utils.CODE_ATTEMPTS));
    assertFalse(attempts.isEmpty());

    back();

    verify(context).forceChallenge(any());
    for (String note :
        java.util.List.of(
            Utils.CODE,
            Utils.EMAIL_VERIFIED,
            Utils.OTL_VISITED,
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_DESTINATION,
            MessagingAttributes.NOTE_CODE_CHANNEL,
            MessagingAttributes.NOTE_CODE_DESTINATION)) {
      assertNull(notes.get(note), note);
    }
    assertEquals(timer, notes.get(Utils.CODE_TTL));
    attempts.forEach((key, value) -> assertEquals(value, notes.get(key), key));
  }

  /**
   * Back and forward again within the resend timer sends nothing: the old code stays forgotten and
   * the voter asks for a new one once the timer allows. Back is not a way around the timer.
   */
  @Test
  void backAndForwardWithinTheResendTimerSendsNothingAndTheOldCodeIsDead() throws Exception {
    afterTheDetailsForm();
    codeIssuedSecondsAgo(1);
    for (int round = 0; round < 3; round++) {
      back();
      verify(context, times(round + 1)).forceChallenge(any());
      detailsFormSubmitted();
      authenticator.authenticate(context);
    }
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());

    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context, never()).success();

    parameters.clear();
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
  }

  /** Once the resend timer has run, coming forward sends one new code, and only it verifies. */
  @Test
  void forwardAfterTheResendTimerSendsOneNewCode() throws Exception {
    afterTheDetailsForm();
    codeIssuedSecondsAgo(61);
    back();
    detailsFormSubmitted();
    authenticator.authenticate(context);
    authenticator.authenticate(context);

    verify(sms, times(1))
        .send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    String newCode = notes.get(Utils.CODE);
    assertNotNull(newCode);

    parameters.putSingle(Utils.CODE, newCode);
    authenticator.action(context);
    verify(context).success();
  }

  /**
   * Back from the identity check runs the verified code step again: its page waits for the resend
   * timer like any other Back, so going back and forth can't send one message after another.
   */
  @Test
  void backFromTheIdentityCheckWaitsForTheResendTimer() throws Exception {
    AuthenticationExecutionModel code = context.getExecution();
    AuthenticationExecutionModel identity =
        StepBackFlow.atTheIdentityCheck(context, realm, authSession);
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context).success();

    when(context.getExecution()).thenReturn(identity);
    assertTrue(StepBack.toPreviousStep(context));
    when(context.getExecution()).thenReturn(code);
    parameters.clear();
    authenticator.authenticate(context);
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());

    codeIssuedSecondsAgo(61);
    authenticator.authenticate(context);
    authenticator.authenticate(context);
    verify(sms, times(1))
        .send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
  }

  /**
   * Without Back nothing waits: the next code step of the flow sends its code at once, although the
   * last step's code was sent a second ago.
   */
  @Test
  void theNextCodeStepSendsAtOnceAfterTheLastOneSucceeded() throws Exception {
    codeIssuedSecondsAgo(1);
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);
    verify(context).success();

    context.getExecution().setId("second-code-step");
    parameters.clear();
    authenticator.authenticate(context);
    verify(sms, times(1))
        .send(eq(PHONE), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), any(), any(), any());
    assertNotNull(notes.get(Utils.CODE));
  }

  private long resendSecondsLeftShown() {
    ArgumentCaptor<Object> left = ArgumentCaptor.forClass(Object.class);
    verify(form).setAttribute(eq("resendSecondsLeft"), left.capture());
    return (Long) left.getValue();
  }

  /**
   * After Back the page has no live code: it says so, and its countdown continues the last send's
   * instead of offering a resend the server refuses. Rendering it again, or Back again, never
   * restarts the full wait.
   */
  @Test
  void aPacedPageShowsTheWaitLeftAndRenderingAgainDoesNotRestartIt() throws Exception {
    afterTheDetailsForm();
    codeIssuedSecondsAgo(20);
    back();
    detailsFormSubmitted();
    clearInvocations(form);
    authenticator.authenticate(context);

    verify(form).setInfo(MessageOTPAuthenticator.NO_CODE_KEY);
    verify(form).setAttribute("resendTimerStarted", false);
    verify(form).setAttribute("codeJustSent", false);
    long first = resendSecondsLeftShown();
    assertTrue(first > 35 && first <= 40, "left=" + first);

    codeIssuedSecondsAgo(50);
    back();
    detailsFormSubmitted();
    clearInvocations(form);
    authenticator.authenticate(context);
    long later = resendSecondsLeftShown();
    assertTrue(later > 5 && later <= 10, "left=" + later);
    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
  }

  /** A page shown right after a send starts the full countdown; it carries no stale wait. */
  @Test
  void aPageThatJustSentStartsTheFullCountdown() throws Exception {
    notes.remove(Utils.CODE);
    notes.remove(Utils.CODE_TTL);
    authenticator.authenticate(context);
    verify(form).setAttribute("resendTimerStarted", true);
    verify(form, never()).setAttribute(eq("resendSecondsLeft"), any());
    verify(form, never()).setInfo(MessageOTPAuthenticator.NO_CODE_KEY);
  }

  /** Going back never resets the step's wrong codes. */
  @Test
  void wrongCodesCountAcrossBack() throws Exception {
    configMap.put(Utils.MAX_CODE_ATTEMPTS, "3");
    afterTheDetailsForm();
    parameters.putSingle(Utils.CODE, "000000");
    authenticator.action(context);
    authenticator.action(context);
    back();
    detailsFormSubmitted();
    codeIssuedSecondsAgo(61);
    authenticator.authenticate(context);
    String newCode = notes.get(Utils.CODE);
    assertNotNull(newCode);

    parameters.putSingle(Utils.CODE, newCode.equals("000000") ? "111111" : "000000");
    authenticator.action(context);
    verify(event).error(MessageOTPAuthenticator.TOO_MANY_ATTEMPTS);
    parameters.putSingle(Utils.CODE, newCode);
    authenticator.action(context);
    verify(context, never()).success();
  }

  /** The used-up page refuses Back, so it doesn't offer it either, also when reloaded. */
  @Test
  void theUsedUpPageOffersNoBack() {
    afterTheDetailsForm();
    for (int attempt = 0; attempt < 4; attempt++) {
      parameters.putSingle(Utils.CODE, "000000");
      authenticator.action(context);
    }
    clearInvocations(form);
    authenticator.action(context);
    parameters.clear();
    authenticator.authenticate(context);

    verify(form, atLeast(2)).setAttribute("attemptsExhausted", true);
    verify(form, never()).setAttribute(StepBack.ATTRIBUTE, true);
    verify(form, atLeast(2)).setAttribute(StepBack.ATTRIBUTE, false);
  }

  private void fiveWrongCodes() {
    for (int attempt = 0; attempt < 5; attempt++) {
      parameters.putSingle(Utils.CODE, "000000");
      authenticator.action(context);
    }
    parameters.clear();
  }

  /** KC-R9: a sixth code after five wrong ones is "Too many attempts", never a 500. */
  @Test
  void aSixthCodeShowsTooManyAttemptsAndNeverAnInternalError() throws Exception {
    fiveWrongCodes();
    parameters.putSingle(Utils.CODE, CODE);
    authenticator.action(context);

    verify(context, never()).success();
    verify(context, never())
        .failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any(Response.class));
    verify(form, never()).createErrorPage(Response.Status.INTERNAL_SERVER_ERROR);
    verify(form, atLeastOnce()).setAttribute("attemptsExhausted", true);
    verify(form, atLeastOnce()).setError(MessageOTPAuthenticator.TOO_MANY_ATTEMPTS_KEY);
  }

  /** After five wrong codes the session is finished: no new code, only a restart. */
  @Test
  void afterFiveWrongCodesNoCodeIsSentAgainInThisSession() throws Exception {
    fiveWrongCodes();
    codeIssuedSecondsAgo(61);
    parameters.putSingle("resend", "true");
    authenticator.action(context);
    parameters.clear();
    authenticator.authenticate(context);

    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    assertNull(notes.get(Utils.CODE));
    verify(form, atLeast(2)).setAttribute("attemptsExhausted", true);
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
      // R2-23: the communications log keeps the channel and the masked number, never the text.
      verify(event)
          .detail(
              eq("msgBody"),
              argThat(
                  (String body) ->
                      body.contains("\"channel\":\"SMS\"")
                          && !body.contains("Your code")
                          && !body.contains(generatedCode)
                          && !body.contains(PHONE)));
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
