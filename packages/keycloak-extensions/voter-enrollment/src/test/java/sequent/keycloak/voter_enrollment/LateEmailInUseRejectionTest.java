// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_SELF;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.doReturn;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.mockito.MockedStatic;
import org.mockito.Mockito;
import sequent.keycloak.authenticator.Utils.MessageCourier;
import sequent.keycloak.voter_enrollment.LookupAndUpdateUser.LateRejection;

/**
 * An enrollment windmill accepted whose email another voter took before Keycloak updated the voter
 * is rejected late. Windmill must record the rejection before the session says REJECTED, and
 * nothing may later show the success page: Keycloak runs the authenticator again on a reload.
 */
class LateEmailInUseRejectionTest {

  private static final String APPLICANT_EMAIL = "voter@example.com";
  private static final String MOBILE = "+34600000000";
  private static final LateRejection REJECTION = new LateRejection("application-1", "voter-1");
  private static final int OK = 200;
  private static final int CONFLICT = 409;

  private final Map<String, String> notes = new HashMap<>();
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final LookupAndUpdateUser authenticator = spy(new LookupAndUpdateUser());
  private MockedStatic<sequent.keycloak.authenticator.Utils> messaging;

  @BeforeEach
  void setUp() {
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(call -> notes.get(call.getArgument(0)));
    doAnswer(call -> notes.put(call.getArgument(0), call.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), any());
    doAnswer(call -> notes.remove(call.getArgument(0)))
        .when(authSession)
        .removeAuthNote(anyString());
    RootAuthenticationSessionModel root = mock(RootAuthenticationSessionModel.class);
    when(root.getId()).thenReturn("enrollment-session");
    when(authSession.getParentSession()).thenReturn(root);

    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    when(session.getContext()).thenReturn(keycloakContext);

    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    when(config.getConfig()).thenReturn(Map.of("messageCourierAttribute", "EMAIL"));

    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(context.form()).thenReturn(form);
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);

    // The state authenticate() leaves once windmill accepted the enrollment
    notes.put("email", APPLICANT_EMAIL);
    notes.put(Utils.PHONE_NUMBER_ATTRIBUTE, MOBILE);
    notes.put(LookupAndUpdateUser.VERIFICATION_STATUS, "ACCEPTED");
    notes.put(LookupAndUpdateUser.VERIFICATION_COMPLETED, "true");

