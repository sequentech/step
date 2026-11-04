// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
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
import static sequent.keycloak.scanovate_authenticator.OcrResultsTest.PASSPORT_RESPONSE;
import static sequent.keycloak.scanovate_authenticator.OnPremSessionsTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.Event;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.headers.SecurityHeadersOptions;
import org.keycloak.headers.SecurityHeadersProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;
import sequent.keycloak.authenticator.StepBack;
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceComparison;
import sequent.keycloak.voter_enrollment.IdentityMethod;
import sequent.keycloak.voter_enrollment.LookupAndUpdateUser;
import sequent.keycloak.voter_enrollment.Utils;

class ScanovateAuthenticatorTest {
  private static final String STATUS_NOTE = "sequent.read-only.id-card-number-validated";
  private static final String IDENTITY_METHOD = LookupAndUpdateUser.IDENTITY_METHOD;

  /** The portrait in {@link OcrResultsTest#PASSPORT_RESPONSE}. */
  private static final byte[] PORTRAIT = "hello".getBytes(StandardCharsets.US_ASCII);

  /** The photo of the voter holding their document that the capture page uploads. */
  private static final byte[] HOLDING = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 'h', 'o', 'l', 'd'};

  private final Map<String, String> config = new HashMap<>();
  private final Map<String, String> authNotes = new HashMap<>();
  private final MultivaluedHashMap<String, String> formParams = new MultivaluedHashMap<>();

  private AuthenticationFlowContext context;
  private LoginFormsProvider form;
  private AuthenticationExecutionModel execution;
  private ScanovateAuthenticator authenticator;
  private MockedStatic<Utils> utils;
  private OnPremSessions livenessSessions;
  private OnPremSessions ocrSessions;
  private HoldingPhotos holdingPhotos;

  /** What OCR reads from the document in its result callback. */
  private JsonNode ocrResponse;

  private HttpHeaders httpHeaders;
  private SecurityHeadersOptions securityHeaders;
  private EventBuilder event;
  private Event eventData;
  private final List<Event> failures = new ArrayList<>();
  private FaceMatchClient faceMatch;
  private final List<FaceMatchSettings> faceMatchSettings = new ArrayList<>();

  @BeforeEach
  void setUp() throws IOException {
    config.putAll(OnPremSettingsTest.livenessConfig());
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
    eventData = new Event();
    eventData.setType(EventType.REGISTER);
    failures.clear();
    eventData.setDetails(new HashMap<>());
    when(event.getEvent()).thenReturn(eventData);
    doAnswer(
            i -> {
              eventData.getDetails().put(i.getArgument(0), i.getArgument(1));
              return event;
            })
        .when(event)
        .detail(anyString(), anyString());
    doAnswer(
            i -> {
              eventData.setUserId(i.getArgument(0, String.class));
              return event;
            })
        .when(event)
        .user((String) any());
    doAnswer(
            i -> {
              failures.add(eventData.clone());
              return null;
            })
        .when(event)
        .error(anyString());
    when(event.clone())
        .thenAnswer(
            i -> {
              Event snapshot = eventData.clone();
              EventBuilder copy = mock(EventBuilder.class, RETURNS_SELF);
              when(copy.getEvent()).thenReturn(snapshot);
              doAnswer(
                      call -> {
                        snapshot.setError(call.getArgument(0));
                        snapshot.setType(EventType.REGISTER_ERROR);
                        failures.add(snapshot.clone());
                        return null;
                      })
                  .when(copy)
                  .error(anyString());
              return copy;
            });
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

    OnPremSessionsTest.MemoryStore store = new OnPremSessionsTest.MemoryStore();
    livenessSessions = new OnPremSessions(OnPremService.LIVENESS, store, millis -> {});
    ocrSessions = new OnPremSessions(OnPremService.OCR, store, millis -> {});
    holdingPhotos = new HoldingPhotos(store);
    ocrResponse = json(PASSPORT_RESPONSE);
    faceMatch = mock(FaceMatchClient.class);
    when(faceMatch.compare(any(), any())).thenReturn(match(0.9));
    faceMatchSettings.clear();
    authenticator =
        new ScanovateAuthenticator(
            (service, ignored) -> service == OnPremService.OCR ? ocrSessions : livenessSessions,
            settings -> {
              faceMatchSettings.add(settings);
              return faceMatch;
            },
            ignored -> holdingPhotos);

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
  void failureHasBoundedEnteredAndExtractedDataWithoutSecretsOrDocumentNumbers()
      throws IOException {
    authNotes.put("email", "person@example.invalid");
    authNotes.put(Utils.PHONE_NUMBER_ATTRIBUTE, "+12025550123");
    authNotes.put("sequent.read-only.id-card-number", "P123456789");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "passport");
    UserProfileProvider profiles = mock(UserProfileProvider.class);
    UPConfig profileConfig = mock(UPConfig.class);
    UPAttribute document = mock(UPAttribute.class);
    when(context.getSession().getProvider(UserProfileProvider.class)).thenReturn(profiles);
    when(profiles.getConfiguration()).thenReturn(profileConfig);
    when(profileConfig.getAttribute(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE))
        .thenReturn(document);
    when(document.getValidations())
        .thenReturn(Map.of("options", Map.of("options", List.of("passport"))));
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE,
        """
        {"default": [{"type": "equalValue", "equalValue": "ESP", "process": "ocr",
          "attributePath": "/issuing_country_code", "errorMsg": "customError"}]}
        """);
    String token = passedCapture();
    String caseId = caseId();
    eventData.getDetails().put("session_id", "synthetic-session");
    eventData.getDetails().put("redirect_uri", "https://example.invalid/?secret=synthetic");

    authenticator.action(context);

    Map<String, String> details = lastFailure();
    assertEquals("validation", details.get("scanovate_stage"));
    assertEquals(caseId, details.get("scanovate_case_id"));
    assertEquals("passport", details.get("scanovate_entered_document_type"));
    assertEquals("present_redacted", details.get("scanovate_entered_email_state"));
    assertEquals("present_redacted", details.get("scanovate_entered_phone_state"));
    assertEquals("P", details.get("scanovate_extracted_document_type"));
    assertEquals("PHL", details.get("scanovate_extracted_issuing_country"));
    assertEquals("true", details.get("scanovate_check_expiry_date_valid"));
    assertEquals("missing", details.get("scanovate_check_barcode_valid"));
    assertEquals("scanovateAttributesError", details.get("scanovate_failure_reason"));
    for (String forbidden :
        List.of(
            token,
            "person@",
            "+12025550123",
            "P123456789",
            "P1234567A",
            "JUAN",
            "synthetic-session",
            "https://",
            "customError")) {
      assertFalse(details.toString().contains(forbidden));
    }
    assertTrue(details.size() <= 40);
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_CODE_ID, details.get("scanovate_attempt_id"));
    verify(context, never()).success();
    Map<String, String> normalized = new HashMap<>(details);
    normalized.put("scanovate_attempt_id", "00000000-0000-4000-8000-000000000004");
    normalized.put("scanovate_case_id", "00000000-0000-4000-8000-000000000005");
    var fixture =
        new ObjectMapper()
            .readTree(
                Files.readString(
                    Path.of(
                        "../custom-event-listener/src/test/resources/scanovate-failure-contract.json")));
    assertEquals(fixture.get("producer_details"), new ObjectMapper().valueToTree(normalized));
  }

  @Test
  void finalAttemptKeepsTheUnderlyingFailure() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");
    ocrResponse =
        json(
            "{\"status\":\"completed\",\"front\":{\"processing_result\":{\"status\":\"card_not_detected\"}}}");
    passedCapture();
    authenticator.action(context);
    assertEquals("scanovateMaxRetriesError", lastFailure().get("scanovate_error"));
    assertEquals("scanovateDocumentUnreadableError", lastFailure().get("scanovate_failure_reason"));
    assertEquals("ocr", lastFailure().get("scanovate_stage"));
    assertEquals("card_not_detected", lastFailure().get("scanovate_ocr_front_result"));
    assertEquals("completed", lastFailure().get("scanovate_ocr_front_status"));
    assertEquals("false", lastFailure().get("scanovate_can_retry"));
    verify(context, never()).success();
  }

  @Test
  void retryDoesNotCarryAReadingOrReasonFromThePreviousAttempt() throws IOException {
    when(faceMatch.compare(any(), any())).thenReturn(match(0.1));
    passedCapture();
    authenticator.action(context);
    Map<String, String> first = lastFailure();
    post(FormAction.RETRY);
    authenticator.action(context);
    // A new HTTP request has its own builder metadata. The second failure is a missing liveness
    // result.
    post(FormAction.CAPTURE);
    authenticator.action(context);
    Map<String, String> second = lastFailure();
    assertEquals(2, failures.size());
    assertNotEquals(first.get("scanovate_attempt_id"), second.get("scanovate_attempt_id"));
    assertEquals("scanovateFaceMismatchError", first.get("scanovate_failure_reason"));
    assertEquals("scanovateInternalError", second.get("scanovate_failure_reason"));
    assertFalse(second.containsKey("scanovate_face_match_document"));
  }

  @Test
  void configurationFailureHasAnAttemptReferenceAndNoInventedReading() {
    config.put(ScanovateAuthenticatorFactory.OCR_TYPES, "{\"iBP\": \"regula\"}");
    authenticator.authenticate(context);
    assertEquals("configuration", lastFailure().get("scanovate_stage"));
    assertEquals("ocr", lastFailure().get("scanovate_config_component"));
    assertNotNull(lastFailure().get("scanovate_attempt_id"));
    assertFalse(lastFailure().containsKey("scanovate_case_id"));
    assertFalse(lastFailure().containsKey("scanovate_extracted_document_type"));
    verify(context, never()).success();
  }

  private Map<String, String> lastFailure() {
    assertFalse(failures.isEmpty(), "a failure must actually be emitted");
    return failures.get(failures.size() - 1).getDetails();
  }

  @Test
  void matchedVoterAttributionAndInheritedMetadataSurviveFailureEmission() throws IOException {
    authNotes.put(Utils.USER_ID, "00000000-0000-4000-8000-000000000003");
    eventData.getDetails().put("auth_method", "openid-connect");
    when(execution.isAlternative()).thenReturn(true);
    when(faceMatch.compare(any(), any()))
        .thenAnswer(
            call -> {
              eventData.getDetails().put("session_id", "synthetic-session");
              return match(0.1);
            });
    passedCapture();
    authenticator.action(context);
    assertEquals("00000000-0000-4000-8000-000000000003", failures.get(0).getUserId());
    assertFalse(lastFailure().containsKey("session_id"));
    assertFalse(lastFailure().containsKey("auth_method"));
    assertEquals("openid-connect", eventData.getDetails().get("auth_method"));
    assertEquals(EventType.REGISTER, eventData.getType());
    assertNull(eventData.getError());
    verify(context).attempted();
  }

  @Test
  void successAndAlreadyVerifiedPassThroughKeepAuthenticationMetadata() {
    eventData.getDetails().put("auth_method", "openid-connect");
    config.put(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE, "{}");
    passedCapture();
    authenticator.action(context);
    assertEquals("openid-connect", eventData.getDetails().get("auth_method"));
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(STATUS_NOTE)).thenReturn("VERIFIED");
    when(context.getUser()).thenReturn(user);
    authenticator.authenticate(context);
    assertEquals("openid-connect", eventData.getDetails().get("auth_method"));
    assertTrue(failures.isEmpty());
  }

  @Test
  void retryAfterConfirmationNeverRelabelsOcrAsEntered() throws IOException {
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE,
        "{\"default\":[{\"UserAttribute\":\"sequent.read-only.id-card-number\",\"process\":\"ocr\",\"attributePath\":\"/document_number\",\"type\":\"text\"}]}");
    passedCapture();
    authenticator.action(context);
    assertEquals("P1234567A", authNotes.get("sequent.read-only.id-card-number"));
    post(FormAction.RETRY);
    authenticator.action(context);
    ocrResult();
    livenessResult(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE), true);
    when(faceMatch.compare(any(), any())).thenReturn(match(0.1));
    post(FormAction.CAPTURE);
    authenticator.action(context);
    assertEquals("missing", lastFailure().get("scanovate_entered_document_number_state"));
  }

  @Test
  void serviceFailuresKeepSafeCategoriesAndHttpStatus() throws IOException {
    when(faceMatch.compare(any(), any())).thenThrow(ServiceFailure.http(401));
    passedCapture();
    authenticator.action(context);
    assertEquals("http_error", lastFailure().get("scanovate_service_error"));
    assertEquals("401", lastFailure().get("scanovate_http_status"));
    assertEquals("face_document", lastFailure().get("scanovate_stage"));
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
        OnPremSessions.RecordOutcome.RECORDED,
        livenessSessions.record(
            token,
            "callback-secret",
            json(
                OnPremSessionsTest.result("completed", passed, JPEG)
                    .replace("\"proc-1\"", "\"" + caseId() + "\""))));
  }

  /** OCR posts what it read from the document, {@link #ocrResponse}, for the pending capture. */
  private void ocrResult() {
    ObjectNode body = ocrResponse.deepCopy();
    body.put("message_type", "result");
    body.putObject("onprem_params").put("case_id", caseId());
    assertEquals(
        OnPremSessions.RecordOutcome.RECORDED,
        ocrSessions.record(
            authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE), "callback-secret", body));
  }

  /** The capture page submits once the voter went through both services' pages. */
  private void submitCapture() {
    if (!formParams.containsKey(ScanovateAuthenticator.FORM_ACTION_PARAM)) {
      formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CAPTURE.value());
    }
  }

  /** The capture page uploads the photo of the voter holding their document. */
  private void holdingPhoto() {
    assertEquals(
        HoldingPhotos.UploadOutcome.STORED,
        holdingPhotos.store(authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE), HOLDING));
  }

  /**
   * A voter whose document OCR read and whose liveness passed uploads the photo holding their
   * document and submits the capture.
   */
  private String passedCapture() {
    String token = startCapture();
    ocrResult();
    livenessResult(token, true);
    holdingPhoto();
    submitCapture();
    return token;
  }

  @SuppressWarnings("unchecked")
  private Map<String, Object> capturePage() {
    ArgumentCaptor<Map<String, Object>> page = ArgumentCaptor.forClass(Map.class);
    verify(form).setAttribute(eq(ScanovateAuthenticator.FTL_SCANOVATE), page.capture());
    verify(form).createForm(ScanovateAuthenticator.CAPTURE_FORM);
    return page.getValue();
  }

  private void verifyCapturePage(int attemptsLeft) {
    verifyCapturePage("passport", attemptsLeft);
  }

  private void verifyCapturePage(String ocrType, int attemptsLeft) {
    Map<String, Object> page = capturePage();
    assertEquals(
        authNotes.getOrDefault(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "default"),
        page.get(ScanovateAuthenticator.FTL_DOCUMENT_TYPE));
    assertEquals(attemptsLeft, page.get(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT));
    assertEquals(3, page.get(ScanovateAuthenticator.FTL_MAX_ATTEMPTS));
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_PAGE_URL,
            "https://liveness.example.com:8443/ocr/?ocr_type="
                + ocrType
                + "&case_id="
                + caseId()
                + "&token="
                + authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE)),
        page.get(ScanovateAuthenticator.FTL_OCR));
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_PAGE_URL,
            "https://liveness.example.com:8443/liveness/?case_id="
                + caseId()
                + "&token="
                + authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE)),
        page.get(ScanovateAuthenticator.FTL_LIVENESS));
    assertEquals(
        Map.of(
            ScanovateAuthenticator.FTL_PAGE_URL,
            "/realms/r/identity-verification/capture/holding",
            ScanovateAuthenticator.FTL_UPLOAD_TOKEN,
            authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE),
            ScanovateAuthenticator.FTL_HOLDING_SECONDS,
            5),
        page.get(ScanovateAuthenticator.FTL_HOLDING));
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
    assertNull(authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE));
    assertNull(authNotes.get(STATUS_NOTE));
    verify(context, never()).success();
  }

  /** The method the enrollment's application would carry, as LookupAndUpdateUser reads it. */
  private Optional<IdentityMethod> forwardedMethod() {
    return LookupAndUpdateUser.identityMethod(context.getAuthenticationSession());
  }

  private void post(FormAction action) {
    formParams.clear();
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, action.value());
  }

  /**
   * The user attribute says that an earlier enrollment verified the voter, but it is not proof of
   * this session's verification: the shortcut keeps passing the step and establishes no method.
   */
  @Test
  void alreadyVerifiedUserSkipsVerification() {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(STATUS_NOTE)).thenReturn("VERIFIED");
    when(context.getUser()).thenReturn(user);
    authNotes.put(IDENTITY_METHOD, "VERIFIED");

    authenticator.authenticate(context);

    verify(context).success();
    verify(form, never()).createForm(anyString());
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void aConfirmedVerificationEstablishesAVerifiedIdentity() {
    passedCapture();

    authenticator.action(context);

    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    assertNull(authNotes.get(IDENTITY_METHOD));

    post(FormAction.CONFIRM);
    authenticator.action(context);

    verify(context).success();
    assertEquals(Optional.of(IdentityMethod.VERIFIED), forwardedMethod());
  }

  @Test
  void aVerificationWithoutAttributesToConfirmEstablishesAVerifiedIdentity() {
    config.remove(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE);
    passedCapture();

    authenticator.action(context);

    verify(context).success();
    assertEquals(Optional.of(IdentityMethod.VERIFIED), forwardedMethod());
  }

  /**
   * The status note is also a profile attribute, which a form could submit: a confirmation needs
   * the verification this step completed, not that note.
   */
  @Test
  void aConfirmationWithoutACompletedVerificationEstablishesNothing() {
    authNotes.put(STATUS_NOTE, "VERIFIED");
    post(FormAction.CONFIRM);

    authenticator.action(context);

    verify(context, never()).success();
    assertNull(authNotes.get(STATUS_NOTE));
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void aConfirmationAbandonedForANewCaptureEstablishesNothing() {
    passedCapture();
    authenticator.action(context);

    // The voter reloads the confirmation page, which starts over, and confirms the old one.
    authenticator.authenticate(context);
    post(FormAction.CONFIRM);
    authenticator.action(context);

    verify(context, never()).success();
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void aFailedVerificationDropsAnEarlierVerifiedIdentity() throws IOException {
    authNotes.put(IDENTITY_METHOD, "VERIFIED");
    passedCapture();
    when(faceMatch.compare(PORTRAIT, JPEG)).thenReturn(match(0.3));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void aProviderErrorDropsAnEarlierVerifiedIdentity() throws IOException {
    authNotes.put(IDENTITY_METHOD, "VERIFIED");
    passedCapture();
    when(faceMatch.compare(any(), any())).thenThrow(new IOException("down"));

    authenticator.action(context);

    verifyError(ScanovateError.INTERNAL, true);
    assertEquals(Optional.empty(), forwardedMethod());
  }

  /** The enrollment's code step, done, then this step. */
  private void afterTheCodeStep() {
    Map<String, ExecutionStatus> statuses = new HashMap<>();
    AuthenticationFlowModel top = new AuthenticationFlowModel();
    top.setId("registration");
    AuthenticationExecutionModel code = new AuthenticationExecutionModel();
    code.setId("code");
    when(execution.getId()).thenReturn("identity");
    when(context.getTopLevelFlow()).thenReturn(top);
    when(context.getRealm().getAuthenticationExecutionsStream(top.getId()))
        .thenAnswer(i -> Stream.of(code, execution));
    when(context.getRefreshUrl(false)).thenReturn(URI.create("https://kc/registration"));
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    when(authSession.getExecutionStatus()).thenReturn(statuses);
    doAnswer(i -> statuses.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setExecutionStatus(anyString(), any());
    statuses.put(code.getId(), ExecutionStatus.SUCCESS);
  }

  @Test
  void backReturnsToTheCodeStep() {
    afterTheCodeStep();
    formParams.putSingle(StepBack.PARAM, "true");

    authenticator.action(context);

    verify(context).forceChallenge(any());
    verify(form, never()).createForm(anyString());
  }

  @Test
  void theCapturePageOffersBackAfterTheCodeStep() {
    afterTheCodeStep();

    authenticator.action(context);

    verify(form).setAttribute(StepBack.ATTRIBUTE, true);
  }

  @Test
  void withoutAPreviousStepBackShowsTheCapturePage() {
    formParams.putSingle(StepBack.PARAM, "true");

    authenticator.action(context);

    verify(context, never()).forceChallenge(any());
    verify(form).setAttribute(StepBack.ATTRIBUTE, false);
    verifyCapturePage(3);
  }

  @Test
  void aRetryDropsAnEarlierVerifiedIdentity() {
    authNotes.put(IDENTITY_METHOD, "VERIFIED");
    post(FormAction.RETRY);

    authenticator.action(context);

    verifyCapturePage(3);
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void aRetryAfterAFailureCanStillEstablishAVerifiedIdentity() throws IOException {
    passedCapture();
    when(faceMatch.compare(PORTRAIT, JPEG)).thenReturn(match(0.3));
    authenticator.action(context);
    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));

    when(faceMatch.compare(PORTRAIT, JPEG)).thenReturn(match(0.9));
    post(FormAction.RETRY);
    authenticator.action(context);
    formParams.clear();
    ocrResult();
    livenessResult(authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE), true);
    holdingPhoto();
    submitCapture();
    authenticator.action(context);
    post(FormAction.CONFIRM);
    authenticator.action(context);

    verify(context).success();
    assertEquals(Optional.of(IdentityMethod.VERIFIED), forwardedMethod());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
  }

  @Test
  void restartingTheStepDropsAnEarlierVerifiedIdentity() {
    authNotes.put(IDENTITY_METHOD, "VERIFIED");

    authenticator.authenticate(context);

    verifyCapturePage(3);
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void reachingTheAttemptLimitDropsAnEarlierVerifiedIdentity() {
    authNotes.put(IDENTITY_METHOD, "VERIFIED");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");

    authenticator.authenticate(context);

    verifyMaxRetriesWithoutNewCapture();
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void anAlternativeStepThatFailsLeavesNoVerifiedIdentity() {
    when(execution.isAlternative()).thenReturn(true);
    authNotes.put(IDENTITY_METHOD, "VERIFIED");
    config.put(ScanovateAuthenticatorFactory.OCR_TYPES, "{\"iBP\": \"regula\"}");

    authenticator.authenticate(context);

    verify(context).attempted();
    assertEquals(Optional.empty(), forwardedMethod());
  }

  @Test
  void authenticateRendersTheCapturePage() throws IOException {
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "philSysID");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");

    String token = startCapture();

    assertNotNull(caseId());
    verifyCapturePage(2);
    // The capture page embeds the services' pages, served from the configured origin
    verify(securityHeaders).allowFrameSrc("https://liveness.example.com:8443");
    assertTrue(livenessSessions.verify(token, caseId(), "callback-secret"));
    String ocrToken = authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE);
    assertTrue(ocrSessions.verify(ocrToken, caseId(), "callback-secret"));
    // Each service only accepts its own token
    assertFalse(livenessSessions.verify(ocrToken, caseId(), "callback-secret"));
    verify(faceMatch, never()).compare(any(), any());
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
  void refreshingThePendingCaptureRendersItAgainWithNewTokens() {
    String first = startCapture();
    String firstOcr = authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE);
    String caseId = caseId();

    authenticator.authenticate(context);

    assertEquals(caseId, caseId());
    String second = authNotes.get(ScanovateAuthenticator.LIVENESS_TOKEN_NOTE);
    String secondOcr = authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE);
    assertNotEquals(first, second);
    assertNotEquals(firstOcr, secondOcr);
    assertFalse(livenessSessions.verify(first, caseId, "callback-secret"));
    assertTrue(livenessSessions.verify(second, caseId, "callback-secret"));
    assertFalse(ocrSessions.verify(firstOcr, caseId, "callback-secret"));
    assertTrue(ocrSessions.verify(secondOcr, caseId, "callback-secret"));
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
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_SECRET, ""),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "high"),
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
    config.put(ScanovateAuthenticatorFactory.OCR_TYPES, "{\"iBP\": \"regula\"}");

    authenticator.authenticate(context);

    verify(context).attempted();
    verify(form, never()).createForm(anyString());
  }

  @Test
  void matchingFacesShowWhatOcrReadForConfirmation() throws IOException {
    String token = passedCapture();
    String ocrToken = authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE);
    String caseId = caseId();

    authenticator.action(context);

    verify(faceMatch).compare(PORTRAIT, JPEG);
    verify(faceMatch).compare(HOLDING, JPEG);
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_CASE_ID, caseId);
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.9000");
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_HOLDING, "0.9000");
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
    assertNull(authNotes.get(ScanovateAuthenticator.OCR_TOKEN_NOTE));
    assertFalse(livenessSessions.verify(token, caseId, "callback-secret"));
    assertFalse(ocrSessions.verify(ocrToken, caseId, "callback-secret"));
    assertNull(authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE));
  }

  @Test
  void theCapturePageGetsATokenForTheHoldingPhoto() {
    startCapture();
    String first = authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE);
    holdingPhoto();

    // A refresh issues a new token and forgets the photo of the earlier one
    authenticator.authenticate(context);

    String second = authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE);
    assertNotNull(second);
    assertNotEquals(first, second);
    assertTrue(holdingPhotos.photo(first).isEmpty());
    assertEquals(HoldingPhotos.UploadOutcome.UNAUTHORIZED, holdingPhotos.store(first, HOLDING));
  }

  @Test
  void aHoldingPhotoOfSomeoneElseCountsAsAnAttempt() throws IOException {
    passedCapture();
    when(faceMatch.compare(HOLDING, JPEG)).thenReturn(match(0.3));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
    assertEquals("face_holding", lastFailure().get("scanovate_stage"));
    assertEquals("0.3000", lastFailure().get("scanovate_face_match_holding"));
    assertNull(authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE));
  }

  @Test
  void aHoldingPhotoWithoutAFaceCountsAsAnAttempt() throws IOException {
    passedCapture();
    when(faceMatch.compare(HOLDING, JPEG)).thenReturn(new FaceComparison(false, 0.0, 0.67, 0, 1));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_NOT_FOUND.messageKey());
    assertEquals("face_holding", lastFailure().get("scanovate_stage"));
  }

  @Test
  void aCaptureWithoutTheHoldingPhotoIsRejectedWithoutComparingIt() throws IOException {
    String token = startCapture();
    ocrResult();
    livenessResult(token, true);
    submitCapture();

    authenticator.action(context);

    verify(faceMatch).compare(PORTRAIT, JPEG);
    verify(faceMatch, never()).compare(HOLDING, JPEG);
    verifyError(ScanovateError.CAPTURE_INVALID, true);
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertEquals("face_holding", lastFailure().get("scanovate_stage"));
    assertNull(authNotes.get(STATUS_NOTE));
    assertNull(caseId());
  }

  @Test
  void theDocumentFaceIsComparedBeforeTheHoldingPhoto() throws IOException {
    passedCapture();
    when(faceMatch.compare(PORTRAIT, JPEG)).thenReturn(match(0.3));

    authenticator.action(context);

    verify(faceMatch, never()).compare(HOLDING, JPEG);
    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
  }

  @Test
  void withoutTheHoldingPhotoPolicyThereIsNoThirdStep() throws IOException {
    config.put(ScanovateAuthenticatorFactory.HOLDING_PHOTO, HoldingPhotoPolicy.NONE.name());
    String token = startCapture();

    assertFalse(capturePage().containsKey(ScanovateAuthenticator.FTL_HOLDING));
    assertNull(authNotes.get(ScanovateAuthenticator.HOLDING_TOKEN_NOTE));

    ocrResult();
    livenessResult(token, true);
    submitCapture();
    authenticator.action(context);

    verify(faceMatch).compare(PORTRAIT, JPEG);
    verify(faceMatch, never()).compare(HOLDING, JPEG);
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
  }

  @Test
  void theHoldingTimeIsConfigurable() {
    config.put(ScanovateAuthenticatorFactory.HOLDING_SECONDS, "3");

    startCapture();

    @SuppressWarnings("unchecked")
    Map<String, Object> holding =
        (Map<String, Object>) capturePage().get(ScanovateAuthenticator.FTL_HOLDING);
    assertEquals(3, holding.get(ScanovateAuthenticator.FTL_HOLDING_SECONDS));
  }

  @Test
  void anInvalidHoldingTimeIsAConfigurationError() {
    config.put(ScanovateAuthenticatorFactory.HOLDING_SECONDS, "0");

    authenticator.authenticate(context);

    verifyError(ScanovateError.INTERNAL, false);
    assertEquals("capture", lastFailure().get("scanovate_config_component"));
  }

  @Test
  void anUnknownHoldingPhotoPolicyIsAConfigurationError() {
    config.put(ScanovateAuthenticatorFactory.HOLDING_PHOTO, "SOMETIMES");

    authenticator.authenticate(context);

    verifyError(ScanovateError.INTERNAL, false);
    assertEquals("capture", lastFailure().get("scanovate_config_component"));
  }

  @Test
  void theDocumentTypeSelectsItsOcrType() {
    config.put(
        ScanovateAuthenticatorFactory.OCR_TYPES,
        "{\"philippinePassport\": \"passport\", \"default\": \"regula\"}");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "driversLicense");

    authenticator.authenticate(context);

    verifyCapturePage("regula", 3);
  }

  /** Only the services' callbacks carry a verdict: what the browser posts is never read. */
  @Test
  void verdictsPostedByTheBrowserAreIgnored() throws IOException {
    startCapture();
    formParams.add("passed", "true");
    formParams.add("face_image", "aGVsbG8=");
    submitCapture();

    authenticator.action(context);

    verify(faceMatch, never()).compare(any(), any());
    assertNull(authNotes.get(STATUS_NOTE));
    verifyError(ScanovateError.INTERNAL, true);
  }

  @Test
  void anUnreadableDocumentCountsAsAnAttempt() throws IOException {
    ocrResponse =
        json(
            """
            {"status": "completed", "front": {"processing_result":
             {"status": "card_not_detected", "images": {"face_image": "aGVsbG8="}}}}
            """);
    passedCapture();

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.DOCUMENT_UNREADABLE.messageKey());
    verify(faceMatch, never()).compare(any(), any());
  }

  @Test
  void missingOcrResultIsARetryableInternalError() throws IOException {
    String token = startCapture();
    livenessResult(token, true);
    submitCapture();

    authenticator.action(context);

    verifyError(ScanovateError.INTERNAL, true);
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ATTEMPTS_LEFT, 3);
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertNull(caseId());
    verify(faceMatch, never()).compare(any(), any());
  }

  @Test
  void anOcrSessionThatDidNotCompleteCountsAsAnAttempt() throws IOException {
    ((ObjectNode) ocrResponse).put("status", "expired");
    passedCapture();

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.DOCUMENT_UNREADABLE.messageKey());
    assertEquals("expired", lastFailure().get("scanovate_ocr_front_status"));
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
    ocrResponse =
        json(
            """
            {"status": "completed", "front": {"processing_result":
             {"status": "fail_to_recognize_mrz"}}}
            """);
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
    passedCapture();
    authenticator.action(context);
    post(FormAction.CONFIRM);

    authenticator.action(context);

    verify(context).success();
  }

  @Test
  void forgedConfirmationStartsACapture() {
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context, never()).success();
    verifyCapturePage(3);
  }

  @Test
  void retryStartsANewCapture() {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verifyCapturePage(2);
  }

  @Test
  void theFacesAreComparedBeforeTheRulesApply() throws IOException {
    passedCapture();
    when(faceMatch.compare(PORTRAIT, JPEG)).thenReturn(match(0.3));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_MISMATCH.messageKey());
    verify(event).detail(ScanovateAuthenticator.EVENT_DETAIL_FACE_MATCH_DOCUMENT, "0.3000");
    assertEquals("face_document", lastFailure().get("scanovate_stage"));
    assertNull(authNotes.get("firstName"));
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
  void faceNotFoundCountsAsAnAttempt() throws IOException {
    passedCapture();
    when(faceMatch.compare(PORTRAIT, JPEG))
        .thenReturn(new FaceComparison(false, 0.0, 0.67, 1101, 0));

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.FACE_NOT_FOUND.messageKey());
  }

  @Test
  void unreachableFaceMatchIsARetryableInternalError() throws IOException {
    passedCapture();
    when(faceMatch.compare(any(), any())).thenThrow(new IOException("down"));

    authenticator.action(context);

    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    verifyError(ScanovateError.INTERNAL, true);
  }

  @Test
  void failedLivenessCountsAsAnAttempt() throws IOException {
    String token = startCapture();
    ocrResult();
    livenessResult(token, false);
    submitCapture();

    authenticator.action(context);

    verifyFailedAttempt(ScanovateError.LIVENESS_FAILED.messageKey());
    assertEquals("false", lastFailure().get("scanovate_liveness_passed"));
    assertEquals("liveness", lastFailure().get("scanovate_stage"));
    verify(faceMatch, never()).compare(any(), any());
  }

  @Test
  void missingLivenessResultIsARetryableInternalError() throws IOException {
    startCapture();
    ocrResult();
    submitCapture();

    authenticator.action(context);

    verify(faceMatch, never()).compare(any(), any());
    assertNull(authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertEquals("unknown", lastFailure().get("scanovate_liveness_status"));
    verifyError(ScanovateError.INTERNAL, true);
  }

  @Test
  void captureWithoutAPendingCaptureStartsOver() throws IOException {
    submitCapture();

    authenticator.action(context);

    verify(faceMatch, never()).compare(any(), any());
    verifyCapturePage(3);
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
    submitCapture();

    authenticator.action(context);

    verifyMaxRetriesWithoutNewCapture();
    verify(faceMatch, never()).compare(any(), any());
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

    verifyCapturePage(1);
  }
}
