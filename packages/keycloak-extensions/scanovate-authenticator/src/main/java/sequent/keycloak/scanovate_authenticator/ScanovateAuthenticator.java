// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.time.LocalDate;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceComparison;
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceMatchOutcome;
import sequent.keycloak.scanovate_authenticator.LivenessSessions.LivenessResult;
import sequent.keycloak.voter_enrollment.Utils;

/**
 * Verifies the voter's identity with B-Trust (Scanovate).
 *
 * <p>The voter is redirected to a B-Trust flow and, when they come back, the results are fetched
 * server to server, validated against the configured rules, and the extracted attributes are stored
 * as auth notes for the voter to confirm. In the embedded mode the voter captures the media in
 * Keycloak's own page instead, and Keycloak uploads it to the B-Trust session before fetching the
 * results. The page uploads each file to Keycloak on its own before submitting the capture, see
 * {@link CaptureUploads}.
 *
 * <p>With the liveness face capture, the voter's face never leaves our premises. The capture page
 * checks the voter's liveness with the on-premise Liveness Plus API, whose verdict and picture of
 * the voter reach Keycloak server to server (see {@link LivenessSessions}). Keycloak then compares
 * that picture with the photo of the ID and with a photo of the voter holding it, using the
 * on-premise Face Match service, and only uploads the photos of the ID to B-Trust, for OCR and
 * document authenticity.
 */
@JBossLog
public class ScanovateAuthenticator implements Authenticator {
  static final String PROCESS_ID_NOTE = "scanovate-process-id";
  static final String LIVENESS_TOKEN_NOTE = "scanovate-liveness-token";
  static final String CAPTURE_TOKEN_NOTE = "scanovate-capture-token";
  static final String ATTEMPTS_NOTE = "scanovate-attempts";
  static final String PROCESS_ID_QUERY_PARAM = "processId";
  static final String FORM_ACTION_PARAM = "action";
  static final String USER_STATUS_VERIFIED = "VERIFIED";
  static final String CONFIRMATION_FORM = "scanovate-confirmation.ftl";
  static final String ERROR_FORM = "scanovate-error.ftl";
  static final String CAPTURE_FORM = "scanovate-capture.ftl";
  static final String FTL_ERROR = "error";
  static final String FTL_CODE_ID = "code_id";
  static final String FTL_CAN_RETRY = "canRetry";
  static final String FTL_STORED_ATTRIBUTES = "storedAttributes";
  static final String FTL_DOCUMENT_TYPE = "documentType";
  static final String FTL_ATTEMPTS_LEFT = "attemptsLeft";
  static final String FTL_MAX_ATTEMPTS = "maxAttempts";
  static final String FTL_SCANOVATE = "scanovate";
  static final String FTL_SIDES = "sides";
  static final String FTL_VIDEO_SECONDS = "videoSeconds";
  static final String FTL_LIVENESS = "liveness";
  static final String FTL_LIVENESS_URL = "url";
  static final String FTL_LIVENESS_TOKEN = "token";
  static final String FTL_LIVENESS_CASE_ID = "caseId";
  static final String FTL_UPLOAD = "upload";
  static final String FTL_UPLOAD_URL = "url";
  static final String FTL_UPLOAD_TOKEN = "token";
  static final String EVENT_ERROR = "scanovate_verification_failed";
  static final String EVENT_DETAIL_ERROR = "scanovate_error";
  static final String EVENT_DETAIL_PROCESS_ID = "scanovate_process_id";
  static final String EVENT_DETAIL_FACE_MATCH_DOCUMENT = "scanovate_face_match_document";
  static final String EVENT_DETAIL_FACE_MATCH_HOLDING = "scanovate_face_match_holding";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final Function<Map<String, String>, ScanovateClient> clientFactory;
  private final Function<KeycloakSession, LivenessSessions> livenessFactory;
  private final Function<FaceMatchSettings, FaceMatchClient> faceMatchFactory;
  private final Function<KeycloakSession, CaptureUploads> uploadsFactory;

  public ScanovateAuthenticator() {
    this(ScanovateAuthenticator::defaultClient);
  }

