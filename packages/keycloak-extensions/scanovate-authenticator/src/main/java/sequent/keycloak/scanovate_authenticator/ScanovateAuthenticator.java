// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.time.LocalDate;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.voter_enrollment.Utils;

/**
 * Verifies the voter's identity with B-Trust (Scanovate).
 *
 * <p>The voter is redirected to a B-Trust flow and, when they come back, the results are fetched
 * server to server, validated against the configured rules, and the extracted attributes are stored
 * as auth notes for the voter to confirm.
 */
@JBossLog
public class ScanovateAuthenticator implements Authenticator {
  static final String PROCESS_ID_NOTE = "scanovate-process-id";
  static final String ATTEMPTS_NOTE = "scanovate-attempts";
  static final String PROCESS_ID_QUERY_PARAM = "processId";
  static final String FORM_ACTION_PARAM = "action";
  static final String USER_STATUS_VERIFIED = "VERIFIED";
  static final String CONFIRMATION_FORM = "scanovate-confirmation.ftl";
  static final String ERROR_FORM = "scanovate-error.ftl";
  static final String FTL_ERROR = "error";
  static final String FTL_CODE_ID = "code_id";
  static final String FTL_CAN_RETRY = "canRetry";
  static final String FTL_STORED_ATTRIBUTES = "storedAttributes";
  static final String EVENT_ERROR = "scanovate_verification_failed";
  static final String EVENT_DETAIL_ERROR = "scanovate_error";
  static final String EVENT_DETAIL_PROCESS_ID = "scanovate_process_id";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final Function<Map<String, String>, ScanovateClient> clientFactory;

  public ScanovateAuthenticator() {
    this(ScanovateAuthenticator::defaultClient);
  }

  ScanovateAuthenticator(Function<Map<String, String>, ScanovateClient> clientFactory) {
    this.clientFactory = clientFactory;
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

    startVerification(context);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    buildEventDetails(context);
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    Optional<FormAction> formAction = FormAction.fromValue(formData.getFirst(FORM_ACTION_PARAM));

    if (formAction.isEmpty()) {
      processReturn(context);
      return;
    }
    switch (formAction.get()) {
      case CONFIRM -> confirm(context);
      case RETRY -> retry(context);
    }
  }

  private void startVerification(AuthenticationFlowContext context) {
    Map<String, String> config = config(context);
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    clearVerification(context);

    int flowId;
    ExecutionMode mode;
    SaveOption saveOption;
    Map<String, String> params;
    try {
      flowId = Integer.parseInt(config.getOrDefault(ScanovateAuthenticatorFactory.FLOW_ID, ""));
      mode =
          ExecutionMode.fromValue(
                  config.getOrDefault(
                      ScanovateAuthenticatorFactory.EXECUTION_MODE,
                      ExecutionMode.INTERACTIVE.value()))
              .orElseThrow(() -> new ScanovateException("Invalid execution mode"));
      saveOption =
          SaveOption.fromValue(config.get(ScanovateAuthenticatorFactory.SAVE_OPTION))
              .orElseThrow(() -> new ScanovateException("Invalid save option"));
      params = linkParams(config, authSession);
    } catch (NumberFormatException | ScanovateException e) {
      log.error("startVerification: invalid authenticator configuration", e);
      showError(context, ScanovateError.INTERNAL, false);
      return;
    }

    String docIdNote =
        config.getOrDefault(
            ScanovateAuthenticatorFactory.DOC_ID, ScanovateAuthenticatorFactory.DEFAULT_DOC_ID);
    String redirectUrl = context.getActionUrl(context.generateAccessCode()).toString();
    LinkRequest request =
        new LinkRequest(
            flowId,
            UUID.randomUUID().toString(),
            authSession.getAuthNote(docIdNote),
            redirectUrl,
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
    }
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

    String docType =
        authSession.getAuthNote(
            config.getOrDefault(
                ScanovateAuthenticatorFactory.DOC_ID_TYPE,
                ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE));
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
            .setAttribute(FTL_STORED_ATTRIBUTES, storedAttributes)
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

  private void retry(AuthenticationFlowContext context) {
    int maxAttempts = maxAttempts(config(context));
    if (attempts(context) >= maxAttempts) {
      showError(context, ScanovateError.MAX_RETRIES, false);
      return;
    }
    startVerification(context);
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
            .setAttribute(
                FTL_CODE_ID, context.getAuthenticationSession().getParentSession().getId())
            .createForm(ERROR_FORM));
  }

  private void clearVerification(AuthenticationFlowContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    authSession.removeAuthNote(PROCESS_ID_NOTE);
    authSession.removeAuthNote(userStatusNote(config(context)));
  }

  private int attempts(AuthenticationFlowContext context) {
    return parseInt(context.getAuthenticationSession().getAuthNote(ATTEMPTS_NOTE), 0);
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
