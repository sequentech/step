// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_SELF;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.PNG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.WEBM;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.SUCCESSFUL_RESULTS;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.json;

import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.headers.SecurityHeadersOptions;
import org.keycloak.headers.SecurityHeadersProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceComparison;
import sequent.keycloak.voter_enrollment.Utils;

class ScanovateAuthenticatorTest {
  private static final String ACTION_URL =
      "https://kc/realms/r/login-actions/authenticate?session_code=c&execution=e";
  private static final String FLOW_URL = "https://btrust.example.com/flow?process_id=proc-1";
  private static final String STATUS_NOTE = "sequent.read-only.id-card-number-validated";

  private final Map<String, String> config = new HashMap<>();
  private final Map<String, String> authNotes = new HashMap<>();
  private final MultivaluedHashMap<String, String> queryParams = new MultivaluedHashMap<>();
  private final MultivaluedHashMap<String, String> formParams = new MultivaluedHashMap<>();

  private AuthenticationFlowContext context;
  private LoginFormsProvider form;
  private AuthenticationExecutionModel execution;
  private ScanovateClient client;
  private ScanovateAuthenticator authenticator;
  private MockedStatic<Utils> utils;
  private LivenessSessionsTest.MemoryStore livenessStore;
  private LivenessSessions livenessSessions;
  private CaptureUploads captureUploads;
  private HttpHeaders httpHeaders;
  private SecurityHeadersOptions securityHeaders;
  private EventBuilder event;
  private FaceMatchClient faceMatch;
  private final List<FaceMatchSettings> faceMatchSettings = new ArrayList<>();

