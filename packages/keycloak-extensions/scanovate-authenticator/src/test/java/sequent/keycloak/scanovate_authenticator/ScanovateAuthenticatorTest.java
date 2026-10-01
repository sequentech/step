// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_SELF;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.PNG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.WEBM;
import static sequent.keycloak.scanovate_authenticator.OcrResultsTest.PASSPORT_RESPONSE;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

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
  private static final String STATUS_NOTE = "sequent.read-only.id-card-number-validated";
  private static final byte[] FRONT_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'f'};
  private static final byte[] BACK_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'b'};
  private static final byte[] HOLDING_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'h'};

  private final Map<String, String> config = new HashMap<>();
  private final Map<String, String> authNotes = new HashMap<>();
  private final MultivaluedHashMap<String, String> formParams = new MultivaluedHashMap<>();

  private AuthenticationFlowContext context;
  private LoginFormsProvider form;
  private AuthenticationExecutionModel execution;
  private ScanovateAuthenticator authenticator;
  private MockedStatic<Utils> utils;
  private LivenessSessions livenessSessions;
  private CaptureUploads captureUploads;
  private HttpHeaders httpHeaders;
  private SecurityHeadersOptions securityHeaders;
  private EventBuilder event;
  private FaceMatchClient faceMatch;
  private OcrClient ocr;
  private final List<FaceMatchSettings> faceMatchSettings = new ArrayList<>();
  private final List<OcrSettings> ocrSettings = new ArrayList<>();

  @BeforeEach
  void setUp() throws IOException {
    config.putAll(LivenessSettingsTest.livenessConfig());
    config.put(ScanovateAuthenticatorFactory.OCR_URL, "http://ocr:5040");
    config.put(
        ScanovateAuthenticatorFactory.CAPTURE_SIDES,
        "{\"philSysID\": [\"front\"], \"default\": [\"front\", \"back\"]}");
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE,
        """
        {"default": [
          {"type": "equalValue", "equalValue": "PHL", "process": "ocr",
           "attributePath": "/issuing_country_code", "errorMsg": "scanovateAttributesError"}
        ]}
        """);
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE,
        """
        {"default": [
          {"UserAttribute": "firstName", "process": "ocr",
           "attributePath": "/first_name_english", "type": "text"}
        ]}
        """);

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

    form = mock(LoginFormsProvider.class, RETURNS_SELF);
    when(form.createForm(anyString())).thenReturn(Response.ok().build());
    when(context.form()).thenReturn(form);

    execution = mock(AuthenticationExecutionModel.class);
    when(context.getExecution()).thenReturn(execution);

    livenessSessions = new LivenessSessions(new LivenessSessionsTest.MemoryStore(), millis -> {});
    captureUploads = new CaptureUploads(new LivenessSessionsTest.MemoryStore());
    faceMatch = mock(FaceMatchClient.class);
    when(faceMatch.compare(any(), any())).thenReturn(match(0.9));
    ocr = mock(OcrClient.class);
    when(ocr.recognize(anyString(), any(), anyString())).thenReturn(json(PASSPORT_RESPONSE));
    faceMatchSettings.clear();
    ocrSettings.clear();
    authenticator =
        new ScanovateAuthenticator(
            settings -> {
              ocrSettings.add(settings);
              return ocr;
            },
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

  private static FaceComparison match(double similarity) {
    return new FaceComparison(true, similarity, 0.67, 0, 0);
  }

  private String caseId() {
    return authNotes.get(ScanovateAuthenticator.CASE_ID_NOTE);
  }

  /** Renders the capture page and returns the token its page gets for Liveness Plus. */
  private String startCapture() {
    authenticator.authenticate(context);
    String token = authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE);
    assertNotNull(token);
    return token;
  }

  private void livenessResult(String token, boolean passed) {
    assertEquals(
        LivenessSessions.RecordOutcome.RECORDED,
        livenessSessions.record(
            token,
            "callback-secret",
            json(
                LivenessSessionsTest.result("completed", passed, JPEG)
                    .replace("\"proc-1\"", "\"" + caseId() + "\""))));
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
   * Uploads the parts through the capture endpoint, which refuses unknown parts and those of an
   * unrecognised format, then posts the capture form.
   */
  private void upload(Map<String, byte[]> parts) {
    String token = captureToken();
    parts.forEach((part, content) -> captureUploads.store(token, part, content));
    if (!formParams.containsKey(ScanovateAuthenticator.FORM_ACTION_PARAM)) {
      formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CAPTURE.value());
    }
  }

  private void capture(byte[] front, byte[] back, byte[] holding) {
    upload(CaptureMediaTest.parts(front, back, holding));
  }

  /** A voter whose liveness passed posts the photos of the document and the one holding it. */
  private String passedCapture() {
    String token = startCapture();
    livenessResult(token, true);
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);
    return token;
  }

  @SuppressWarnings("unchecked")
  private Map<String, Object> capturePage() {
    ArgumentCaptor<Map<String, Object>> page = ArgumentCaptor.forClass(Map.class);
    verify(form).setAttribute(eq(ScanovateAuthenticator.FTL_SCANOVATE), page.capture());
    verify(form).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    return page.getValue();
  }

  private void verifyCapturePage(List<String> sides, int attemptsLeft) {
    Map<String, Object> page = capturePage();
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_UPLOAD_URL,
            "/realms/r/identity-verification/capture",
            ScanovateAuthenticator.FTL_UPLOAD_TOKEN,
            authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE)),
        page.get(ScanovateAuthenticator.FTL_UPLOAD));
    assertEquals(
        authNotes.getOrDefault(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "default"),
        page.get(ScanovateAuthenticator.FTL_DOCUMENT_TYPE));
    assertEquals(sides, page.get(ScanovateAuthenticator.FTL_SIDES));
    assertEquals(5, page.get(ScanovateAuthenticator.FTL_VIDEO_SECONDS));
    assertEquals(attemptsLeft, page.get(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT));
    assertEquals(3, page.get(ScanovateAuthenticator.FTL_MAX_ATTEMPTS));
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_LIVENESS_URL,
            "https://liveness.example.com:8443/liveness",
            ScanovateAuthenticator.FTL_LIVENESS_TOKEN,
            authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE),
            ScanovateAuthenticator.FTL_LIVENESS_CASE_ID,
            caseId()),
        page.get(ScanovateAuthenticator.FTL_LIVENESS));
  }

  private void verifyError(ScanovateError error, boolean canRetry) {
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ERROR, error.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, canRetry);
    verify(form).createForm(ScanovateAuthenticator.ERROR_FORM);
  }

  private void verifyFailedAttempt(String messageKey) throws IOException {
    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ERROR, messageKey);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 2);
    assertNull(caseId());
    assertNull(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE));
    assertNull(authNotes.get(STATUS_NOTE));
    verify(context, never()).success();
  }

  private void verifyCaptureRejected() throws IOException {
    verify(form).setError(ScanovateError.CAPTURE_INVALID.messageKey());
    // Once when the capture started, and again with the error
    verify(form, times(2)).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    verify(faceMatch, never()).compare(any(), any());
    verify(ocr, never()).recognize(anyString(), any(), anyString());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
  }

  @Test
  void alreadyVerifiedUserSkipsVerification() {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(STATUS_NOTE)).thenReturn("VERIFIED");
    when(context.getUser()).thenReturn(user);

    authenticator.authenticate(context);

    verify(context).success();
    verify(form, never()).createForm(anyString());
  }

  @Test
  void authenticateRendersTheCapturePage() throws IOException {
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");

    String token = startCapture();

    assertNotNull(caseId());
    verifyCapturePage(List.of("FRONT"), 2);
    verify(securityHeaders, never()).allowFrameSrc(anyString());
    assertEquals(true, livenessSessions.verify(token, caseId(), "callback-secret"));
    verify(ocr, never()).recognize(anyString(), any(), anyString());
  }

  @Test
  void capturePageDefaultsToBothSides() {
    config.remove(ScanovateAuthenticatorFactory.CAPTURE_SIDES);

    authenticator.authenticate(context);

    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void everyCaptureHasItsOwnCaseId() {
    startCapture();
    String first = caseId();
    authNotes.remove(ScanovateAuthenticator.CASE_ID_NOTE);

    startCapture();

    assertNotNull(first);
    assertNotEquals(first, caseId());
  }

  @Test
  void refreshingThePendingCaptureRendersItAgainWithANewLivenessToken() {
    String first = startCapture();
    String caseId = caseId();

    authenticator.authenticate(context);

    assertEquals(caseId, caseId());
    String second = authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE);
    assertNotEquals(first, second);
    assertEquals(false, livenessSessions.verify(first, caseId, "callback-secret"));
    assertEquals(true, livenessSessions.verify(second, caseId, "callback-secret"));
  }

  @Test
  void postsWithoutAFormActionShowThePendingCapture() {
    startCapture();
    String caseId = caseId();

    authenticator.action(context);

    assertEquals(caseId, caseId());
    verify(form, times(2)).createForm(ScanovateAuthenticator.CAPTURE_FORM);
  }

  @Test
  void configurationErrorsAreInternalErrors() {
    List<Map.Entry<String, String>> broken =
        List.of(
            Map.entry(ScanovateAuthenticatorFactory.CAPTURE_SIDES, "{\"default\": [\"top\"]}"),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_SECRET, ""),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "high"),
            Map.entry(ScanovateAuthenticatorFactory.OCR_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.OCR_TYPES, "{\"iBP\": \"regula\"}"));
    for (Map.Entry<String, String> setting : broken) {
      Map<String, String> original = new HashMap<>(config);
      config.put(setting.getKey(), setting.getValue());
      form = mock(LoginFormsProvider.class, RETURNS_SELF);
      when(context.form()).thenReturn(form);

      authenticator.authenticate(context);

      verify(form)
          .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
      verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
      verify(form, never()).createForm(ScanovateAuthenticator.CAPTURE_FORM);
      assertNull(caseId(), setting.getKey());
      config.clear();
      config.putAll(original);
    }
  }

  @Test
  void alternativeExecutionFallsThroughOnError() {
    when(execution.isAlternative()).thenReturn(true);
    config.put(ScanovateAuthenticatorFactory.OCR_URL, "");

    authenticator.authenticate(context);

    verify(context).attempted();
    verify(form, never()).createForm(anyString());
  }

  @Test
  void matchingFacesReadTheDocumentOnPremiseAndShowTheConfirmation() throws IOException {
    String token = passedCapture();
    String caseId = caseId();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.9));
    when(faceMatch.compare(HOLDING_JPEG, JPEG)).thenReturn(match(0.8));

    authenticator.action(context);

    verify(ocr).recognize("passport", FRONT_JPEG, caseId + "-front");
    verify(ocr).recognize("passport", BACK_JPEG, caseId + "-back");
    assertEquals(List.of(new OcrSettings(URI.create("http://ocr:5040"), "passport")), ocrSettings);
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_CASE_ID, caseId);
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.9000");
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_HOLDING, "0.8000");
    assertEquals("JUAN", authNotes.get("firstName"));
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
    assertNull(caseId());
    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_STORED_ATTRIBUTES,
            List.of(Map.of("key", "firstName", "value", "JUAN", "type", "text")));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_DOCUMENT_TYPE, "default");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    verify(context, never()).success();
    assertNull(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE));
    assertEquals(false, livenessSessions.verify(token, caseId, "callback-secret"));
  }

  @Test
  void singleSidedDocumentsOnlyReadTheFront() throws IOException {
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    String token = startCapture();
    livenessResult(token, true);
    capture(FRONT_JPEG, null, HOLDING_JPEG);

    authenticator.action(context);

    verify(ocr).recognize(eq("passport"), eq(FRONT_JPEG), anyString());
    verify(ocr, never()).recognize(anyString(), eq(BACK_JPEG), anyString());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_DOCUMENT_TYPE, "philSysID");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void theDocumentTypeSelectsItsOcrType() throws IOException {
    config.put(
        ScanovateAuthenticatorFactory.OCR_TYPES,
        "{\"philippinePassport\": \"passport\", \"default\": \"regula\"}");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "driversLicense");
    passedCapture();

    authenticator.action(context);

    verify(ocr).recognize(eq("regula"), eq(FRONT_JPEG), anyString());
    verify(ocr).recognize(eq("regula"), eq(BACK_JPEG), anyString());
  }

  @Test
  void facePhotosPostedByTheBrowserAreNeverRead() throws IOException {
    passedCapture();
    upload(Map.of("face", JPEG, "video", WEBM));

    authenticator.action(context);

    verify(ocr).recognize(anyString(), eq(FRONT_JPEG), anyString());
    verify(ocr).recognize(anyString(), eq(BACK_JPEG), anyString());
    verify(ocr, never()).recognize(anyString(), eq(JPEG), anyString());
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void anUnreadableDocumentCountsAsAnAttempt() throws IOException {
    when(ocr.recognize(anyString(), eq(BACK_JPEG), anyString()))
        .thenReturn(
            json(
                """
                {"status": "completed", "back": {"processing_result":
                 {"status": "card_not_detected"}}}
                """));
    passedCapture();

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.DOCUMENT_UNREADABLE.messageKey());
  }

  @Test
  void anUnreachableOcrServiceIsARetryableInternalError() throws IOException {
    when(ocr.recognize(anyString(), any(), anyString())).thenThrow(new IOException("down"));
    passedCapture();

    authenticator.action(context);

    verifyError(ScanovateError.INTERNAL, true);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 3);
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertNull(caseId());
  }

  @Test
  void imagesTheOcrServiceCouldNotProcessAreARetryableInternalError() throws IOException {
    when(ocr.recognize(anyString(), any(), anyString()))
        .thenReturn(json("{\"status\": \"internal error\"}"));
    passedCapture();

    authenticator.action(context);

    verifyError(ScanovateError.INTERNAL, true);
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
  }

  @Test
  void aFailedRuleCountsAsAnAttempt() throws IOException {
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE,
        """
        {"default": [{"type": "equalValue", "equalValue": "ESP", "process": "ocr",
          "attributePath": "/issuing_country_code", "errorMsg": "customError"}]}
        """);
    passedCapture();

    authenticator.action(context);

    verifyFailedAttempt("customError");
  }

  @Test
  void theLastAllowedAttemptRejectsTheVoter() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");
    when(ocr.recognize(anyString(), any(), anyString()))
        .thenReturn(
            json(
                """
                {"status": "completed", "front": {"processing_result":
                 {"status": "fail_to_recognize_mrz"}}}
                """));
    passedCapture();

    authenticator.action(context);

    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 0);
  }

  @Test
  void documentTypesWithoutRulesAreRejected() throws IOException {
    config.put(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE, "{\"passport\": []}");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "seamanBook");
    passedCapture();

    authenticator.action(context);

    verify(context, never()).success();
    assertNull(authNotes.get(STATUS_NOTE));
    verifyError(ScanovateError.INTERNAL, false);
  }

  @Test
  void withoutAttributesToStoreTheVerificationSucceedsDirectly() throws IOException {
    config.remove(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE);
    passedCapture();

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
  void forgedConfirmationStartsACapture() {
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context, never()).success();
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void retryStartsANewCapture() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verifyCapturePage(List.of("FRONT", "BACK"), 2);
  }

  @Test
  void theFaceIsCheckedBeforeTheDocumentIsRead() throws IOException {
    passedCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.3));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.3000");
    verify(ocr, never()).recognize(anyString(), any(), anyString());
  }

  @Test
  void faceMatchUsesTheConfiguredServiceAndMinimum() throws IOException {
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "driversLicense");
    config.put(
        ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY,
        "{\"driversLicense\": 0.9, \"default\": 0.5}");
    passedCapture();
    when(faceMatch.compare(any(), any())).thenReturn(match(0.85));

    authenticator.action(context);

    assertEquals(
        new FaceMatchSettings(URI.create("http://face-match:3000"), 0.9),
        faceMatchSettings.get(faceMatchSettings.size() - 1));
    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
  }

  @Test
  void anotherPersonHoldingTheDocumentCountsAsAnAttempt() throws IOException {
    passedCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG)).thenReturn(match(0.9));
    when(faceMatch.compare(HOLDING_JPEG, JPEG)).thenReturn(match(0.2));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
  }

  @Test
  void faceNotFoundCountsAsAnAttempt() throws IOException {
    passedCapture();
    when(faceMatch.compare(FRONT_JPEG, JPEG))
        .thenReturn(new FaceComparison(false, 0.0, 0.67, 1101, 0));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_NOT_FOUND.messageKey());
    verify(faceMatch, never()).compare(HOLDING_JPEG, JPEG);
  }

  @Test
  void unreachableFaceMatchIsARetryableInternalError() throws IOException {
    passedCapture();
    when(faceMatch.compare(any(), any())).thenThrow(new IOException("down"));

    authenticator.action(context);

    verify(ocr, never()).recognize(anyString(), any(), anyString());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verifyError(ScanovateError.INTERNAL, true);
  }

  @Test
  void failedLivenessCountsAsAnAttempt() throws IOException {
    String token = startCapture();
    livenessResult(token, false);
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.LIVENESS_FAILED.messageKey());
    verify(faceMatch, never()).compare(any(), any());
    verify(ocr, never()).recognize(anyString(), any(), anyString());
  }

  @Test
  void missingLivenessResultIsARetryableInternalError() throws IOException {
    startCapture();
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verify(faceMatch, never()).compare(any(), any());
    verify(ocr, never()).recognize(anyString(), any(), anyString());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verifyError(ScanovateError.INTERNAL, true);
  }

  @Test
  void captureWithoutTheBackIsRejectedWithoutCountingAnAttempt() throws IOException {
    String token = startCapture();
    livenessResult(token, true);
    capture(FRONT_JPEG, null, HOLDING_JPEG);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void captureWithoutThePhotoHoldingTheDocumentIsRejected() throws IOException {
    String token = startCapture();
    livenessResult(token, true);
    capture(FRONT_JPEG, BACK_JPEG, null);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void captureWithWrongMagicBytesIsRejected() throws IOException {
    String token = startCapture();
    livenessResult(token, true);
    capture(PNG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void oversizedCaptureIsRejected() throws IOException {
    config.put(ScanovateAuthenticatorFactory.MAX_IMAGE_BYTES, "3");
    String token = startCapture();
    livenessResult(token, true);
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verifyCaptureRejected();
  }

  @Test
  void multipartCapturePostIsRejectedWithoutReadingTheForm() throws IOException {
    startCapture();
    when(httpHeaders.getMediaType()).thenReturn(MediaType.MULTIPART_FORM_DATA_TYPE);

    authenticator.action(context);

    verify(context.getHttpRequest(), never()).getDecodedFormParameters();
    verifyCaptureRejected();
  }

  @Test
  void multipartPostWithoutPendingCaptureStartsOver() {
    when(httpHeaders.getMediaType())
        .thenReturn(MediaType.valueOf("multipart/form-data; boundary=x"));

    authenticator.action(context);

    verify(context.getHttpRequest(), never()).getDecodedFormParameters();
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  @Test
  void uploadsAreDiscardedOnceTheCaptureIsProcessed() {
    passedCapture();
    String token = captureToken();

    authenticator.action(context);

    assertEquals(Optional.empty(), captureUploads.parts(token));
    assertNull(authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE));
  }

  @Test
  void rejectedCaptureGetsAFreshUploadToken() throws IOException {
    startCapture();
    capture(FRONT_JPEG, null, HOLDING_JPEG);
    String token = captureToken();

    authenticator.action(context);

    assertEquals(Optional.empty(), captureUploads.parts(token));
    String fresh = authNotes.get(ScanovateAuthenticator.CAPTURE_TOKEN_NOTE);
    assertNotNull(fresh);
    assertNotEquals(token, fresh);
    assertEquals(Optional.of(Map.of()), captureUploads.parts(fresh));
  }

  @Test
  void captureWithoutAPendingCaptureStartsOver() throws IOException {
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verify(ocr, never()).recognize(anyString(), any(), anyString());
    verifyCapturePage(List.of("FRONT", "BACK"), 3);
  }

  private void verifyMaxRetriesWithoutNewCapture() {
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 0);
    verify(form, never()).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    assertNull(caseId());
  }

  @Test
  void refreshAfterMaxRetriesDoesNotStartACapture() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");

    authenticator.authenticate(context);

    verifyMaxRetriesWithoutNewCapture();
    verify(context, never()).success();
  }

  @Test
  void retryAfterMaxRetriesIsRejected() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verifyMaxRetriesWithoutNewCapture();
  }

  @Test
  void captureWithoutPendingCaptureAfterMaxRetriesIsRejected() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "4");
    capture(FRONT_JPEG, BACK_JPEG, HOLDING_JPEG);

    authenticator.action(context);

    verifyMaxRetriesWithoutNewCapture();
    verify(ocr, never()).recognize(anyString(), any(), anyString());
  }

  @Test
  void forgedConfirmationAfterMaxRetriesIsRejected() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verifyMaxRetriesWithoutNewCapture();
    verify(context, never()).success();
  }

  @Test
  void aCaptureIsStillStartedBeforeTheLastAttempt() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");

    authenticator.authenticate(context);

    verifyCapturePage(List.of("FRONT", "BACK"), 1);
  }
}
