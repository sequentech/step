// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.text.Collator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

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
  private static final String AUTH_NOTE_ATTRIBUTE_ID = "authnoteAttributeId";
  private static final String INETUM_ATTRIBUTE_PATH = "inetumAttributePath";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    // Authentication is successful if the user already has the user's
    // validation status attribute set to true, otherwise initiate a new
    // flow and show form
    log.info("authenticate()");

    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    UserModel user = context.getUser();

    if (user != null) {
      String statusAttributeName = configMap.get(Utils.USER_STATUS_ATTRIBUTE);
      String statusAttributeValue = user.getFirstAttribute(statusAttributeName);
      log.info("checking statusAttributeValue=" + statusAttributeValue);
      boolean validated = (statusAttributeValue != null && statusAttributeValue.equals("TRUE"));

      log.info("validated=" + validated);
      if (validated) {
        log.info("validated IS TRUE, pass");
        context.success();
        return;
      }
    }

    log.info("validated is NOT TRUE, rendering the form");
    try {
      Map<String, String> transactionData = newTransaction(configMap, context);

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
    } catch (IOException error) {
      context.failure(AuthenticationFlowError.INTERNAL_ERROR);
      context.attempted();
      Response challenge =
          getBaseForm(context)
              .setAttribute(Utils.FTL_ERROR, Utils.FTL_ERROR_INTERNAL)
              .createForm(Utils.INETUM_ERROR);
      context.challenge(challenge);
    }
  }

  /** Send a POST to Inetum API */
  protected SimpleHttp.Response doPost(
      Map<String, String> configMap,
      AuthenticationFlowContext context,
      Object payload,
      String uriPath)
      throws IOException {
    String url = configMap.get(Utils.BASE_URL_ATTRIBUTE) + uriPath;
    String authorization = "Bearer " + configMap.get(Utils.API_KEY_ATTRIBUTE);
    log.info("doPost: url=" + url + ", payload =" + payload.toString());

    SimpleHttp.Response response =
        SimpleHttp.doPost(url, context.getSession())
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .json(payload)
            .asResponse();
    return response;
  }

  /** Send a GET to Inetum API */
  protected SimpleHttp.Response doGet(
      Map<String, String> configMap, AuthenticationFlowContext context, String uriPath)
      throws IOException {
    String url = configMap.get(Utils.BASE_URL_ATTRIBUTE) + uriPath;
    String authorization = "Bearer " + configMap.get(Utils.API_KEY_ATTRIBUTE);
    log.info("doGet: url=" + url);

    SimpleHttp.Response response =
        SimpleHttp.doGet(url, context.getSession())
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .asResponse();
    return response;
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
      throw new IOException(error);
    }

    try {
      SimpleHttp.Response response =
          doPost(configMap, context, jsonPayload, Utils.API_TRANSACTION_NEW);

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

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action()");
    boolean validated = verifyResults(context);
    if (!validated) {
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
                .createForm(Utils.INETUM_ERROR);
        context.challenge(challenge);
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
    } else {
      // valid
      context.success();
    }
  }

  /*
   * Calls Inetum API results/get and verify results
   */
  protected boolean verifyResults(AuthenticationFlowContext context) {
    log.info("verifyResults: start");

    // Get the transaction data from the auth session
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();
    String tokenDob = sessionModel.getAuthNote(Utils.FTL_TOKEN_DOB);
    String userId = sessionModel.getAuthNote(Utils.FTL_USER_ID);
    if (tokenDob == null || userId == null) {
      log.info("verifyResults: TRUE; tokenDob == null || userId == null");
      return false;
    }
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    try {
      String uriPath = "/transaction/" + userId + "/status?t=" + tokenDob;
      SimpleHttp.Response response = doGet(configMap, context, uriPath);

      if (response.getStatus() != 200) {
        log.error(
            "verifyResults: Error calling transaction/status, status = " + response.getStatus());
        log.error(
            "verifyResults: Error calling transaction/status, response.asString() = "
                + response.asString());
        return false;
      }

      int code = response.asJson().get("code").asInt();
      if (code != 0) {
        log.error("verifyResults: Error calling transaction/status, code = " + code);
        return false;
      }
      String idStatus = response.asJson().get("response").get("idStatus").asText();
      log.info("verifyResults: transaction/status, idStatus = " + idStatus);
      // TODO: I don't know why I'm getting "processing" instead of
      // "verificationOk"
      // if (!idStatus.equals("verificationOk") && !idStatus.equals("processing")) {
      // log.error("verifyResults: Error calling transaction/status, idStatus = " +
      // idStatus);
      // return false;
      // }

      // The status is verification OK. Now we need to retrieve the
      // information
      uriPath = "/transaction/" + userId + "/results";
      response = doGet(configMap, context, uriPath);

      if (response.getStatus() != 200) {
        log.error(
            "verifyResults: Error calling transaction/results, status = " + response.getStatus());
        log.error(
            "verifyResults: Error calling transaction/results, response.asString() = "
                + response.asString());
        return false;
      }

      code = response.asJson().get("code").asInt();
      if (code != 0) {
        log.error("verifyResults: Error calling transaction/results, code = " + code);
        return false;
      }
      String responseStr = response.asString();
      log.info("verifyResults: response Str = " + responseStr);

      String docIdTypeAttributeName = configMap.get(Utils.DOC_ID_TYPE_ATTRIBUTE);
      String docIdType = sessionModel.getAuthNote(docIdTypeAttributeName);

      String attributesToValidate = configMap.get(Utils.ATTRIBUTES_TO_VALIDATE);
      log.infov("verifyResults: attributes to validate configuration: {0}", attributesToValidate);
      JsonNode attributesToCheck = null;

      if (attributesToValidate != null) {
        log.infov("verifyResults: attributesToValidate {0}", attributesToValidate);
        log.infov("verifyResults: docIdType {0}", docIdType);

        // Read the attributes to check from the configuration depending on the ID Type
        attributesToCheck = new ObjectMapper().readTree(attributesToValidate).get(docIdType);

        if (attributesToCheck != null) {
          for (JsonNode attributeToCheck : attributesToCheck) {
            String attribute = attributeToCheck.get(AUTH_NOTE_ATTRIBUTE_ID).asText();
            log.infov("verifyResults: attribute {0}", attribute);
            String inetumField = attributeToCheck.get(INETUM_ATTRIBUTE_PATH).asText();
            log.infov("verifyResults: inetumField {0}", inetumField);

            // Get attribute from authentication notes
            String attributeValue = context.getAuthenticationSession().getAuthNote(attribute);
            log.infov("verifyResults: attributeValue {0}", attributeValue);

            if (attributeValue == null) {
              log.errorv("verifyResults: could not find value in auth notes {0}", attribute);
              return false;
            }

            // Get inetum value from response
            String inetumValue = getValueFromInetumResponse(response, inetumField);

            if (inetumValue == null) {
              log.errorv("verifyResults: could not find value in inetum response {0}", inetumField);
              return false;
            }

            // Compare and return false if different
            Collator collator = Collator.getInstance();
            collator.setDecomposition(2);
            collator.setStrength(0);

            if (collator.compare(attributeValue.trim(), inetumValue.trim()) != 0) {
              log.errorv(
                  "verifyResults: FALSE; attribute: {0}, inetumField: {1}, attributeValue: {2}, inetumValue: {3}",
                  attribute, inetumField, attributeValue, inetumValue);
              return false;
            }
          }
        } else {
          log.info("verifyResults: Empty configuration provided. No attributes checked.");
        }
      }

      String configScore = configMap.get(Utils.SCORING_THRESHOLD);
      int minimumScore = Integer.parseInt(Optional.<String>ofNullable(configScore).orElse("50"));
      log.infov("verifyResults: minimumScore {0}", minimumScore);

      boolean scoreOk = validateInetumScore(minimumScore, response);

      if(!scoreOk) {
        log.error("Found a score that is less than minimum allowed.");
        return false;
      }

      log.info("verifyResults: TRUE");

      sessionModel.setAuthNote(
          configMap.get(Utils.USER_STATUS_ATTRIBUTE), Utils.USER_STATUS_VERIFIED);

      return true;
    } catch (IOException error) {
      log.error("verifyResults(): FALSE; Exception: " + error.toString());
      return false;
    }
  }

  private boolean validateInetumScore(int minimumScore, SimpleHttp.Response response) {
    try {
      JsonNode scores = response.asJson().at("/response/resultData");

      if(scores != null) {
        var iter = scores.fields();

        while (iter.hasNext()) {
          var field = iter.next();

          int score = Integer.parseInt(field.getValue().asText());
          log.infov("{0} : {1}", field.getKey(), score);

          // We ignore the scores from the validations that we did not run.
          if (score != -1 && score < minimumScore) {
            return false;
          }
        }
      }
    } catch (IOException e) {
      e.printStackTrace();
      return false;
    }

    return true;
  }

  private String getValueFromInetumResponse(SimpleHttp.Response response, String inetumField) {
    String inetumValue = null;
    try {
      inetumValue = response.asJson().at(inetumField).asText();
    } catch (Exception error) {
      log.errorv("Could not get value: {0}", error.getMessage());
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
            Utils.SCORING_THRESHOLD,
            "Minimum validation score threshold",
            "A number representing the minim value for inetum score between 0 and 100",
            ProviderConfigProperty.STRING_TYPE,
            "50"),
        new ProviderConfigProperty(
            Utils.ATTRIBUTES_TO_VALIDATE,
            "Attributes to validate using inetum data",
            "A Json where it's fields represent every id available. For each id a list of attributes to check need to be provided. With authnoteAttributeId to indicate the user profile attribute and inetumAttributePath to indicate the path to get the attribute from the inetum response.",
            ProviderConfigProperty.TEXT_TYPE,
            """
            {
                "PhilSys ID": [
                    {
                        "authnoteAttributeId": "sequent.read-only.id-card-number",
                        "inetumAttributePath": "/response/mrz/personal_number"
                    },
                    {
                        "authnoteAttributeId": "firstName",
                        "inetumAttributePath": "/response/mrz/given_names"
                    }
                ],
                "Philippine Passport": [
                    {
                        "authnoteAttributeId": "sequent.read-only.id-card-number",
                        "inetumAttributePath": "/response/mrz/personal_number"
                    }
                ],
                "Philippine Passport": [
                    {
                        "authnoteAttributeId": "sequent.read-only.id-card-number",
                        "inetumAttributePath": "/response/mrz/personal_number"
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
                	broadcast: new LocalBroadcastManager()
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
                				"""));
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
