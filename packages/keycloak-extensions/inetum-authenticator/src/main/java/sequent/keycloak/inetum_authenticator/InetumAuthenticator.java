// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.text.Collator;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.broker.provider.util.SimpleHttp;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class InetumAuthenticator implements Authenticator, AuthenticatorFactory {
  public static final String PROVIDER_ID = "inetum-authenticator";
  private static final InetumAuthenticator SINGLETON = new InetumAuthenticator();
  private static final String AUTH_NOTE_ATTRIBUTE_ID = "equalAuthnoteAttributeId";
  private static final String INETUM_ATTRIBUTE_PATH = "inetumAttributePath";
  private static final String USER_ATTRIBUTE = "UserAttribute";
  private static final String VALIDATION_ATTRIBUTE_TYPE = "type";
  private static final String VALIDATION_ATTRIBUTE_ERROR = "errorMsg";
  private static final String INTEGER_MIN_VALUE = "intMinValue";
  private static final String EQUAL_VALUE = "equalValue";
  private static final String EQUAL_DATE = "equalDateAuthnoteAttributeId";
  private static final String VALUE_DATE_FORMAT = "valueDateFormat";
  private static final String STORE_DATE_FORMAT = "storeDateFormat";
  private static final String INETUM_DATE_FORMAT = "inetumDateFormat";
  private static final String EXPIRED_DATE = "isBeforeDateValue";
  private static final String NOW = "now";
  public static final String ERROR_FAILED_TO_LOAD_INETUM_FORM = "Failed to load inetumForm";
  public static final String ERROR_TO_CREATE_INETUM_TRANSACTION = "Failed to create transaction";
  public static final String ERROR_TO_GET_INETUM_STATUS_RESPONSE =
      "Failed to get inetum status response";
  public static final String ERROR_TO_GET_INETUM_RESPONSE = "Failed to get inetum response";
  public static final String ERROR_TO_GET_INETUM_RESULTS_RESPONSE =
      "Failed to get inetum results response";
  public static final String ERROR_INVALIDE_CODE = "Invalide Code";
  public static final String ERROR_ATTRIBUTE_VALIDATION = "Attribute Validation Error";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    // Authentication is successful if the user already has the user's
    // validation status attribute set to true, otherwise initiate a new
    // flow and show form
    log.info("authenticate() intetum-authenticator");

    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    UserModel user = context.getUser();
    Utils.buildEventDetails(
        context.getEvent(),
        context.getAuthenticationSession(),
        user,
        context.getSession(),
        this.getClass().getSimpleName());
    if (user != null) {
      String statusAttributeName = configMap.get(Utils.USER_STATUS_ATTRIBUTE);
      String statusAttributeValue = user.getFirstAttribute(statusAttributeName);
      log.info("checking statusAttributeValue=" + statusAttributeValue);
      boolean validated = (statusAttributeValue != null && statusAttributeValue.equals("TRUE"));
      log.info("validated=" + validated);
      if (validated) {
        log.info("validated IS TRUE, pass");
        context
            .getEvent()
            .detail("action", "InetumAuthenticator: User already validated")
            .success();
        context.success();
        return;
      }
    }

    log.info("validated is NOT TRUE, rendering the form");
    try {
      Boolean isTestMode = Boolean.parseBoolean(configMap.get(Utils.TEST_MODE_ATTRIBUTE));
      if (isTestMode) {
        // Make a new transaction request to mock server
        SimpleHttp.Response mockTransactionData =
            doPost(configMap, context, "{}", Utils.API_TRANSACTION_NEW, true);
        JsonNode responseContent = mockTransactionData.asJson().get("response");
        log.info(responseContent);
        String tokenDob = responseContent.get("token_dob").asText();
        String userId = responseContent.get("user_id").asText();

        AuthenticationSessionModel sessionModel = context.getAuthenticationSession();
        sessionModel.setAuthNote(Utils.FTL_TOKEN_DOB, tokenDob);
        sessionModel.setAuthNote(Utils.FTL_USER_ID, userId);

        // Verifying results
        // Getting status
        SimpleHttp.Response response = verifyResults(context, isTestMode);
        log.info("response" + response);
        String error = validateAttributes(context, response);
        storeAttributes(context, response);
        context.success();
      } else {
        Map<String, String> transactionData = newTransaction(configMap, context);
        log.infov(
            "New transaction: TOKEN_DOB: {0} USER_ID: {1}",
            transactionData.get(Utils.FTL_TOKEN_DOB), transactionData.get(Utils.FTL_USER_ID));

        // Save the transaction data into the auth session
        AuthenticationSessionModel sessionModel = context.getAuthenticationSession();
        sessionModel.setAuthNote(Utils.FTL_TOKEN_DOB, transactionData.get(Utils.FTL_TOKEN_DOB));
        sessionModel.setAuthNote(Utils.FTL_USER_ID, transactionData.get(Utils.FTL_USER_ID));

        Response challenge =
            getBaseForm(context)
                .setAttribute(Utils.FTL_USER_ID, transactionData.get(Utils.FTL_USER_ID))
                .setAttribute(Utils.FTL_TOKEN_DOB, transactionData.get(Utils.FTL_TOKEN_DOB))
                .createForm(Utils.INETUM_FORM);
        context.challenge(challenge);
      }

    } catch (IOException error) {
      context.getEvent().error(ERROR_FAILED_TO_LOAD_INETUM_FORM);
      context.failure(AuthenticationFlowError.INTERNAL_ERROR);
      context.attempted();
      Response challenge =
          getBaseForm(context)
              .setAttribute(Utils.FTL_ERROR, Utils.FTL_ERROR_INTERNAL)
              .setAttribute(Utils.CODE_ID, sessionId)
              .createForm(Utils.INETUM_ERROR);
      context.challenge(challenge);
    } catch (InetumException e) {
      // TODO Auto-generated catch block
      e.printStackTrace();
    }
  }

  /** Send a POST to Inetum API */
  protected SimpleHttp.Response doPost(
      Map<String, String> configMap,
      AuthenticationFlowContext context,
      Object payload,
      String uriPath,
      Boolean isTestMode)
      throws IOException {
    String baseUrl =
        isTestMode
            ? configMap.get(Utils.TEST_MODE_SERVER_URL)
            : configMap.get(Utils.BASE_URL_ATTRIBUTE);
    String url = baseUrl + uriPath;
    String authorization = "Bearer " + configMap.get(Utils.API_KEY_ATTRIBUTE);
    log.info("doPost: url=" + url + ", payload =" + payload.toString());

    var attempt = 0;
    int maxRetries = Utils.parseInt(configMap.get(Utils.MAX_RETRIES), Utils.DEFAULT_MAX_RETRIES);
    int baseRetryDelay = Utils.BASE_RETRY_DELAY;

    while (attempt < maxRetries) {
      try {
        SimpleHttp.Response response =
            SimpleHttp.doPost(url, context.getSession())
                .header("Content-Type", "application/json")
                .header("Authorization", authorization)
                .json(payload)
                .asResponse();
        return response;

      } catch (IOException e) {
        attempt++;
        log.warnv("doPost: Request failed (attempt {0}): {1}", attempt, e.getMessage());
        context
            .getEvent()
            .error(
                String.format(
                    "%s - After %s attempts, max attemtps is %s with error message: %.100s",
                    ERROR_TO_CREATE_INETUM_TRANSACTION, attempt, maxRetries, e.getMessage()));
        if (attempt >= maxRetries) {
          throw e; // Propagate the exception if max retries are reached
        }

        // Wait before retrying
        sleep(baseRetryDelay, attempt);
      }
    }
    context.getEvent().error(ERROR_TO_CREATE_INETUM_TRANSACTION + "Max retries reached");
    throw new IOException("doPost: Failed to execute request after " + maxRetries + " attempts.");
  }

  /** Send a GET to Inetum API */
  protected SimpleHttp.Response doGet(
      Map<String, String> configMap,
      AuthenticationFlowContext context,
      String uriPath,
      Boolean isTestMode)
      throws IOException {
    String baseUrl =
        isTestMode
            ? configMap.get(Utils.TEST_MODE_SERVER_URL)
            : configMap.get(Utils.BASE_URL_ATTRIBUTE);
    String url = baseUrl + uriPath;
    String authorization = "Bearer " + configMap.get(Utils.API_KEY_ATTRIBUTE);
    log.info("doGet: url=" + url);

    var attempt = 0;
    int maxRetries = Utils.parseInt(configMap.get(Utils.MAX_RETRIES), Utils.DEFAULT_MAX_RETRIES);
    int baseRetryDelay = Utils.BASE_RETRY_DELAY;

    while (attempt < maxRetries) {
      try {
        SimpleHttp.Response response =
            SimpleHttp.doGet(url, context.getSession())
                .header("Content-Type", "application/json")
                .header("Authorization", authorization)
                .asResponse();

        return response;

      } catch (IOException e) {
        attempt++;
        log.warnv("doGet: Request failed (attempt {0}): {1}", attempt, e.getMessage());
        context
            .getEvent()
            .error(
                String.format(
                    "%s - After %s attempts, max attemtps is %s with error message: %.100s",
                    ERROR_TO_GET_INETUM_RESPONSE, attempt, maxRetries, e.getMessage()));
        if (attempt >= maxRetries) {
          throw e; // Propagate the exception if max retries are reached
        }

        // Wait before retrying
        sleep(baseRetryDelay, attempt);
      }
    }
    context.getEvent().error(ERROR_TO_GET_INETUM_RESPONSE + "Max retries reached");
    throw new IOException("doGet: Failed to execute request after " + maxRetries + " attempts.");
  }

  protected Map<String, String> getTemplateMap(Map<String, String> configMap) {
    Map<String, String> attributes = new HashMap<String, String>();
    attributes.put(Utils.FTL_API_KEY, configMap.get(Utils.API_KEY_ATTRIBUTE));
    attributes.put(Utils.FTL_APP_ID, configMap.get(Utils.APP_ID_ATTRIBUTE));
    attributes.put(Utils.FTL_CLIENT_ID, configMap.get(Utils.CLIENT_ID_ATTRIBUTE));
    attributes.put(Utils.FTL_BASE_URL, configMap.get(Utils.BASE_URL_ATTRIBUTE));
    attributes.put(Utils.FTL_ENV_CONFIG, configMap.get(Utils.ENV_CONFIG_ATTRIBUTE));
    attributes.put(Utils.FTL_DOC_ID, configMap.get(Utils.DOC_ID_ATTRIBUTE));
    return attributes;
  }

  protected JsonNode renderJsonTemplate(
      String sourceCode, Map<String, String> configMap, Map<String, String> extraAttributes)
      throws Exception {
    ObjectMapper mapper = new ObjectMapper();
    Map<String, String> attributes = getTemplateMap(configMap);
    if (extraAttributes != null) {
      attributes.putAll(extraAttributes);
    }
    String stringPayload =
        Utils.processStringTemplate(attributes, configMap.get(Utils.TRANSACTION_NEW_ATTRIBUTE));
    JsonNode jsonPayload = mapper.readValue(stringPayload, JsonNode.class);
    return jsonPayload;
  }

  private Map<String, String> getAuthNotesMap(
      Map<String, String> configMap, AuthenticationFlowContext context) {
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();
    Map<String, String> map = new HashMap<String, String>();

    String docIdAttributeName = configMap.get(Utils.DOC_ID_ATTRIBUTE);
    map.put(Utils.FTL_DOC_ID, sessionModel.getAuthNote(docIdAttributeName));
    String docIdTypeAttributeName = configMap.get(Utils.DOC_ID_TYPE_ATTRIBUTE);
    map.put(Utils.FTL_DOC_ID_TYPE, sessionModel.getAuthNote(docIdTypeAttributeName));

    return map;
  }

  /** Start a new Inetum transaction */
  protected Map<String, String> newTransaction(
      Map<String, String> configMap, AuthenticationFlowContext context) throws IOException {
    JsonNode jsonPayload = null;
    Map<String, String> authNotesMap = getAuthNotesMap(configMap, context);

    try {
      jsonPayload =
          renderJsonTemplate(
              configMap.get(Utils.TRANSACTION_NEW_ATTRIBUTE), configMap, authNotesMap);
    } catch (Exception error) {
      log.error("newTransaction: Error rendering template", error);
      context.getEvent().error(ERROR_FAILED_TO_LOAD_INETUM_FORM);
      throw new IOException(error);
    }

    try {
      SimpleHttp.Response response =
          doPost(configMap, context, jsonPayload, Utils.API_TRANSACTION_NEW, false);

      if (response.getStatus() != 200) {
        log.error(
            "newTransaction: Error calling transaction/new, status = " + response.getStatus());
        log.error(
            "newTransaction: Error calling transaction/new, response.asString() = "
                + response.asString());
        throw new IOException("Error calling transaction/new, status = " + response.getStatus());
      }

      JsonNode responseContent = response.asJson().get("response");
      Map<String, String> output = new HashMap<String, String>();
      output.put(Utils.FTL_TOKEN_DOB, responseContent.get("tokenDob").asText());
      output.put(Utils.FTL_USER_ID, responseContent.get("userID").asText());
      return output;
    } catch (IOException error) {
      log.error("Error calling transaction/new", error);
      throw error;
    }
  }

  private void sleep(int delay, int attempt) throws IOException {
    // Wait before retrying
    try {
      Double interval = delay * Math.pow(2, attempt);
      log.infov("sleep: Sleeping {0} ms, delay={1}, attempt={2}", interval, delay, attempt);
      Thread.sleep((int) Math.round(interval));
      log.infov("sleep: Slept {0} ms, delay={1}, attempt={2}", interval, delay, attempt);
    } catch (InterruptedException interruptedException) {
      Thread.currentThread().interrupt();
      throw new IOException("doGet: Retry interrupted", interruptedException);
    }
  }

  int action_retries = 0;

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action(): start inetum-authenticator");

    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    String action = formData.getFirst("action");
    log.infov("action(): Get action from request {0}", action);

    // Check if user has confirmed data
    if ("confirm".equals(action)) {
      log.info("action(): success");
      // valid
      context
          .getEvent()
          .detail("action", "InetumAuthenticator: User validated successfully")
          .success();

      context.success();
      return;
    } else if (action == null) {
      // Retrieve error details from form
      String errorCode = formData.getFirst("error_code");
      log.errorv("Received error from form: Code={0}", errorCode);

      // Handle uploadAndCheckException error properly
      if ("uploadAndCheckException".equals(errorCode)) {
        String sessionId = context.getAuthenticationSession().getParentSession().getId();

        Response challenge =
            getBaseForm(context)
                .setAttribute(Utils.FTL_ERROR, Utils.UPLOAD_AND_CHECK_EXCEPTION)
                .setAttribute(Utils.CODE_ID, sessionId)
                .createForm(Utils.INETUM_ERROR);

        context.challenge(challenge);
        return;
      }
    }

    UserModel user = context.getUser();
    Utils.buildEventDetails(
        context.getEvent(),
        context.getAuthenticationSession(),
        user,
        context.getSession(),
        this.getClass().getSimpleName());
    SimpleHttp.Response result = verifyResults(context, false);
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    if (result == null) {
      // invalid
      AuthenticationExecutionModel execution = context.getExecution();
      if (execution.isRequired()) {
        // context.failureChallenge(
        // AuthenticationFlowError.INVALID_CREDENTIALS,
        // getBaseForm(context)
        // .setError(Utils.FTL_ERROR_AUTH_INVALID)
        // .createForm(Utils.INETUM_ERROR)
        // );
        context.failure(AuthenticationFlowError.INVALID_CREDENTIALS);
        context.attempted();
        Response challenge =
            getBaseForm(context)
                .setAttribute(Utils.FTL_ERROR, Utils.FTL_ERROR_AUTH_INVALID)
                .setAttribute(Utils.CODE_ID, sessionId)
                .createForm(Utils.INETUM_ERROR);
        context.challenge(challenge);
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
      return;
    }

    String error = validateAttributes(context, result);

    if (error != null) {
      action_retries++;
      log.error(
          "action(): The submitted form data does not correspond with the ones provided by Inetum.");
      // invalid
      AuthenticationExecutionModel execution = context.getExecution();
      if (execution.isRequired()) {
        if (action_retries == 3) {
          error = "maxRetriesError";
          Response challenge =
              getBaseForm(context)
                  .setAttribute(Utils.FTL_ERROR, error)
                  .setAttribute(Utils.CODE_ID, sessionId)
                  .createForm(Utils.INETUM_ERROR);
          context.challenge(challenge);
          return;
        }
        context.failure(AuthenticationFlowError.INVALID_CREDENTIALS);
        context.attempted();
        Response challenge =
            getBaseForm(context)
                .setAttribute(Utils.FTL_ERROR, error)
                .setAttribute(Utils.CODE_ID, sessionId)
                .createForm(Utils.INETUM_ERROR);
        context.challenge(challenge);
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
      return;
    }

    List<HashMap<String, String>> storedAttributes;
    try {
      storedAttributes = storeAttributes(context, result);
    } catch (InetumException exception) {
      exception.printStackTrace();

      log.error("action(): Error storing data obtained from inetum");
      // invalid
      AuthenticationExecutionModel execution = context.getExecution();
      if (execution.isRequired()) {
        context.failure(AuthenticationFlowError.INVALID_CREDENTIALS);
        context.attempted();
        Response challenge =
            getBaseForm(context)
                .setAttribute(Utils.FTL_ERROR, exception.getError())
                .setAttribute(Utils.CODE_ID, sessionId)
                .createForm(Utils.INETUM_ERROR);
        context.challenge(challenge);
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
      return;
    }
    if (!storedAttributes.isEmpty()) {
      log.infov("action(): SHOW CONFIRM!!!! {0}", action);
      // Manually construct the action URL for the form
      String actionUrl = context.getActionUrl(context.generateAccessCode()).toString();

      Response challenge =
          getBaseForm(context)
              .setAttribute("actionUrl", actionUrl)
              .setAttribute("storedAttributes", storedAttributes)
              .createForm(Utils.INETUM_CONFIRM);
      context.challenge(challenge);
      return;
    }

    log.info("action(): success");
    // valid
    context
        .getEvent()
        .detail("action", "InetumAuthenticator: User validated successfully")
        .success();
    context.success();
  }

  private List<HashMap<String, String>> storeAttributes(
      AuthenticationFlowContext context, SimpleHttp.Response response) throws InetumException {
    log.info("storeAttributes: start");

    List<HashMap<String, String>> storedAttributes = new ArrayList<>();

    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();

    String docIdTypeAttributeName = configMap.get(Utils.DOC_ID_TYPE_ATTRIBUTE);
    String docIdType = context.getAuthenticationSession().getAuthNote(docIdTypeAttributeName);

    String attributesToStore = configMap.get(Utils.ATTRIBUTES_TO_STORE);
    log.infov("storeAttributes: attributes to store configuration: {0}", attributesToStore);
    JsonNode attributesToCheck = null;

    if (attributesToStore != null) {
      log.infov("storeAttributes: docIdType {0}", docIdType);

      try {
        // Read the attributes to check from the configuration depending on the ID Type
        attributesToCheck = new ObjectMapper().readTree(attributesToStore).get(docIdType);
      } catch (Exception exception) {
        throw new InetumException(Utils.FTL_ERROR_AUTH_INVALID);
      }

      if (attributesToCheck != null) {
        for (JsonNode attributeToStore : attributesToCheck) {
          String storedValue;
          HashMap<String, String> attributeMap = new HashMap<>();

          // Get inetum path from config
          String inetumField = attributeToStore.get(INETUM_ATTRIBUTE_PATH).asText();
          log.infov("storeAttributes: inetumField {0}", inetumField);

          String attribute = attributeToStore.get(USER_ATTRIBUTE).asText();
          log.infov("storeAttributes: attribute {0}", attribute);

          String type = attributeToStore.get(VALIDATION_ATTRIBUTE_TYPE).asText();
          log.infov("storeAttributes: type {0}", type);

          if (type == null || type.isBlank()) {
            log.errorv("storeAttributes: could not find attribute type {0}", attributeToStore);
            throw new InetumException(Utils.FTL_ERROR_AUTH_INVALID);
          }

          // Get OCR value from response
          String inetumValue = getValueFromInetumResponse(response, inetumField);
          log.infov("storeAttributes: inetumValue {0}", inetumField);

          if (inetumValue == null) {
            // Give a warning that the value was not found in inetun response.
            log.warnv(
                "storeAttributes: could not find value in inetum response {0}. Setting value to empty string",
                inetumField);

            // Don't fail, just set the value to empty.
            inetumValue = "";
          }

          switch (type) {
            case "text":
              storedValue = inetumValue;
              break;

            case "date":
              LocalDate inetumDate = getDate(attributeToStore, INETUM_DATE_FORMAT, inetumValue);
              log.infov("storeAttributes: inetumDate {0}", inetumDate);

              String storeDateFormat = attributeToStore.get(STORE_DATE_FORMAT).asText();
              log.infov("storeAttributes: storeDateFormat {0}", type);

              DateTimeFormatter valueFormat = DateTimeFormatter.ofPattern(storeDateFormat);
              storedValue = (inetumDate != null) ? inetumDate.format(valueFormat) : null;
              break;

            default:
              storedValue = inetumValue;
              break;
          }

          sessionModel.setAuthNote(attribute, storedValue);
          attributeMap.put("key", attribute);
          attributeMap.put("value", storedValue);
          attributeMap.put("type", type);

          storedAttributes.add(attributeMap);
        }
      } else {
        log.info("storeAttributes: Empty configuration provided. No attributes checked.");
      }
    }

    log.info("storeAttributes: success");
    return storedAttributes;
  }

  /*
   * Calls Inetum API results/get and verify results
   */
  protected SimpleHttp.Response verifyResults(
      AuthenticationFlowContext context, Boolean isTestMode) {
    log.info("verifyResults: start");

    // Get the transaction data from the auth session
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();
    String tokenDob = sessionModel.getAuthNote(Utils.FTL_TOKEN_DOB);
    String userId = sessionModel.getAuthNote(Utils.FTL_USER_ID);
    if (tokenDob == null || userId == null) {
      log.info("verifyResults: TRUE; tokenDob == null || userId == null");
      return null;
    }
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    String uriPath = isTestMode ? "/status" : "/transaction/" + userId + "/status?t=" + tokenDob;
    SimpleHttp.Response response = null;

    var attempt = 0;
    int maxRetries = Utils.parseInt(configMap.get(Utils.MAX_RETRIES), Utils.DEFAULT_MAX_RETRIES);
    int baseRetryDelay = Utils.BASE_RETRY_DELAY;

    try {
      while (attempt < maxRetries) {
        response = doGet(configMap, context, uriPath, isTestMode);
        int responseStatus = response.getStatus();
        int code = 0;
        String idStatus = null;

        if (responseStatus != 200) {
          log.errorv(
              "verifyResults (attempt {0}): Error calling transaction/status, status = {1}",
              attempt, responseStatus);
          log.errorv(
              "verifyResults (attempt {0}): Error calling transaction/status, response.asString() = {1}",
              attempt, response.asString());
          attempt++;
          context
              .getEvent()
              .error(
                  String.format(
                      "%s - Error status: %s After %s attempts, max attemtps is %s, with response: %.100s",
                      ERROR_TO_GET_INETUM_STATUS_RESPONSE,
                      responseStatus,
                      attempt,
                      maxRetries,
                      response.asString()));
          if (attempt >= maxRetries) {
            context.getEvent().error(ERROR_TO_GET_INETUM_STATUS_RESPONSE + ", Max retries reached");
            throw new IOException(
                "Too many attempts on transaction/status, bad status=" + responseStatus);
          } else {
            log.errorv("verifyResults (attempt {0}): Will retry again", attempt);
            // Wait before retrying
            sleep(baseRetryDelay, attempt);
            continue;
          }
        }
        log.info("verifyResults: response = " + response.asString());
        code = response.asJson().get("code").asInt();
        if (code != 0) {
          log.errorv(
              "verifyResults (attempt {0}): Error calling transaction/status, code = {1}",
              attempt, code);
          log.errorv(
              "verifyResults (attempt {0}): Error calling transaction/status, response.asString() = {1}",
              attempt, response.asString());
          context
              .getEvent()
              .error(
                  String.format(
                      "%s: %s - error code: %s After %s attempts, max attemtps is %s, with error response: %.100s",
                      ERROR_TO_GET_INETUM_STATUS_RESPONSE,
                      ERROR_INVALIDE_CODE,
                      code,
                      attempt,
                      maxRetries,
                      response.asString()));
          attempt++;
          if (attempt >= maxRetries) {
            context.getEvent().error(ERROR_TO_GET_INETUM_STATUS_RESPONSE + ", Max retries reached");
            throw new IOException("Too many attempts on transaction/status, bad code = " + code);
          } else {
            log.errorv("verifyResults (attempt {0}): Will retry again", attempt);
            // Wait before retrying
            sleep(baseRetryDelay, attempt);
            continue;
          }
        }

        // check that inetum has already verified the data, or else retry
        // again after a delay
        idStatus = response.asJson().get("response").get("idStatus").asText();
        log.infov(
            "verifyResults (attempt {0}): transaction/status, idStatus = {1}", attempt, idStatus);

        if (!idStatus.equals("verificationOK") && !idStatus.equals("verificationKO")) {
          log.errorv(
              "verifyResults (attempt {0}): incorrect idStatus = {1} in transaction/status",
              attempt, idStatus);
          log.errorv(
              "verifyResults (attempt {0}): Error calling transaction/status, response.asString() = {1}",
              attempt, response.asString());
          context
              .getEvent()
              .error(
                  String.format(
                      "%s - Incorrect idStatus: %s with response %s After %s attempts, max attemtps is %s",
                      ERROR_TO_GET_INETUM_STATUS_RESPONSE,
                      idStatus,
                      response.toString(),
                      attempt,
                      maxRetries));
          attempt++;
          if (attempt >= maxRetries) {
            context.getEvent().error(ERROR_TO_GET_INETUM_STATUS_RESPONSE + ", Max retries reached");
            throw new IOException(
                "Too many attempts on transaction/status, bad idStatus = " + idStatus);
          } else {
            log.errorv("verifyResults (attempt {0}): Will retry again", attempt);
            // Wait before retrying
            sleep(baseRetryDelay, attempt);
            continue;
          }
        }

        // Everything good, so we break the loop
        break;
      }

      // The status is verification OK. Now we need to retrieve the
      // information
      String country = context.getAuthenticationSession().getAuthNote("country");
      String encodedCountry = URLEncoder.encode(country, StandardCharsets.UTF_8);
      uriPath =
          isTestMode ? "/results?country=" + encodedCountry : "/transaction/" + userId + "/results";
      response = doGet(configMap, context, uriPath, isTestMode);
      if (response.getStatus() != 200) {
        log.error(
            "verifyResults: Error calling transaction/results, status = " + response.getStatus());
        log.error(
            "verifyResults: Error calling transaction/results, response.asString() = "
                + response.asString());
        context
            .getEvent()
            .error(
                ERROR_TO_GET_INETUM_RESULTS_RESPONSE
                    + String.format(
                        " with error status: %s, response: %.100s",
                        response.getStatus(), response.asString()));
        return null;
      }

      int code = response.asJson().get("code").asInt();
      if (code != 0) {
        log.error("verifyResults: Error calling transaction/results, code = " + code);

        context
            .getEvent()
            .error(
                ERROR_TO_GET_INETUM_RESULTS_RESPONSE
                    + ": "
                    + ERROR_INVALIDE_CODE
                    + String.format(" with error code: %s", code));
        return null;
      }
      String responseStr = response.asString();
      log.info("verifyResults: response Str = " + responseStr);

      log.info("verifyResults: TRUE");

      sessionModel.setAuthNote(
          configMap.get(Utils.USER_STATUS_ATTRIBUTE), Utils.USER_STATUS_VERIFIED);

      return response;
    } catch (IOException error) {
      log.error("verifyResults(): FALSE; Exception: " + error.toString());
      return null;
    }
  }

  private String validateAttributes(
      AuthenticationFlowContext context, SimpleHttp.Response response) {
    log.info("validateAttributes: start");
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    String docIdTypeAttributeName = configMap.get(Utils.DOC_ID_TYPE_ATTRIBUTE);
    String docIdType = context.getAuthenticationSession().getAuthNote(docIdTypeAttributeName);

    String attributesToValidate = configMap.get(Utils.ATTRIBUTES_TO_VALIDATE);
    log.infov(
        "validateAttributes: attributes to validate configuration: {0}", attributesToValidate);
    JsonNode attributesToCheck = null;

    if (attributesToValidate != null) {
      log.infov("validateAttributes: attributesToValidate {0}", attributesToValidate);
      log.infov("validateAttributes: docIdType {0}", docIdType);

      try {
        // Read the attributes to check from the configuration depending on the ID Type
        attributesToCheck = new ObjectMapper().readTree(attributesToValidate).get(docIdType);
      } catch (Exception exception) {
        return Utils.FTL_ERROR_AUTH_INVALID;
      }

      if (attributesToCheck != null) {
        for (JsonNode attributeToCheck : attributesToCheck) {
          // Get inetum path from config
          String inetumField = attributeToCheck.get(INETUM_ATTRIBUTE_PATH).asText();
          log.infov("validateAttributes: inetumField {0}", inetumField);

          // Get OCR value from response
          String inetumValue = getValueFromInetumResponse(response, inetumField);

          if (inetumValue == null) {
            log.errorv(
                "validateAttributes: could not find value in inetum response {0}", inetumField);
            return Utils.FTL_ERROR_AUTH_INVALID;
          }

          String type = attributeToCheck.get(VALIDATION_ATTRIBUTE_TYPE).asText();
          log.infov("validateAttributes: type {0}", type);

          if (type == null || type.isBlank()) {
            return Utils.FTL_ERROR_AUTH_INVALID;
          }

          String typeError = attributeToCheck.get(VALIDATION_ATTRIBUTE_ERROR).asText();
          log.infov("validateAttributes: error {0}", typeError);

          if (typeError == null || typeError.isBlank()) {
            typeError = Utils.FTL_ERROR_AUTH_INVALID;
          }

          String attribute = attributeToCheck.get(type).asText();
          log.infov("validateAttributes: attribute {0}", type);

          if (typeError == null || type.isBlank()) {
            typeError = Utils.FTL_ERROR_AUTH_INVALID;
          }

          String validationError = null;

          switch (type) {
            case AUTH_NOTE_ATTRIBUTE_ID:
              validationError =
                  checkAuthnoteEquals(
                      context, attributeToCheck, attribute, typeError, inetumValue, inetumField);
              break;
            case INTEGER_MIN_VALUE:
              validationError =
                  integerMinValue(
                      context, attributeToCheck, attribute, typeError, inetumValue, inetumField);
              break;
            case EQUAL_VALUE:
              validationError =
                  equalValue(
                      context, attributeToCheck, attribute, typeError, inetumValue, inetumField);
              break;
            case EQUAL_DATE:
              validationError =
                  equalDate(
                      context, attributeToCheck, attribute, typeError, inetumValue, inetumField);
              break;
            case EXPIRED_DATE:
              validationError =
                  isBeforeDate(
                      context, attributeToCheck, attribute, typeError, inetumValue, inetumField);
              break;
            default:
              log.warnv("validateAttributes: Unknow validation {0}. Ignoring validation.", type);
          }

          if (validationError != null) {
            return validationError;
          }
        }
      } else {
        log.info("validateAttributes: Empty configuration provided. No attributes checked.");
      }
    }

    log.info("validateAttributes: success");
    return null;
  }

  private String equalDate(
      AuthenticationFlowContext context,
      JsonNode attributeToCheck,
      String attributeId,
      String typeError,
      String inetumValue,
      String inetumField) {
    log.info("equalDate: start");

    // Get attribute value from authentication notes
    String attributeValue = context.getAuthenticationSession().getAuthNote(attributeId);
    log.infov("equalDate: attributeValue {0}", attributeValue);

    if (attributeValue == null) {
      log.errorv("equalDate: could not find value in auth notes {0}", attributeId);
      return typeError;
    }

    LocalDate valueDate = getDate(attributeToCheck, VALUE_DATE_FORMAT, attributeValue);
    LocalDate inetumDate = getDate(attributeToCheck, INETUM_DATE_FORMAT, inetumValue);

    if (!valueDate.isEqual(inetumDate)) {
      log.error("equalDate: FALSE");
      context
          .getEvent()
          .error(
              ERROR_ATTRIBUTE_VALIDATION
                  + ": "
                  + String.format(
                      "invalid date - value date is %s and inetum date is %s",
                      valueDate, inetumDate));
      return typeError;
    }

    log.info("equalDate: success");
    return null;
  }

  private String isBeforeDate(
      AuthenticationFlowContext context,
      JsonNode attributeToCheck,
      String attributeValue,
      String typeError,
      String inetumValue,
      String inetumField) {
    log.info("equalDate: start");

    LocalDate valueDate = null;
    // If now is provided use current date.
    if (NOW.equalsIgnoreCase(attributeValue)) {
      log.info("equalDate: valueDate set to now");
      valueDate = LocalDate.now();
    } else {
      valueDate = getDate(attributeToCheck, VALUE_DATE_FORMAT, attributeValue);
    }
    LocalDate inetumDate = getDate(attributeToCheck, INETUM_DATE_FORMAT, inetumValue);

    if (!valueDate.isBefore(inetumDate)) {
      log.error("equalDate: FALSE");
      context.getEvent().error(ERROR_ATTRIBUTE_VALIDATION + ": " + "invalide date");
      return typeError;
    }

    log.info("equalDate: success");
    return null;
  }

  private LocalDate getDate(JsonNode attributeToCheck, String format, String dateValue) {
    // wrap it in a try/catch since the date parsing might fail
    try {
      String valuePattern = attributeToCheck.get(format).asText();
      log.infov("getDate: valuePattern {0}", valuePattern);
      if (valuePattern == null || valuePattern.isBlank()) {
        valuePattern = Utils.FTL_ERROR_AUTH_INVALID;
      }
      DateTimeFormatter valueFormat = DateTimeFormatter.ofPattern(valuePattern);
      LocalDate valueDate = LocalDate.parse(dateValue, valueFormat);
      log.infov("getDate: valueDate {0}", valueDate);
      return valueDate;
    } catch (Exception err) {
      log.info("getDate: error parsing, returning null");
      return null;
    }
  }

  private String equalValue(
      AuthenticationFlowContext context,
      JsonNode attributeToCheck,
      String attributeValue,
      String typeError,
      String inetumValue,
      String inetumField) {
    log.info("equalValue: start");

    // Compare and return false if different
    Collator collator = Collator.getInstance();
    collator.setDecomposition(2);
    collator.setStrength(0);

    if (collator.compare(attributeValue.trim(), inetumValue.trim()) != 0) {
      String errorMessage =
          String.format(
              "attribute %s with value %s  does not match OCR value %s",
              inetumField, attributeValue, inetumValue);
      log.errorv(
          "equalValue: FALSE; attribute: {0}, inetumField: {1}, attributeValue: {2}, inetumValue: {3}",
          attributeValue, inetumField, attributeValue, inetumValue);
      context.getEvent().error(ERROR_ATTRIBUTE_VALIDATION + ": " + errorMessage);
      return typeError;
    }

    log.info("equalValue: success");
    return null;
  }

  private String integerMinValue(
      AuthenticationFlowContext context,
      JsonNode attributeToCheck,
      String attributeValue,
      String typeError,
      String inetumValue,
      String inetumField) {
    log.info("integerMinValue: start");

    int minValue = Integer.parseInt(attributeValue);
    log.infov("integerMinValue: minValue {0}", minValue);

    int intInetumValue = Integer.parseInt(inetumValue);
    log.infov("integerMinValue: intInetumValue {0}", minValue);

    if (intInetumValue < minValue) {
      context
          .getEvent()
          .error(
              ERROR_ATTRIBUTE_VALIDATION
                  + ": "
                  + String.format(
                      "Calculated score %s is less than minimum required of %s",
                      inetumValue, minValue));
      return typeError;
    }

    log.info("integerMinValue: success");
    return null;
  }

  private String checkAuthnoteEquals(
      AuthenticationFlowContext context,
      JsonNode attributeToCheck,
      String attributeId,
      String typeError,
      String inetumValue,
      Object inetumField) {
    log.info("checkAuthnoteEquals: start");
    // Get attribute value from authentication notes
    String attributeValue = context.getAuthenticationSession().getAuthNote(attributeId);
    log.infov("checkAuthnoteEquals: attributeValue {0}", attributeValue);

    if (attributeValue == null) {
      log.errorv("checkAuthnoteEquals: could not find value in auth notes {0}", attributeId);
      return typeError;
    }

    // Compare and return false if different
    Collator collator = Collator.getInstance();
    collator.setDecomposition(2);
    collator.setStrength(0);

    if (collator.compare(attributeValue.trim(), inetumValue.trim()) != 0) {
      String errorMessage =
          String.format(
              "attribute %s with value %.100s does not match OCR value %s",
              attributeId, attributeValue, inetumValue);
      log.errorv(
          "checkAuthnoteEquals: FALSE; attribute: {0}, inetumField: {1}, attributeValue: {2}, inetumValue: {3}",
          attributeId, inetumField, attributeValue, inetumValue);
      context.getEvent().error(ERROR_ATTRIBUTE_VALIDATION + ": " + errorMessage);
      return typeError;
    }

    log.info("checkAuthnoteEquals: start");
    return null;
  }

  private String getValueFromInetumResponse(SimpleHttp.Response response, String inetumField) {
    String inetumValue = null;
    try {
      inetumValue = response.asJson().at(inetumField).asText();
    } catch (Exception error) {
      log.warnv("getValueFromInetumResponse: Could not get value: {0}", error.getMessage());
    }

    log.infov("getValueFromInetumResponse: {0}: {1}", inetumField, inetumValue);
    return inetumValue;
  }

  protected LoginFormsProvider getBaseForm(AuthenticationFlowContext context) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    Map<String, String> authNotesMap = getAuthNotesMap(configMap, context);
    return context
        .form()
        .setAttribute(Utils.FTL_REALM, context.getRealm())
        .setAttribute(Utils.FTL_API_KEY, configMap.get(Utils.API_KEY_ATTRIBUTE))
        .setAttribute(Utils.FTL_APP_ID, configMap.get(Utils.APP_ID_ATTRIBUTE))
        .setAttribute(Utils.FTL_CLIENT_ID, configMap.get(Utils.CLIENT_ID_ATTRIBUTE))
        .setAttribute(Utils.FTL_BASE_URL, configMap.get(Utils.BASE_URL_ATTRIBUTE))
        .setAttribute(Utils.FTL_ENV_CONFIG, configMap.get(Utils.ENV_CONFIG_ATTRIBUTE))
        .setAttribute(Utils.FTL_SDK_VERSION, configMap.get(Utils.SDK_VERSION))
        .setAttribute(Utils.FTL_DOC_ID, authNotesMap.get(Utils.FTL_DOC_ID))
        .setAttribute(Utils.FTL_DOC_ID_TYPE, authNotesMap.get(Utils.FTL_DOC_ID_TYPE));
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return false;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Inetum Authentication";
  }

  @Override
  public String getHelpText() {
    return "Validates the User using Inetum Platform.";
  }

  @Override
  public String getReferenceCategory() {
    return "External Authenticator";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return true;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            Utils.API_KEY_ATTRIBUTE, "API KEY", "-", ProviderConfigProperty.STRING_TYPE, ""),
        new ProviderConfigProperty(
            Utils.APP_ID_ATTRIBUTE, "APP ID", "-", ProviderConfigProperty.STRING_TYPE, ""),
        new ProviderConfigProperty(
            Utils.CLIENT_ID_ATTRIBUTE, "CLIENT ID", "-", ProviderConfigProperty.STRING_TYPE, ""),
        new ProviderConfigProperty(
            Utils.DOC_ID_ATTRIBUTE,
            "User Data Attribute",
            "The name of the user data attribute to check against, and name of the auth note to be set.",
            ProviderConfigProperty.STRING_TYPE,
            "sequent.read-only.id-card-number"),
        new ProviderConfigProperty(
            Utils.DOC_ID_TYPE_ATTRIBUTE,
            "User Data Type Attribute",
            "The name of the user data attribute to check against for data type, and name of the auth note to be set.",
            ProviderConfigProperty.STRING_TYPE,
            "sequent.read-only.id-card-type"),
        new ProviderConfigProperty(
            Utils.USER_STATUS_ATTRIBUTE,
            "User Status Attribute",
            "The name of the user validation status attribute.",
            ProviderConfigProperty.STRING_TYPE,
            "sequent.read-only.id-card-number-validated"),
        new ProviderConfigProperty(
            Utils.ATTRIBUTES_TO_STORE,
            "Attributes to store from inetum data",
            "A Json where it's fields represent every id available. For each id a list of attributes to store need to be provided. With UserAttribute to indicate the user profile attribute and inetumAttributePath to indicate the path to get the attribute from the inetum response.",
            ProviderConfigProperty.TEXT_TYPE,
            """
                {
                    "PhilSys ID": [
                        {
                            "UserAttribute": "firstName",
                            "inetumAttributePath": "/response/mrz/given_names",
                            "type": "text"
                        },
                        {
                            "UserAttribute": "lastName",
                            "inetumAttributePath": "/response/mrz/surname",
                            "type": "text"
                        },
                        {
                            "UserAttribute": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "type": "text"
                        },
                        {
                            "UserAttribute": "dateOfBirth",
                            "inetumAttributePath": "/response/mrz/date_of_birth",
                            "type": "date",
                            "storeDateFormat": "yyyy-MM-dd",
                            "inetumDateFormat": "dd/MM/yyyy"
                        }
                    ],
                    "Seaman's Book": [
                        {
                            "UserAttribute": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "type": "text"
                        }
                    ],
                    "Philippine Passport": [
                        {
                            "UserAttribute": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "type": "text"
                        }
                    ]
                }
                """),
        new ProviderConfigProperty(
            Utils.ATTRIBUTES_TO_VALIDATE,
            "Attributes to validate using inetum data",
            "A Json where it's fields represent every id available. For each id a list of attributes to check need to be provided. With authnoteAttributeId to indicate the user profile attribute and inetumAttributePath to indicate the path to get the attribute from the inetum response.",
            ProviderConfigProperty.TEXT_TYPE,
            """
                {
                    "PhilSys ID": [
                        {
                            "type": "equalAuthnoteAttributeId",
                            "equalAuthnoteAttributeId": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "errorMsg": "attributesInetumError"
                        },
                        {
                            "type": "equalValue",
                            "equalValue": "Maria",
                            "inetumAttributePath": "/response/mrz/given_names",
                            "errorMsg": "attributesInetumError"
                        },
                        {
                            "type": "intMinValue",
                            "intMinValue": "50",
                            "inetumAttributePath": "/response/resultData/scoreDocumental",
                            "errorMsg": "scoringInetumError"
                        },
                        {
                            "type": "equalDateAuthnoteAttributeId",
                            "equalDateAuthnoteAttributeId": "dateOfBirth",
                            "valueDateFormat": "yyyy-MM-dd",
                            "inetumAttributePath": "/response/mrz/date_of_birth",
                            "inetumDateFormat": "dd/MM/yyyy",
                            "errorMsg": "attributesInetumError"
                        },
                        {
                            "type": "isBeforeDateValue",
                            "isBeforeDateValue": "now",
                            "valueDateFormat": "yyyy-MM-dd",
                            "inetumAttributePath": "/response/mrz/date_of_expiry",
                            "inetumDateFormat": "dd/MM/yyyy",
                            "errorMsg": "attributesInetumError"
                        }
                    ],
                    "Seaman's Book": [
                        {
                            "type": "equalAuthnoteAttributeId",
                            "equalAuthnoteAttributeId": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "errorMsg": "attributesInetumError"
                        }
                    ],
                    "Philippine Passport": [
                        {
                            "type": "equalAuthnoteAttributeId",
                            "equalAuthnoteAttributeId": "sequent.read-only.id-card-number",
                            "inetumAttributePath": "/response/mrz/personal_number",
                            "errorMsg": "attributesInetumError"

                        }
                    ]
                }
                    """),
        new ProviderConfigProperty(
            Utils.SDK_ATTRIBUTE,
            "Configuration for the SDK",
            "-",
            ProviderConfigProperty.TEXT_TYPE,
            "{}"),
        new ProviderConfigProperty(
            Utils.MAX_RETRIES,
            "Maximum number of retries for inetum requests. Will use exponential backoff, starting with 1 second.",
            "-",
            ProviderConfigProperty.STRING_TYPE,
            String.valueOf(Utils.DEFAULT_MAX_RETRIES)),
        new ProviderConfigProperty(
            Utils.SDK_VERSION,
            "The version of Inetum SDK to use",
            "Possible options: 4.0.2 or 4.0.3 ",
            ProviderConfigProperty.STRING_TYPE,
            String.valueOf(Utils.DEFAULT_SDK_VERSION)),
        new ProviderConfigProperty(
            Utils.ENV_CONFIG_ATTRIBUTE,
            "Configuration for the env_config",
            "Uses FreeMarker template, see example",
            ProviderConfigProperty.TEXT_TYPE,
            """
                {
                	environment: 0,
                	customTextsConfig: myStrings,
                	baseAssetsUrl: "../../../",
                	uploadAndCheckIdentifiers: ["ESP"],
                	showLogs: false,
                	logTypes: ['ERROR', 'INFO'],
                	design: design,
                	bamEnabled: true,
                	ocrCountdown: false,
                	videoSelfieShowDNI: true,
                	cancelProcessButton: true,
                	showPermissionsHelp: true,
                	qrEnabled: false,
                	voiceEnabled: true,
                	voiceLanguage: VoiceLanguage.spanishSpain,
                	customIOSBrowsersConfig: [IOSBrowser.safari],
                	otpEmailAddress: 'xxxxxxx@inetum.com',
                	otpPhoneNumber: 'xxxxxxxx',
                	countryCode: CountryCode.españa,
                	applicationId: window.DOB_APP_ID,
                	broadcast: new LocalBroadcastManager(),
                  customProtocol: 'https',
                  customHost: 'des.digitalonboarding.es',
                  customPort: '443',
                  sequent: {
                    disableStreaming: true
                  }
                }
                				"""),
        new ProviderConfigProperty(
            Utils.BASE_URL_ATTRIBUTE,
            "Base URL for Inetum API",
            "-",
            ProviderConfigProperty.STRING_TYPE,
            "https://des.digitalonboarding.es/dob-api/2.0.0"),
        new ProviderConfigProperty(
            Utils.TRANSACTION_NEW_ATTRIBUTE,
            "transaction/new template",
            "Uses FreeMarker template, see example",
            ProviderConfigProperty.TEXT_TYPE,
            """
                {
                	"wFtype_Facial": true,
                	"wFtype_OCR": true,
                	"wFtype_Video": false,
                	"wFtype_Anti_Spoofing": false,
                	"wFtype_Sign": false,
                	"wFtype_VerifAvan": false,
                	"wFtype_UECertificate": false,
                	"docID": "${doc_id}",
                	"name": "",
                	"lastname1": "",
                	"lastname2": "",
                	"country": "",
                	"mobilePhone": "",
                	"eMail": "",
                	"priority": 3,
                	"maxRetries": 3,
                	"maxProcessTime": 30,
                	"application": "sequent-keycloak",
                	"clienteID": "${client_id}"
                }
                				"""),
        new ProviderConfigProperty(
            Utils.TEST_MODE_ATTRIBUTE,
            "Test Mode",
            "If true, the authenticator will skip the real OCR flow and mock the data.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            "false"),
        new ProviderConfigProperty(
            Utils.TEST_MODE_SERVER_URL,
            "Test Mode Server Url",
            "If in test mode this will be the case url of the mock server.",
            ProviderConfigProperty.TEXT_TYPE,
            ""));
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}
}