  @BeforeEach
  void setUp() throws IOException {
    config.put(ScanovateAuthenticatorFactory.FLOW_ID, "3659");
    config.put(ScanovateAuthenticatorFactory.LINK_PARAMS, "{\"country\": \"country\"}");
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE,
        """
        {"default": [
          {"type": "minValue", "minValue": "0.5", "process": "biometric_match",
           "attributePath": "/score", "errorMsg": "scanovateScoringError"}
        ]}
        """);
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE,
        """
        {"default": [
          {"UserAttribute": "firstName", "process": "ocr", "attributePath": "/firstName",
           "type": "text"}
        ]}
        """);
    authNotes.put("country", "Spain");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID, "123456789");

    context = mock(AuthenticationFlowContext.class);
    AuthenticatorConfigModel configModel = mock(AuthenticatorConfigModel.class);
    when(configModel.getConfig()).thenReturn(config);
    when(context.getAuthenticatorConfig()).thenReturn(configModel);

    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> authNotes.get(i.getArgument(0)));
    doAnswer(i -> authNotes.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), anyString());
    doAnswer(i -> authNotes.remove(i.getArgument(0))).when(authSession).removeAuthNote(anyString());
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    when(rootSession.getId()).thenReturn("root-session");
    when(authSession.getParentSession()).thenReturn(rootSession);
    when(context.getAuthenticationSession()).thenReturn(authSession);

    event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    KeycloakUriInfo uriInfo = mock(KeycloakUriInfo.class);
    when(uriInfo.getQueryParameters()).thenReturn(queryParams);
    when(uriInfo.getBaseUri()).thenReturn(URI.create("https://kc/"));
    when(context.getUriInfo()).thenReturn(uriInfo);
    RealmModel realm = mock(RealmModel.class);
    when(realm.getName()).thenReturn("r");
    when(context.getRealm()).thenReturn(realm);
    HttpRequest httpRequest = mock(HttpRequest.class);
    when(httpRequest.getDecodedFormParameters()).thenReturn(formParams);
    httpHeaders = mock(HttpHeaders.class);
    when(httpHeaders.getMediaType()).thenReturn(MediaType.APPLICATION_FORM_URLENCODED_TYPE);
    when(httpRequest.getHttpHeaders()).thenReturn(httpHeaders);
    when(context.getHttpRequest()).thenReturn(httpRequest);
    when(context.generateAccessCode()).thenReturn("c");
    when(context.getActionUrl("c")).thenReturn(URI.create(ACTION_URL));

    form = mock(LoginFormsProvider.class, RETURNS_SELF);
    when(form.createForm(anyString())).thenReturn(Response.ok().build());
    when(context.form()).thenReturn(form);

    execution = mock(AuthenticationExecutionModel.class);
    when(context.getExecution()).thenReturn(execution);

    client = mock(ScanovateClient.class);
    when(client.fetchAccessToken()).thenReturn("jwt");
    when(client.createSessionLink(eq("jwt"), any()))
        .thenReturn(new SessionLink(FLOW_URL, "proc-1"));
    when(client.fetchResultsForProcess("proc-1")).thenReturn(json(SUCCESSFUL_RESULTS));
    livenessStore = new LivenessSessionsTest.MemoryStore();
    livenessSessions = new LivenessSessions(livenessStore, millis -> {});
    captureUploads = new CaptureUploads(new LivenessSessionsTest.MemoryStore());
    faceMatch = mock(FaceMatchClient.class);
    faceMatchSettings.clear();
    authenticator =
        new ScanovateAuthenticator(
            ignored -> client,
            ignored -> livenessSessions,
            settings -> {
              faceMatchSettings.add(settings);
              return faceMatch;
            },
            ignored -> captureUploads);

    KeycloakSession session = mock(KeycloakSession.class);
    SecurityHeadersProvider headersProvider = mock(SecurityHeadersProvider.class);
    securityHeaders = mock(SecurityHeadersOptions.class, RETURNS_SELF);
    when(headersProvider.options()).thenReturn(securityHeaders);
    when(session.getProvider(SecurityHeadersProvider.class)).thenReturn(headersProvider);
    when(context.getSession()).thenReturn(session);

    utils = mockStatic(Utils.class);
  }

  @AfterEach
  void tearDown() {
    utils.close();
  }

  @Test
  void alreadyVerifiedUserSkipsVerification() throws IOException {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(STATUS_NOTE)).thenReturn("VERIFIED");
    when(context.getUser()).thenReturn(user);

    authenticator.authenticate(context);

    verify(context).success();
    verify(client, never()).fetchAccessToken();
  }

  @Test
  void authenticateRedirectsToBTrustFlow() throws IOException {
    authenticator.authenticate(context);

    ArgumentCaptor<LinkRequest> request = ArgumentCaptor.forClass(LinkRequest.class);
    verify(client).createSessionLink(eq("jwt"), request.capture());
    assertEquals(3659, request.getValue().flowId());
    assertEquals(
        "https://kc/realms/r/scanovate/return?flow=authenticate&session_code=c&execution=e",
        request.getValue().redirectUrl());
    assertEquals("123456789", request.getValue().idNumber());
    assertEquals(Map.of("country", "Spain"), request.getValue().params());
    assertEquals(SaveOption.DEFAULT, request.getValue().saveOption());
    assertEquals("proc-1", authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));

    ArgumentCaptor<Response> challenge = ArgumentCaptor.forClass(Response.class);
    verify(context).challenge(challenge.capture());
    assertEquals(303, challenge.getValue().getStatus());
    assertEquals(URI.create(FLOW_URL), challenge.getValue().getLocation());
  }

  @Test
  void returnFromBTrustShowsConfirmation() {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    assertEquals("JUAN", authNotes.get("firstName"));
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
    assertNull(authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_STORED_ATTRIBUTES,
            List.of(Map.of("key", "firstName", "value", "JUAN", "type", "text")));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_DOCUMENT_TYPE, "default");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    verify(context, never()).success();
  }

  @Test
  void confirmationShowsTheDocumentType() {
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(form).setAttribute(ScanovateAuthenticator.FTL_DOCUMENT_TYPE, "philSysID");
  }

  @Test
  void returnHandledAsRefreshIsProcessedInAuthenticate() throws IOException {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    queryParams.add(ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM, "proc-1");

    authenticator.authenticate(context);

    verify(client).fetchResultsForProcess("proc-1");
    verify(client, never()).createSessionLink(any(), any());
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void unknownProcessIdInQueryStartsANewSession() throws IOException {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    queryParams.add(ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM, "someone-else");

    authenticator.authenticate(context);

    verify(client, never()).fetchResultsForProcess(anyString());
    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void returnWithoutStoredAttributesSucceedsDirectly() {
    config.remove(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE);
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(context).success();
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
  }

  @Test
  void confirmationCompletesTheVerification() {
    authNotes.put(STATUS_NOTE, "VERIFIED");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context).success();
  }

  @Test
  void forgedConfirmationDoesNotSucceed() throws IOException {
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context, never()).success();
    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void failedRuleShowsRetryableError() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": true, \"errorCode\": 0,"
                    + " \"resultsList\": [{\"process\": \"biometric_match\", \"success\": true,"
                    + " \"score\": 0.1}]}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertNull(authNotes.get(STATUS_NOTE));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ERROR, "scanovateScoringError");
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 2);
    verify(form).createForm(ScanovateAuthenticator.ERROR_FORM);
    verify(context, never()).success();
  }

  @Test
  void flowFailureIsMappedToItsMessage() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 1026}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_ERROR, ScanovateError.DOCUMENT_AUTHENTICATION.messageKey());
  }

  @Test
  void lastAllowedAttemptRejectsTheVoter() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 1030}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");

    authenticator.action(context);

    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 0);
  }

  @Test
  void retryStartsANewSession() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void retryAfterMaxAttemptsIsRejected() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verify(client, never()).createSessionLink(any(), any());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
  }

  @Test
  void autoCompleteModeFetchesResultsWithoutRedirect() throws IOException {
    config.put(ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.AUTO_COMPLETE.value());

    authenticator.authenticate(context);

    verify(client).fetchResultsForProcess("proc-1");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void missingFlowIdIsAnInternalError() throws IOException {
    config.remove(ScanovateAuthenticatorFactory.FLOW_ID);

    authenticator.authenticate(context);

    verify(client, never()).fetchAccessToken();
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
  }

  @Test
  void apiFailureIsARetryableInternalError() throws IOException {
    when(client.fetchAccessToken()).thenThrow(new IOException("down"));

    authenticator.authenticate(context);

    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
  }

  @Test
  void documentTypeWithoutRulesIsRejected() {
    config.put(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE, "{\"passport\": []}");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "seamanBook");
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(context, never()).success();
    assertNull(authNotes.get(STATUS_NOTE));
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
  }

  @Test
  void alternativeExecutionFallsThroughOnError() throws IOException {
    when(execution.isAlternative()).thenReturn(true);
    when(client.fetchAccessToken()).thenThrow(new IOException("down"));

    authenticator.authenticate(context);

    verify(context).attempted();
    verify(form, never()).createForm(anyString());
  }

  @Test
  void linkParamsSkipsMissingAuthNotes() throws ScanovateException {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    config.put(
        ScanovateAuthenticatorFactory.LINK_PARAMS,
        "{\"country\": \"country\", \"x\": \"missing\"}");
    assertEquals(
        Map.of("country", "Spain"), ScanovateAuthenticator.linkParams(config, authSession));
  }

  @Test
  void linkParamsRejectsNonObject() {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    config.put(ScanovateAuthenticatorFactory.LINK_PARAMS, "[]");
    assertThrows(
        ScanovateException.class, () -> ScanovateAuthenticator.linkParams(config, authSession));
  }

  private void embedded() {
    config.put(ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.EMBEDDED.value());
    config.put(
        ScanovateAuthenticatorFactory.CAPTURE_SIDES,
        "{\"philSysID\": [\"front\"], \"default\": [\"front\", \"back\"]}");
  }

  /**
   * The capture token of the rendered capture page. Tests that post a capture without rendering the
   * page first get one as if it had been rendered.
   */
  private String captureToken() {
    String token = authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE);
    if (token == null) {
      token = captureUploads.create();
      authNotes.put(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE, token);
    }
    return token;
  }

  /**
   * Uploads the parts through the capture endpoint, which refuses those of an unrecognised format,
   * then posts the capture form.
   */
  private void upload(Map<String, byte[]> parts) {
    String token = captureToken();
    parts.forEach((part, content) -> captureUploads.store(token, part, content));
    if (!formParams.containsKey(ScanovateAuthenticator.FORM_ACTION_PARAM)) {
      formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CAPTURE.value());
    }
  }

  private void capture(byte[] front, byte[] back, byte[] face, byte[] video) {
    upload(CaptureMediaTest.parts(front, back, face, video));
  }

  private void verifyCapturePage(List<String> sides, int attemptsLeft) {
    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_SCANOVATE,
            Map.of(
                ScanovateAuthenticator.FTL_UPLOAD,
                Map.of(
                    ScanovateAuthenticator.FTL_UPLOAD_URL,
                    "/realms/r/scanovate/capture",
                    ScanovateAuthenticator.FTL_UPLOAD_TOKEN,
                    authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE)),
                ScanovateAuthenticator.FTL_DOCUMENT_TYPE,
                authNotes.getOrDefault(
                    ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "default"),
                ScanovateAuthenticator.FTL_SIDES,
                sides,
                ScanovateAuthenticator.FTL_VIDEO_SECONDS,
                5,
                ScanovateAuthenticator.FTL_ATTEMPTS_LEFT,
                attemptsLeft,
                ScanovateAuthenticator.FTL_MAX_ATTEMPTS,
                3));
    verify(form).createForm(ScanovateAuthenticator.CAPTURE_FORM);
  }

  private void verifyCaptureRejected() throws IOException {
    verify(form).setError(ScanovateError.CAPTURE_INVALID.messageKey());
    verify(form).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    verify(client, never()).uploadMedia(any(), any(), any());
    verify(client, never()).fetchResultsForProcess(anyString());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertEquals("proc-1", authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
  }

  @Test
  void embeddedModeRendersTheCapturePage() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");

    authenticator.authenticate(context);

    verify(client).createSessionLink(eq("jwt"), any());
    assertEquals("proc-1", authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
    verifyCapturePage(List.of("FRONT"), 2);
    verify(client, never()).fetchResultsForProcess(anyString());
  }

  @Test
  void embeddedModeDefaultsToBothSides() {
    config.put(ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.EMBEDDED.value());

    authenticator.authenticate(context);

    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void refreshingThePendingCaptureRendersItAgain() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.authenticate(context);

    verify(client, never()).createSessionLink(any(), any());
    assertEquals("proc-1", authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void malformedCaptureSidesIsAnInternalError() throws IOException {
    embedded();
    config.put(ScanovateAuthenticatorFactory.CAPTURE_SIDES, "{\"default\": [\"top\"]}");

    authenticator.authenticate(context);

    verify(client, never()).createSessionLink(any(), any());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
  }

  @Test
  void malformedCaptureSidesIsIgnoredOutsideEmbeddedMode() throws IOException {
    config.put(ScanovateAuthenticatorFactory.CAPTURE_SIDES, "nope");

    authenticator.authenticate(context);

    verify(context).challenge(any());
    verify(form, never()).createForm(anyString());
  }

  @Test
  void captureUploadsTheMediaAndShowsTheConfirmation() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    ArgumentCaptor<CaptureMedia> media = ArgumentCaptor.forClass(CaptureMedia.class);
    verify(client).uploadMedia(eq("jwt"), eq("proc-1"), media.capture());
    assertEquals(
        List.of(
            MediaKind.FRONT_IMAGE,
            MediaKind.BACK_IMAGE,
            MediaKind.FACE_IMAGE,
            MediaKind.SCAN_VIDEO),
        List.copyOf(media.getValue().files().keySet()));
    verify(client).fetchResultsForProcess("proc-1");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
    assertNull(authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
  }

  @Test
  void captureOfSingleSidedDocumentDoesNotNeedTheBack() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    config.put(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE, "{\"default\": []}");
    capture(JPEG, null, JPEG, WEBM);

    authenticator.action(context);

    ArgumentCaptor<CaptureMedia> media = ArgumentCaptor.forClass(CaptureMedia.class);
    verify(client).uploadMedia(eq("jwt"), eq("proc-1"), media.capture());
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.FACE_IMAGE, MediaKind.SCAN_VIDEO),
        List.copyOf(media.getValue().files().keySet()));
  }

  @Test
  void captureFailingTheRulesCountsAsAnAttempt() throws IOException {
    embedded();
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 1026}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_ERROR, ScanovateError.DOCUMENT_AUTHENTICATION.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 2);
  }

  @Test
  void captureWithoutTheBackIsRejectedWithoutCountingAnAttempt() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, null, JPEG, WEBM);

    authenticator.action(context);

    verifyCaptureRejected();
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void captureWithoutVideoIsRejected() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, null);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void captureWithWrongMagicBytesIsRejected() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, PNG, WEBM);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void oversizedCaptureIsRejected() throws IOException {
    embedded();
    config.put(ScanovateAuthenticatorFactory.MAX_VIDEO_BYTES, "4");
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void multipartCapturePostIsRejectedWithoutReadingTheForm() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    when(httpHeaders.getMediaType()).thenReturn(MediaType.MULTIPART_FORM_DATA_TYPE);

    authenticator.action(context);

    verify(context.getHttpRequest(), never()).getDecodedFormParameters();
    verifyCaptureRejected();
  }

  @Test
  void multipartPostWithoutPendingCaptureStartsOver() throws IOException {
    embedded();
    when(httpHeaders.getMediaType())
        .thenReturn(MediaType.valueOf("multipart/form-data; boundary=x"));

    authenticator.action(context);

    verify(context.getHttpRequest(), never()).getDecodedFormParameters();
    verify(client).createSessionLink(eq("jwt"), any());
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void uploadsAreDiscardedOnceTheCaptureIsProcessed() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);
    String token = captureToken();

    authenticator.action(context);

    assertEquals(Optional.empty(), captureUploads.parts(token));
    assertNull(authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE));
  }

  @Test
  void rejectedCaptureGetsAFreshUploadToken() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, null, JPEG, WEBM);
    String token = captureToken();

    authenticator.action(context);

    verifyCaptureRejected();
    assertEquals(Optional.empty(), captureUploads.parts(token));
    String fresh = authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE);
    assertNotNull(fresh);
    assertNotEquals(token, fresh);
    assertEquals(Optional.of(Map.of()), captureUploads.parts(fresh));
  }

  @Test
  void captureUploadFailureIsARetryableInternalError() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);
    doThrow(new IOException("down")).when(client).uploadMedia(any(), any(), any());

    authenticator.action(context);

    verify(client, never()).fetchResultsForProcess(anyString());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 3);
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertNull(authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
  }

  @Test
  void captureWithoutPendingSessionStartsOver() throws IOException {
    embedded();
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    verify(client, never()).uploadMedia(any(), any(), any());
    verify(client).createSessionLink(eq("jwt"), any());
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void captureOutsideEmbeddedModeStartsOver() throws IOException {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    verify(client, never()).uploadMedia(any(), any(), any());
    verify(client).createSessionLink(eq("jwt"), any());
    verify(context).challenge(any());
    verify(form, never()).createForm(anyString());
  }

  private void verifyMaxRetriesWithoutNewSession() throws IOException {
    verify(client, never()).createSessionLink(any(), any());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 0);
    verify(form, never()).createForm(ScanovateAuthenticator.CAPTURE_FORM);
  }

  @Test
  void refreshAfterMaxRetriesDoesNotCreateASession() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");

    authenticator.authenticate(context);

    verifyMaxRetriesWithoutNewSession();
    verify(context, never()).success();
  }

  @Test
  void embeddedRefreshAfterMaxRetriesDoesNotCreateASession() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");

    authenticator.authenticate(context);

    verifyMaxRetriesWithoutNewSession();
  }

  @Test
  void autoCompleteAfterMaxRetriesDoesNotCreateASession() throws IOException {
    config.put(ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.AUTO_COMPLETE.value());
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "4");

    authenticator.authenticate(context);

    verifyMaxRetriesWithoutNewSession();
    verify(client, never()).fetchResultsForProcess(anyString());
  }

  @Test
  void captureWithoutSessionAfterMaxRetriesDoesNotCreateASession() throws IOException {
    embedded();
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    capture(JPEG, JPEG, JPEG, WEBM);

    authenticator.action(context);

    verifyMaxRetriesWithoutNewSession();
    verify(client, never()).uploadMedia(any(), any(), any());
  }

  @Test
  void forgedConfirmationAfterMaxRetriesDoesNotCreateASession() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verifyMaxRetriesWithoutNewSession();
    verify(context, never()).success();
  }

  @Test
  void sessionIsStillCreatedBeforeTheLastAttempt() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");

    authenticator.authenticate(context);

    verify(client).createSessionLink(eq("jwt"), any());
  }

  private static final byte[] FRONT_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'f'};
  private static final byte[] HOLDING_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'h'};

  private static FaceComparison match(double similarity) {
    return new FaceComparison(true, similarity, 0.67, 0, 0);
  }

  private void liveness() {
    embedded();
    config.put(ScanovateAuthenticatorFactory.FACE_CAPTURE, FaceCapture.LIVENESS.value());
    config.putAll(LivenessSettingsTest.livenessConfig());
  }

  /** Starts the liveness capture and returns the token its page gets for Liveness Plus. */
  private String startLiveness() {
    authenticator.authenticate(context);
    String token = authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE);
    assertNotNull(token);
    return token;
  }

  private void livenessResult(String token, String status, boolean passed) {
    assertEquals(
        LivenessSessions.RecordOutcome.RECORDED,
        livenessSessions.record(
            token, "callback-secret", json(LivenessSessionsTest.result(status, passed, JPEG))));
  }

  private void livenessCapture(byte[] front, byte[] back, byte[] holding) {
    upload(CaptureMediaTest.livenessParts(front, back, holding));
  }

  /** A voter whose liveness passed posts the photos of the document and the one holding it. */
  private String passedLivenessCapture() {
    liveness();
    String token = startLiveness();
    livenessResult(token, "completed", true);
    livenessCapture(FRONT_JPEG, JPEG, HOLDING_JPEG);
    return token;
  }

  private void verifyFailedAttempt(String messageKey) throws IOException {
    verify(client, never()).uploadMedia(any(), any(), any());
    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ERROR, messageKey);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
    assertNull(authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
    assertNull(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE));
  }

  @Test
  void livenessCapturePageGetsTheLivenessApiAndItsToken() throws IOException {
    liveness();

    String token = startLiveness();

    @SuppressWarnings("unchecked")
    ArgumentCaptor<Map<String, Object>> capture = ArgumentCaptor.forClass(Map.class);
    verify(form).setAttribute(eq(ScanovateAuthenticator.FTL_SCANOVATE), capture.capture());
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_LIVENESS_URL,
            "https://liveness.example.com:8443/liveness",
            ScanovateAuthenticator.FTL_LIVENESS_TOKEN,
            token,
            ScanovateAuthenticator.FTL_LIVENESS_CASE_ID,
            "proc-1"),
        capture.getValue().get(ScanovateAuthenticator.FTL_LIVENESS));
    verify(securityHeaders, never()).allowFrameSrc(anyString());
    verify(form).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    assertEquals(true, livenessSessions.verify(token, "proc-1", "callback-secret"));
  }

  @Test
  void photoCapturePageHasNoLiveness() {
    embedded();

    authenticator.authenticate(context);

    verifyCapturePage(List.of("FRONT", "BACK"), 3);
    assertNull(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE));
  }

  @Test
  void refreshingTheLivenessCaptureReplacesItsToken() {
    liveness();
    String first = startLiveness();

    authenticator.authenticate(context);

    String second = authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE);
    assertNotNull(second);
    assertEquals(false, first.equals(second));
    assertEquals(false, livenessSessions.verify(first, "proc-1", "callback-secret"));
  }

  private void verifyLivenessConfigurationError() throws IOException {
    authenticator.authenticate(context);

    verify(client, never()).createSessionLink(any(), any());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
  }

  @Test
  void livenessWithoutItsSecretIsAnInternalError() throws IOException {
    liveness();
    config.remove(ScanovateAuthenticatorFactory.LIVENESS_SECRET);

    verifyLivenessConfigurationError();
  }

  @Test
  void livenessWithoutFaceMatchIsAnInternalError() throws IOException {
    liveness();
    config.remove(ScanovateAuthenticatorFactory.FACE_MATCH_URL);

    verifyLivenessConfigurationError();
  }

  @Test
  void malformedFaceMatchMinimumIsAnInternalError() throws IOException {
    liveness();
    config.put(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "high");

    verifyLivenessConfigurationError();
  }

  @Test
  void matchingFacesUploadOnlyTheDocumentToBTrust() throws IOException {
    String token = passedLivenessCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.9));
    when(faceMatch.compare(HOLDING_JPEG, JPEG)).thenReturn(match(0.8));

    authenticator.action(context);

    ArgumentCaptor<CaptureMedia> media = ArgumentCaptor.forClass(CaptureMedia.class);
    verify(client).uploadMedia(eq("jwt"), eq("proc-1"), media.capture());
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE),
        List.copyOf(media.getValue().files().keySet()));
    assertArrayEquals(FRONT_JPEG, media.getValue().files().get(MediaKind.FRONT_IMAGE).content());
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.9000");
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_HOLDING, "0.8000");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    assertNull(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE));
    assertEquals(false, livenessSessions.verify(token, "proc-1", "callback-secret"));
  }

  @Test
  void faceMatchUsesTheConfiguredServiceAndMinimum() throws IOException {
    passedLivenessCapture();
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "driversLicense");
    config.put(
        ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY,
        "{\"driversLicense\": 0.9, \"default\": 0.5}");
    when(faceMatch.compare(any(), any())).thenReturn(match(0.85));

    authenticator.action(context);

    assertEquals(
        new FaceMatchSettings(URI.create("http://face-match:3000"), 0.9), faceMatchSettings.get(0));
    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
  }

  @Test
  void facePartsPostedByTheBrowserAreNeverUploaded() throws IOException {
    passedLivenessCapture();
    upload(CaptureMediaTest.parts(null, null, JPEG, WEBM));
    when(faceMatch.compare(any(), any())).thenReturn(match(0.9));

    authenticator.action(context);

    ArgumentCaptor<CaptureMedia> media = ArgumentCaptor.forClass(CaptureMedia.class);
    verify(client).uploadMedia(eq("jwt"), eq("proc-1"), media.capture());
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE),
        List.copyOf(media.getValue().files().keySet()));
  }

  @Test
  void documentPhotoOfAnotherPersonCountsAsAnAttempt() throws IOException {
    passedLivenessCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.3));
    when(faceMatch.compare(HOLDING_JPEG, JPEG)).thenReturn(match(0.9));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.3000");
  }

  @Test
  void anotherPersonHoldingTheDocumentCountsAsAnAttempt() throws IOException {
    passedLivenessCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.9));
    when(faceMatch.compare(HOLDING_JPEG, JPEG)).thenReturn(match(0.2));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
  }

  @Test
  void faceNotFoundCountsAsAnAttempt() throws IOException {
    passedLivenessCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG))
        .thenReturn(new FaceComparison(false, 0.0, 0.67, 1101, 0));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_NOT_FOUND.messageKey());
    verify(faceMatch, never()).compare(HOLDING_JPEG, JPEG);
  }

  @Test
  void unreachableFaceMatchIsARetryableInternalError() throws IOException {
    passedLivenessCapture();
    when(faceMatch.compare(any(), any())).thenThrow(new IOException("down"));

    authenticator.action(context);

    verify(client, never()).uploadMedia(any(), any(), any());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
  }

  @Test
  void failedLivenessCountsAsAnAttempt() throws IOException {
    liveness();
    String token = startLiveness();
    livenessResult(token, "completed", false);
    livenessCapture(FRONT_JPEG, JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.LIVENESS_FAILED.messageKey());
    verify(faceMatch, never()).compare(any(), any());
  }

  @Test
  void missingLivenessResultIsARetryableInternalError() throws IOException {
    liveness();
    startLiveness();
    livenessCapture(FRONT_JPEG, JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verify(client, never()).uploadMedia(any(), any(), any());
    verify(faceMatch, never()).compare(any(), any());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
  }

  @Test
  void invalidDocumentIsRejectedBeforeWaitingForLiveness() throws IOException {
    liveness();
    String token = startLiveness();
    livenessResult(token, "completed", true);
    livenessCapture(PNG, JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verify(form).setError(ScanovateError.CAPTURE_INVALID.messageKey());
    verify(client, never()).uploadMedia(any(), any(), any());
    verify(faceMatch, never()).compare(any(), any());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
  }

  @Test
  void livenessCaptureWithoutThePhotoHoldingTheDocumentIsRejected() throws IOException {
    liveness();
    String token = startLiveness();
    livenessResult(token, "completed", true);
    livenessCapture(FRONT_JPEG, JPEG, null);

    authenticator.action(context);

    verify(form).setError(ScanovateError.CAPTURE_INVALID.messageKey());
    verify(faceMatch, never()).compare(any(), any());
  }
}