  ScanovateAuthenticator(Function<Map<String, String>, ScanovateClient> clientFactory) {
    this(
        clientFactory,
        ScanovateAuthenticator::defaultLiveness,
        ScanovateAuthenticator::defaultFaceMatch,
        ScanovateAuthenticator::defaultUploads);
  }

  ScanovateAuthenticator(
      Function<Map<String, String>, ScanovateClient> clientFactory,
      Function<KeycloakSession, LivenessSessions> livenessFactory,
      Function<FaceMatchSettings, FaceMatchClient> faceMatchFactory,
      Function<KeycloakSession, CaptureUploads> uploadsFactory) {
    this.clientFactory = clientFactory;
    this.livenessFactory = livenessFactory;
    this.faceMatchFactory = faceMatchFactory;
    this.uploadsFactory = uploadsFactory;
  }

  static CaptureUploads defaultUploads(KeycloakSession session) {
    return new CaptureUploads(new SingleUseLivenessStore(session));
  }

  static FaceMatchClient defaultFaceMatch(FaceMatchSettings settings) {
    return new FaceMatchClient(
        new JdkHttpTransport(),
        settings.url(),
        ScanovateAuthenticatorFactory.DEFAULT_MAX_RETRIES,
        Thread::sleep);
  }

  static LivenessSessions defaultLiveness(KeycloakSession session) {
    return new LivenessSessions(new SingleUseLivenessStore(session), Thread::sleep);
  }

  static ScanovateClient defaultClient(Map<String, String> config) {
    return new ScanovateClient(
        new JdkHttpTransport(),
        config.getOrDefault(ScanovateAuthenticatorFactory.BASE_URL, ""),
        config.getOrDefault(ScanovateAuthenticatorFactory.CLIENT_ID, ""),
        config.getOrDefault(ScanovateAuthenticatorFactory.CLIENT_SECRET, ""),
        parseInt(
            config.get(ScanovateAuthenticatorFactory.MAX_RETRIES),
            ScanovateAuthenticatorFactory.DEFAULT_MAX_RETRIES),
        Thread::sleep);
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    buildEventDetails(context);

    UserModel user = context.getUser();
    String statusAttribute = userStatusNote(config);
    if (user != null
        && statusAttribute != null
        && USER_STATUS_VERIFIED.equals(user.getFirstAttribute(statusAttribute))) {
      log.info("authenticate: user already verified");
      context.success();
      return;
    }

    // Keycloak treats a return with an already consumed session code as a page refresh and calls
    // authenticate() instead of action(), so the B-Trust return is also handled here.
    String processId = authSession.getAuthNote(PROCESS_ID_NOTE);
    String returnedProcessId =
        context.getUriInfo().getQueryParameters().getFirst(PROCESS_ID_QUERY_PARAM);
    if (processId != null && processId.equals(returnedProcessId)) {
      processReturn(context);
      return;
    }
    if (processId != null && isEmbedded(config)) {
      showPendingCapture(context);
      return;
    }

    startVerification(context);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    buildEventDetails(context);
    if (isMultipart(context)) {
      rejectMultipart(context);
      return;
    }
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    Optional<FormAction> formAction = FormAction.fromValue(formData.getFirst(FORM_ACTION_PARAM));

    if (formAction.isEmpty()) {
      processReturn(context);
      return;
    }
    switch (formAction.get()) {
      case CONFIRM -> confirm(context);
      case RETRY -> startVerification(context);
      case CAPTURE -> capture(context);
    }
  }

  /**
   * Whether the request is a multipart form. Keycloak can't read its fields when it has files, so
   * the capture page uploads its files beforehand and submits the capture as a plain form.
   */
  private static boolean isMultipart(AuthenticationFlowContext context) {
    MediaType mediaType =
        context.getHttpRequest().getHttpHeaders() == null
            ? null
            : context.getHttpRequest().getHttpHeaders().getMediaType();
    return mediaType != null && MediaType.MULTIPART_FORM_DATA_TYPE.isCompatible(mediaType);
  }