    // Static methods do nothing: no message leaves the test
    messaging = mockStatic(sequent.keycloak.authenticator.Utils.class);
  }

  @AfterEach
  void tearDown() {
    messaging.close();
  }

  private void windmillAnswers(int status) throws Exception {
    doReturn(status).when(authenticator).postEmailInUseRejection(any(), any());
  }

  private void reload() {
    Mockito.clearInvocations(form, context);
    authenticator.authenticate(context);
  }

  private void verifyRejectionShown() {
    verify(form).createForm("registration-rejected-finish.ftl");
    verify(form).setAttribute("rejectReason", "EMAIL_IN_USE");
    verify(form, never()).createForm("registration-finish.ftl");
  }

  private void verifyRejectionSent(int times) {
    messaging.verify(
        () ->
            sequent.keycloak.authenticator.Utils.sendRejectCommunication(
                eq(session),
                eq(realm),
                eq(MessageCourier.EMAIL),
                eq(APPLICANT_EMAIL),
                eq(MOBILE),
                eq("EMAIL_IN_USE"),
                any(),
                any(),
                eq(context)),
        times(times));
  }

  /** The state of an enrollment windmill hasn't recorded as rejected: an error, never a result. */
  private void verifyNotYetRejected() {
    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    verify(context, never()).challenge(any());
    verify(form, never()).createForm(anyString());
    assertEquals("ACCEPTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));
    assertNull(notes.get(LookupAndUpdateUser.VERIFICATION_REJECTION_REASON));
    assertEquals(REJECTION.toNote(), notes.get(LookupAndUpdateUser.EMAIL_IN_USE_REJECTION_NOTE));
  }

  @Test
  void aRecordedRejectionIsShownSentAndKept() throws Exception {
    windmillAnswers(OK);

    authenticator.rejectEmailInUse(context, REJECTION);

    verify(authenticator).postEmailInUseRejection(context, REJECTION);
    assertEquals("REJECTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));
    assertEquals("EMAIL_IN_USE", notes.get(LookupAndUpdateUser.VERIFICATION_REJECTION_REASON));
    assertEquals("true", notes.get(LookupAndUpdateUser.VERIFICATION_COMPLETED));
    assertNull(notes.get(LookupAndUpdateUser.VERIFICATION_MISMATCHED_FIELDS));
    assertNull(notes.get(LookupAndUpdateUser.EMAIL_IN_USE_REJECTION_NOTE));
    verifyRejectionShown();
    verify(context).challenge(any());
    verify(context, never()).failureChallenge(any(), any());
    verifyRejectionSent(1);
  }

  /** A reload after the recorded rejection shows it again, without asking windmill again. */
  @Test
  void aReloadAfterTheRejectionShowsTheRejectionAgain() throws Exception {
    windmillAnswers(OK);
    authenticator.rejectEmailInUse(context, REJECTION);

    reload();

    verifyRejectionShown();
    verify(authenticator, times(1)).postEmailInUseRejection(any(), any());
    verifyRejectionSent(1);
  }

  /** Windmill unreachable: the voter sees an error, not a result, and nobody is told. */
  @Test
  void aRejectionWindmillDidNotRecordIsAnError() throws Exception {
    doThrow(new IOException("harvest down"))
        .when(authenticator)
        .postEmailInUseRejection(any(), any());

    authenticator.rejectEmailInUse(context, REJECTION);

    verifyNotYetRejected();
    verifyRejectionSent(0);
  }

  /** Windmill's own checks refused it: the voter sees an error, never the success page. */
  @Test
  void aRejectionWindmillRefusedIsAnErrorEvenOnReload() throws Exception {
    windmillAnswers(CONFLICT);

    authenticator.rejectEmailInUse(context, REJECTION);
    verifyNotYetRejected();

    reload();
    verifyNotYetRejected();
    verify(authenticator, times(2)).postEmailInUseRejection(any(), any());
    verifyRejectionSent(0);
  }

  /**
   * Windmill rejected the application but its answer was lost: the reload asks again, windmill
   * answers that it is rejected, and the rejection is shown and sent once.
   */
  @Test
  void aReloadAfterALostAnswerRetriesAndShowsTheRejection() throws Exception {
    doThrow(new IOException("connection reset"))
        .doReturn(OK)
        .when(authenticator)
        .postEmailInUseRejection(any(), any());
    authenticator.rejectEmailInUse(context, REJECTION);
    verifyNotYetRejected();

    reload();

    verify(authenticator, times(2)).postEmailInUseRejection(any(), eq(REJECTION));
    verifyRejectionShown();
    assertEquals("REJECTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));
    assertNull(notes.get(LookupAndUpdateUser.EMAIL_IN_USE_REJECTION_NOTE));
    verifyRejectionSent(1);

    reload();
    verifyRejectionShown();
    verify(authenticator, times(2)).postEmailInUseRejection(any(), any());
  }

  /** Windmill answering anything else, a server error included, isn't a recorded rejection. */
  @Test
  void aServerErrorIsNotARecordedRejection() throws Exception {
    windmillAnswers(500);

    assertEquals(
        LookupAndUpdateUser.LateRejectionOutcome.FAILED,
        authenticator.recordEmailInUseRejection(context, REJECTION));
  }

  /** An older harvest names no application: nothing to ask, so it can't be recorded. */
  @Test
  void anEnrollmentWithoutItsApplicationIsNotRecorded() throws Exception {
    LateRejection unnamed = new LateRejection(null, "voter-1");

    authenticator.rejectEmailInUse(context, unnamed);

    verify(authenticator, never()).postEmailInUseRejection(any(), any());
    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    assertNotNull(notes.get(LookupAndUpdateUser.EMAIL_IN_USE_REJECTION_NOTE));
    reload();
    verify(form, never()).createForm("registration-finish.ftl");
  }

  @Test
  void theNoteKeepsTheApplicationAndVoter() {
    assertEquals(REJECTION, LateRejection.fromNote(REJECTION.toNote()));
  }

  /** A message that can't be sent is an error, but the rejection stays recorded and shown. */
  @Test
  void anUnsentRejectionIsAnErrorButStaysRejected() throws Exception {
    windmillAnswers(OK);
    messaging
        .when(
            () ->
                sequent.keycloak.authenticator.Utils.sendRejectCommunication(
                    any(), any(), any(), any(), any(), any(), any(), any(), any()))
        .thenThrow(new IOException("courier down"));

    authenticator.rejectEmailInUse(context, REJECTION);

    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    assertEquals("REJECTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));
    reload();
    verifyRejectionShown();
  }

  /**
   * Windmill's checks refused the rejection for good: the voter is told to start the enrollment
   * again, on every reload, and never sees a result windmill hasn't recorded.
   */
  @Test
  void aRefusedRejectionTellsTheVoterToStartAgain() throws Exception {
    windmillAnswers(CONFLICT);

    authenticator.rejectEmailInUse(context, REJECTION);
    verify(form).setError(Utils.ERROR_EMAIL_IN_USE_NOT_RECORDED, "enrollment-session");
    verify(form, never()).setError(eq(Utils.ERROR_GENERATING_APPROVAL), any());

    reload();
    verify(form).setError(Utils.ERROR_EMAIL_IN_USE_NOT_RECORDED, "enrollment-session");
    verifyNotYetRejected();
  }

  /** Windmill not answering may pass: the generic error, not the start-again guidance. */
  @Test
  void anUnrecordedRejectionKeepsTheGenericError() throws Exception {
    windmillAnswers(500);

    authenticator.rejectEmailInUse(context, REJECTION);

    verify(form).setError(Utils.ERROR_GENERATING_APPROVAL, "enrollment-session");
    verify(form, never()).setError(eq(Utils.ERROR_EMAIL_IN_USE_NOT_RECORDED), any());
  }

  @Test
  void theStartAgainGuidanceIsTranslated() throws IOException {
    for (String language : new String[] {"en", "tl"}) {
      Properties messages = new Properties();
      try (InputStream in =
          getClass()
              .getResourceAsStream(
                  "/theme-resources/messages/messages_" + language + ".properties")) {
        messages.load(new java.io.InputStreamReader(in, StandardCharsets.UTF_8));
      }
      String text = messages.getProperty(Utils.ERROR_EMAIL_IN_USE_NOT_RECORDED);
      assertNotNull(text, language);
      assertTrue(text.contains("{0}"), language);
    }
  }

  /** What harvest received, and answers with the status the test sets. */
  private static final class Harvest implements AutoCloseable {
    final HttpServer server;
    final Map<String, String> received = new HashMap<>();
    int answer = OK;

    Harvest() throws IOException {
      server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
      server.createContext(
          "/",
          exchange -> {
            received.put("method", exchange.getRequestMethod());
            received.put("path", exchange.getRequestURI().getPath());
            received.put("authorization", exchange.getRequestHeaders().getFirst("Authorization"));
            received.put("contentType", exchange.getRequestHeaders().getFirst("Content-Type"));
            received.put(
                "body",
                new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8));
            exchange.sendResponseHeaders(answer, -1);
            exchange.close();
          });
      server.start();
    }

    String domain() {
      return "127.0.0.1:" + server.getAddress().getPort();
    }

    @Override
    public void close() {
      server.stop(0);
    }
  }

  /** The request is the one harvest's route takes: path, service-account token and body fields. */
  @Test
  void theRequestIsHarvestsRoute() throws Exception {
    try (Harvest harvest = new Harvest()) {
      int status =
          LookupAndUpdateUser.sendEmailInUseRejection(
              harvest.domain(), "service-token", "tenant-1", "event-1", "session-1", REJECTION);

      assertEquals(OK, status);
      assertEquals("POST", harvest.received.get("method"));
      assertEquals("/reject-application-email-in-use", harvest.received.get("path"));
      assertEquals("Bearer service-token", harvest.received.get("authorization"));
      assertEquals("application/json", harvest.received.get("contentType"));
      Map<String, String> body =
          new ObjectMapper()
              .readValue(harvest.received.get("body"), new TypeReference<Map<String, String>>() {});
      assertEquals(
          Map.of(
              "tenant_id", "tenant-1",
              "election_event_id", "event-1",
              "application_id", "application-1",
              "session_id", "session-1",
              "user_id", "voter-1"),
          body);
    }
  }

  /** Harvest's real answers, over HTTP, decide what the voter sees. */
  @Test
  void harvestsAnswersOverHttpDecideTheOutcome() throws Exception {
    try (Harvest harvest = new Harvest()) {
      doAnswer(
              call ->
                  LookupAndUpdateUser.sendEmailInUseRejection(
                      harvest.domain(), "token", "tenant-1", "event-1", "session-1", REJECTION))
          .when(authenticator)
          .postEmailInUseRejection(any(), any());

      harvest.answer = CONFLICT;
      authenticator.rejectEmailInUse(context, REJECTION);
      verify(form).setError(Utils.ERROR_EMAIL_IN_USE_NOT_RECORDED, "enrollment-session");
      assertEquals("ACCEPTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));

      harvest.answer = OK;
      reload();
      verifyRejectionShown();
      assertEquals("REJECTED", notes.get(LookupAndUpdateUser.VERIFICATION_STATUS));
    }
  }

  /** Harvest not listening is a failure, never a recorded rejection. */
  @Test
  void anUnreachableHarvestIsNotARecordedRejection() throws Exception {
    String domain;
    try (Harvest harvest = new Harvest()) {
      domain = harvest.domain();
    }
    doAnswer(
            call ->
                LookupAndUpdateUser.sendEmailInUseRejection(
                    domain, "token", "tenant-1", "event-1", "session-1", REJECTION))
        .when(authenticator)
        .postEmailInUseRejection(any(), any());

    assertEquals(
        LookupAndUpdateUser.LateRejectionOutcome.FAILED,
        authenticator.recordEmailInUseRejection(context, REJECTION));
  }
}