  /** Shows the pending capture again as invalid, or starts a new verification without one. */
  private void rejectMultipart(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (authSession.getAuthNote(PROCESS_ID_NOTE) == null || !isEmbedded(config)) {
      log.warn("action: rejected a multipart form, starting a new session");
      startVerification(context);
      return;
    }
    log.warn("action: rejected a multipart capture");
    CaptureSettings settings;
    try {
      settings = CaptureSettings.fromConfig(config, documentType(config, authSession));
    } catch (ScanovateException e) {
      log.error("action: invalid capture configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }
    showCapture(context, settings, Optional.of(ScanovateError.CAPTURE_INVALID.messageKey()));
  }

  private void startVerification(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (attempts(context) >= maxAttempts(config)) {
      log.warn("startVerification: maximum attempts reached, not starting a new session");
      showError(context, ScanovateError.MAX_RETRIES, false);
      return;
    }
    clearVerification(context);

    int flowId;
    ExecutionMode mode;
    SaveOption saveOption;
    Map<String, String> params;
    URI redirectUrl;
    Optional<CaptureSettings> captureSettings;
    try {
      flowId = Integer.parseInt(config.getOrDefault(ScanovateAuthenticatorFactory.FLOW_ID, ""));
      mode =
          executionMode(config).orElseThrow(() -> new ScanovateException("Invalid execution mode"));
      captureSettings =
          mode == ExecutionMode.EMBEDDED
              ? Optional.of(CaptureSettings.fromConfig(config, documentType(config, authSession)))
              : Optional.empty();
      if (captureSettings.filter(s -> s.faceCapture() == FaceCapture.LIVENESS).isPresent()) {
        LivenessSettings.fromConfig(config);
        FaceMatchSettings.fromConfig(config, documentType(config, authSession));
      }
      saveOption =
          SaveOption.fromValue(config.get(ScanovateAuthenticatorFactory.SAVE_OPTION))
              .orElseThrow(() -> new ScanovateException("Invalid save option"));
      params = linkParams(config, authSession);
      redirectUrl = ReturnUrl.fromActionUrl(context.getActionUrl(context.generateAccessCode()));
    } catch (NumberFormatException | ScanovateException e) {
      log.error("startVerification: invalid authenticator configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }

    String docIdNote =
        config.getOrDefault(
            ScanovateAuthenticatorFactory.DOC_ID, ScanovateAuthenticatorFactory.DEFAULT_DOC_ID);
    LinkRequest request =
        new LinkRequest(
            flowId,
            UUID.randomUUID().toString(),
            authSession.getAuthNote(docIdNote),
            redirectUrl.toString(),
            params,
            saveOption);

    SessionLink link;
    try {
      ScanovateClient client = clientFactory.apply(config);
      link = client.createSessionLink(client.fetchAccessToken(), request);
    } catch (IOException e) {
      log.error("startVerification: could not create the B-Trust session", e);
      showError(context, ScanovateError.INTERNAL, true);
      return;
    }
    log.infov("startVerification: created B-Trust session {0}", link.processId());
    authSession.setAuthNote(PROCESS_ID_NOTE, link.processId());

    switch (mode) {
      case INTERACTIVE -> context.challenge(Response.seeOther(URI.create(link.url())).build());
      case AUTO_COMPLETE -> processReturn(context);
      case EMBEDDED ->
          captureSettings.ifPresent(settings -> showCapture(context, settings, Optional.empty()));
    }
  }

  private void showPendingCapture(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    CaptureSettings settings;
    try {
      settings =
          CaptureSettings.fromConfig(
              config, documentType(config, context.getAuthenticationSession()));
    } catch (ScanovateException e) {
      log.error("showPendingCapture: invalid capture configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }
    showCapture(context, settings, Optional.empty());
  }

  private void showCapture(
      AuthenticationFlowContext context, CaptureSettings settings, Optional<String> errorKey) {
    Map<String, String> config = config(context);
    int maxAttempts = maxAttempts(config);
    Map<String, Object> capture = new LinkedHashMap<>();
    capture.put(
        FTL_DOCUMENT_TYPE,
        documentTypeName(documentType(config, context.getAuthenticationSession())));
    capture.put(FTL_SIDES, settings.sides().stream().map(DocumentSide::name).toList());
    capture.put(FTL_VIDEO_SECONDS, settings.videoSeconds());
    capture.put(FTL_ATTEMPTS_LEFT, attemptsLeft(context, maxAttempts));
    capture.put(FTL_MAX_ATTEMPTS, maxAttempts);
    capture.put(FTL_UPLOAD, uploadPage(context));
    if (settings.faceCapture() == FaceCapture.LIVENESS) {
      LivenessSettings liveness;
      try {
        liveness = LivenessSettings.fromConfig(config);
      } catch (ScanovateException e) {
        log.error("showCapture: invalid liveness configuration", e);
        showError(context, ScanovateError.INTERNAL, false);
        return;
      }
      capture.put(FTL_LIVENESS, livenessPage(context, liveness));
    }

    LoginFormsProvider form = context.form().setAttribute(FTL_SCANOVATE, capture);
    errorKey.ifPresent(form::setError);
    context.challenge(form.createForm(CAPTURE_FORM));
  }

  /**
   * Issues a new token for the capture page to upload its files with, replacing any earlier one and
   * its files, see {@link CaptureUploads}.
   */
  private Map<String, Object> uploadPage(AuthenticationFlowContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    CaptureUploads uploads = uploadsFactory.apply(context.getSession());
    uploads.discard(authSession.getAuthNote(CAPTURE_TOKEN_NOTE));
    String token = uploads.create();
    authSession.setAuthNote(CAPTURE_TOKEN_NOTE, token);

    Map<String, Object> page = new LinkedHashMap<>();
    page.put(FTL_UPLOAD_URL, uploadUrl(context));
    page.put(FTL_UPLOAD_TOKEN, token);
    return page;
  }

  /**
   * Root-relative URL of the upload endpoint, so that the page uploads to its own origin, whatever
   * reverse proxy serves it.
   */
  private static String uploadUrl(AuthenticationFlowContext context) {
    String base = context.getUriInfo().getBaseUri().getRawPath();
    return (base.endsWith("/") ? base : base + "/")
        + "realms/"
        + URLEncoder.encode(context.getRealm().getName(), StandardCharsets.UTF_8)
            .replace("+", "%20")
        + "/"
        + ScanovateReturnResourceFactory.PROVIDER_ID
        + "/"
        + ScanovateReturnResource.CAPTURE_PATH;
  }

  /**
   * Issues a new token for the capture page to start its Liveness Plus sessions with, replacing any
   * earlier one.
   */
  private Map<String, Object> livenessPage(
      AuthenticationFlowContext context, LivenessSettings liveness) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String caseId = authSession.getAuthNote(PROCESS_ID_NOTE);
    LivenessSessions sessions = livenessFactory.apply(context.getSession());
    sessions.discard(authSession.getAuthNote(LIVENESS_TOKEN_NOTE));
    String token = sessions.create(caseId, liveness.secret());
    authSession.setAuthNote(LIVENESS_TOKEN_NOTE, token);

    Map<String, Object> page = new LinkedHashMap<>();
    page.put(FTL_LIVENESS_URL, liveness.apiUrl());
    page.put(FTL_LIVENESS_TOKEN, token);
    page.put(FTL_LIVENESS_CASE_ID, caseId);
    return page;
  }

  private void capture(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String processId = authSession.getAuthNote(PROCESS_ID_NOTE);
    if (processId == null || !isEmbedded(config)) {
      log.warn("capture: no embedded capture in progress, starting a new session");
      startVerification(context);
      return;
    }

    CaptureSettings settings;
    try {
      settings = CaptureSettings.fromConfig(config, documentType(config, authSession));
    } catch (ScanovateException e) {
      log.error("capture: invalid capture configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }

    CaptureUploads uploads = uploadsFactory.apply(context.getSession());
    String uploadToken = authSession.getAuthNote(CAPTURE_TOKEN_NOTE);
    Map<String, byte[]> parts = uploads.parts(uploadToken).orElse(Map.of());
    uploads.discard(uploadToken);
    authSession.removeAuthNote(CAPTURE_TOKEN_NOTE);
    CaptureMedia media;
    try {
      media = CaptureMedia.fromUploads(parts, settings);
    } catch (InvalidCaptureException e) {
      log.warnv("capture: rejected the capture for {0}: {1}", processId, e.getMessage());
      showCapture(context, settings, Optional.of(ScanovateError.CAPTURE_INVALID.messageKey()));
      return;
    }

    if (settings.faceCapture() == FaceCapture.LIVENESS && !checkFaceOnPremise(context, media)) {
      return;
    }

    try {
      ScanovateClient client = clientFactory.apply(config);
      client.uploadMedia(
          client.fetchAccessToken(), processId, media.only(settings.uploadedMedia()));
    } catch (IOException e) {
      log.error("capture: could not upload the capture to B-Trust", e);
      showError(context, ScanovateError.INTERNAL, true);
      return;
    }
    log.infov("capture: uploaded the capture for {0}", processId);
    processReturn(context);
  }

  /**
   * Checks the voter's face on premise: the result callback of the voter's Liveness Plus session
   * must confirm that it passed, and its picture of the voter must match the photo of the ID and
   * the photo of the voter holding it. Otherwise, shows the error and returns false.
   */
  private boolean checkFaceOnPremise(AuthenticationFlowContext context, CaptureMedia media) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String processId = authSession.getAuthNote(PROCESS_ID_NOTE);
    LivenessSettings liveness;
    FaceMatchSettings faceMatch;
    try {
      liveness = LivenessSettings.fromConfig(config);
      faceMatch = FaceMatchSettings.fromConfig(config, documentType(config, authSession));
    } catch (ScanovateException e) {
      log.error("capture: invalid liveness or face match configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return false;
    }

    String token = authSession.getAuthNote(LIVENESS_TOKEN_NOTE);
    LivenessSessions sessions = livenessFactory.apply(context.getSession());
    Optional<LivenessResult> result =
        token == null
            ? Optional.empty()
            : sessions.awaitResult(token, liveness.resultWaitSeconds());
    if (result.isEmpty()) {
      log.errorv("capture: no liveness result for {0}", processId);
      showError(context, ScanovateError.INTERNAL, true);
      return false;
    }
    sessions.discard(token);
    authSession.removeAuthNote(LIVENESS_TOKEN_NOTE);
    if (!result.get().passed() || result.get().image().isEmpty()) {
      log.warnv(
          "capture: liveness of {0} did not pass, status={1}", processId, result.get().status());
      failAttempt(context, ScanovateError.LIVENESS_FAILED.messageKey());
      return false;
    }

    byte[] face = result.get().image().get();
    FaceMatchClient client = faceMatchFactory.apply(faceMatch);
    return facesMatch(
            context,
            client,
            faceMatch,
            media.files().get(MediaKind.FRONT_IMAGE).content(),
            face,
            EVENT_DETAIL_FACE_MATCH_DOCUMENT)
        && facesMatch(
            context,
            client,
            faceMatch,
            media.files().get(MediaKind.HOLDING_IMAGE).content(),
            face,
            EVENT_DETAIL_FACE_MATCH_HOLDING);
  }

  /**
   * Compares the voter's live face with a captured photo, recording the similarity in the event.
   * Otherwise, shows the error and returns false.
   */
  private boolean facesMatch(
      AuthenticationFlowContext context,
      FaceMatchClient client,
      FaceMatchSettings settings,
      byte[] photo,
      byte[] face,
      String eventDetail) {
    String processId = context.getAuthenticationSession().getAuthNote(PROCESS_ID_NOTE);
    FaceComparison comparison;
    try {
      comparison = client.compare(photo, face);
    } catch (IOException e) {
      log.error("capture: could not compare the faces with Face Match", e);
      showError(context, ScanovateError.INTERNAL, true);
      return false;
    }
    String similarity = String.format(Locale.ROOT, "%.4f", comparison.similarity());
    context.getEvent().detail(eventDetail, similarity);
    FaceMatchOutcome outcome = comparison.outcome(settings.minSimilarity());
    log.infov(
        "capture: {0} of {1}: {2}, similarity={3}, statuses={4}/{5}",
        eventDetail,
        processId,
        outcome,
        similarity,
        comparison.image1Status(),
        comparison.image2Status());
    switch (outcome) {
      case MATCH -> {
        return true;
      }
      case MISMATCH -> failAttempt(context, ScanovateError.FACE_MISMATCH.messageKey());
      case FACE_NOT_FOUND -> failAttempt(context, ScanovateError.FACE_NOT_FOUND.messageKey());
    }
    return false;
  }

  private void processReturn(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String processId = authSession.getAuthNote(PROCESS_ID_NOTE);
    if (processId == null) {
      log.warn("processReturn: no B-Trust session in progress, starting a new one");
      startVerification(context);
      return;
    }
    authSession.removeAuthNote(PROCESS_ID_NOTE);
    context.getEvent().detail(EVENT_DETAIL_PROCESS_ID, processId);

    JsonNode results;
    try {
      results = clientFactory.apply(config).fetchResultsForProcess(processId);
    } catch (IOException e) {
      log.error("processReturn: could not fetch the B-Trust results", e);
      showError(context, ScanovateError.INTERNAL, true);
      return;
    }

    Optional<ScanovateError> outcomeError = ScanovateResults.outcomeError(results);
    if (outcomeError.isPresent()) {
      log.warnv(
          "processReturn: verification {0} failed: errorCode={1} errorMessage={2}",
          processId,
          results == null ? null : results.at("/data/errorCode").asText(),
          results == null ? null : results.at("/data/errorMessage").asText());
      failAttempt(context, outcomeError.get().messageKey());
      return;
    }

    String docType = documentType(config, authSession);
    List<StoredAttribute> storedAttributes;
    try {
      Optional<JsonNode> validationRules =
          AttributeRules.rulesFor(
              config.get(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE), docType);
      if (validationRules.isEmpty()) {
        log.errorv("processReturn: no validation rules for document type {0}", docType);
        showError(context, ScanovateError.INTERNAL, false);
        return;
      }
      Optional<String> validationError =
          AttributeRules.validate(
              results, validationRules.get(), authSession::getAuthNote, LocalDate.now());
      if (validationError.isPresent()) {
        failAttempt(context, validationError.get());
        return;
      }
      Optional<JsonNode> storeRules =
          AttributeRules.rulesFor(
              config.get(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE), docType);
      storedAttributes =
          storeRules.isPresent() ? AttributeRules.extract(results, storeRules.get()) : List.of();
    } catch (ScanovateException e) {
      log.error("processReturn: could not apply the configured rules", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }

    storedAttributes.forEach(
        attribute -> authSession.setAuthNote(attribute.key(), attribute.value()));
    authSession.setAuthNote(userStatusNote(config), USER_STATUS_VERIFIED);

    if (storedAttributes.isEmpty()) {
      succeed(context);
      return;
    }
    context.challenge(
        context
            .form()
            .setAttribute(
                FTL_STORED_ATTRIBUTES,
                storedAttributes.stream().map(StoredAttribute::toTemplateModel).toList())
            .setAttribute(FTL_DOCUMENT_TYPE, documentTypeName(docType))
            .createForm(CONFIRMATION_FORM));
  }

  private void confirm(AuthenticationFlowContext context) {
    String statusNote = userStatusNote(config(context));
    if (!USER_STATUS_VERIFIED.equals(context.getAuthenticationSession().getAuthNote(statusNote))) {
      log.warn("confirm: received a confirmation without a successful verification");
      startVerification(context);
      return;
    }
    succeed(context);
  }

  private void succeed(AuthenticationFlowContext context) {
    context.getAuthenticationSession().removeAuthNote(ATTEMPTS_NOTE);
    context.getEvent().detail("action", "ScanovateAuthenticator: user verified").success();
    context.success();
  }

  private void failAttempt(AuthenticationFlowContext context, String messageKey) {
    int attempts = attempts(context) + 1;
    context.getAuthenticationSession().setAuthNote(ATTEMPTS_NOTE, String.valueOf(attempts));
    boolean canRetry = attempts < maxAttempts(config(context));
    showError(context, canRetry ? messageKey : ScanovateError.MAX_RETRIES.messageKey(), canRetry);
  }

  private void showError(
      AuthenticationFlowContext context, ScanovateError error, boolean canRetry) {
    showError(context, error.messageKey(), canRetry);
  }

  private void showError(AuthenticationFlowContext context, String messageKey, boolean canRetry) {
    clearVerification(context);
    context.getEvent().detail(EVENT_DETAIL_ERROR, messageKey).error(EVENT_ERROR);

    AuthenticationExecutionModel execution = context.getExecution();
    if (execution != null && (execution.isAlternative() || execution.isConditional())) {
      context.attempted();
      return;
    }
    context.challenge(
        context
            .form()
            .setAttribute(FTL_ERROR, messageKey)
            .setAttribute(FTL_CAN_RETRY, canRetry)
            .setAttribute(FTL_ATTEMPTS_LEFT, attemptsLeft(context, maxAttempts(config(context))))
            .setAttribute(
                FTL_CODE_ID, context.getAuthenticationSession().getParentSession().getId())
            .createForm(ERROR_FORM));
  }

  private void clearVerification(AuthenticationFlowContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String livenessToken = authSession.getAuthNote(LIVENESS_TOKEN_NOTE);
    if (livenessToken != null) {
      livenessFactory.apply(context.getSession()).discard(livenessToken);
      authSession.removeAuthNote(LIVENESS_TOKEN_NOTE);
    }
    String uploadToken = authSession.getAuthNote(CAPTURE_TOKEN_NOTE);
    if (uploadToken != null) {
      uploadsFactory.apply(context.getSession()).discard(uploadToken);
      authSession.removeAuthNote(CAPTURE_TOKEN_NOTE);
    }
    authSession.removeAuthNote(PROCESS_ID_NOTE);
    authSession.removeAuthNote(userStatusNote(config(context)));
  }

  private int attempts(AuthenticationFlowContext context) {
    return parseInt(context.getAuthenticationSession().getAuthNote(ATTEMPTS_NOTE), 0);
  }

  private int attemptsLeft(AuthenticationFlowContext context, int maxAttempts) {
    return Math.max(0, maxAttempts - attempts(context));
  }

  private static Optional<ExecutionMode> executionMode(Map<String, String> config) {
    return ExecutionMode.fromValue(
        config.getOrDefault(
            ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.INTERACTIVE.value()));
  }

  private static boolean isEmbedded(Map<String, String> config) {
    return executionMode(config).filter(ExecutionMode.EMBEDDED::equals).isPresent();
  }

  /** The document type chosen by the voter, or null if the flow did not ask for it. */
  private static String documentType(
      Map<String, String> config, AuthenticationSessionModel authSession) {
    return authSession.getAuthNote(
        config.getOrDefault(
            ScanovateAuthenticatorFactory.DOC_ID_TYPE,
            ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE));
  }

  private static String documentTypeName(String docType) {
    return docType == null ? AttributeRules.DEFAULT_DOC_TYPE : docType;
  }

  private static int maxAttempts(Map<String, String> config) {
    return parseInt(
        config.get(ScanovateAuthenticatorFactory.MAX_ATTEMPTS),
        ScanovateAuthenticatorFactory.DEFAULT_MAX_ATTEMPTS);
  }

  /** Maps each B-Trust flow parameter to the value of the configured auth note. */
  static Map<String, String> linkParams(
      Map<String, String> config, AuthenticationSessionModel authSession)
      throws ScanovateException {
    String mapping = config.get(ScanovateAuthenticatorFactory.LINK_PARAMS);
    Map<String, String> params = new LinkedHashMap<>();
    if (mapping == null || mapping.isBlank()) {
      return params;
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(mapping);
    } catch (IOException e) {
      throw new ScanovateException("Invalid link parameters configuration", e);
    }
    if (!root.isObject()) {
      throw new ScanovateException("Link parameters configuration must be an object");
    }
    for (Map.Entry<String, JsonNode> field : root.properties()) {
      String value = authSession.getAuthNote(field.getValue().asText());
      if (value != null) {
        params.put(field.getKey(), value);
      }
    }
    return params;
  }

  private void buildEventDetails(AuthenticationFlowContext context) {
    Utils.buildEventDetails(
        context.getEvent(),
        context.getAuthenticationSession(),
        context.getUser(),
        context.getSession(),
        getClass().getSimpleName());
  }

  private static String userStatusNote(Map<String, String> config) {
    return config.getOrDefault(
        ScanovateAuthenticatorFactory.USER_STATUS,
        ScanovateAuthenticatorFactory.DEFAULT_USER_STATUS);
  }

  private static Map<String, String> config(AuthenticationFlowContext context) {
    return context.getAuthenticatorConfig() == null
        ? Map.of()
        : context.getAuthenticatorConfig().getConfig();
  }

  static int parseInt(String value, int defaultValue) {
    if (value == null) {
      return defaultValue;
    }
    try {
      return Integer.parseInt(value.trim());
    } catch (NumberFormatException e) {
      return defaultValue;
    }
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public void close() {}
}
