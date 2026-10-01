// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.google.auto.service.AutoService;
import java.util.Arrays;
import java.util.List;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.provider.ProviderConfigurationBuilder;

@AutoService(AuthenticatorFactory.class)
public class ScanovateAuthenticatorFactory implements AuthenticatorFactory {
  public static final String PROVIDER_ID = "scanovate-authenticator";

  public static final String BASE_URL = "base-url";
  public static final String CLIENT_ID = "client-id";
  public static final String CLIENT_SECRET = "client-secret";
  public static final String FLOW_ID = "flow-id";
  public static final String EXECUTION_MODE = "execution-mode";
  public static final String SAVE_OPTION = "save-option";
  public static final String LINK_PARAMS = "link-params";
  public static final String DOC_ID = "doc-id";
  public static final String DOC_ID_TYPE = "doc-id-type";
  public static final String USER_STATUS = "user-status";
  public static final String ATTRIBUTES_TO_VALIDATE = "attributes-to-validate";
  public static final String ATTRIBUTES_TO_STORE = "attributes-to-store";
  public static final String MAX_RETRIES = "max-retries";
  public static final String MAX_ATTEMPTS = "max-attempts";
  public static final String CAPTURE_SIDES = "capture-sides";
  public static final String VIDEO_SECONDS = "video-seconds";
  public static final String MAX_IMAGE_BYTES = "max-image-bytes";
  public static final String MAX_VIDEO_BYTES = "max-video-bytes";
  public static final String FACE_CAPTURE = "face-capture";
  public static final String LIVENESS_URL = "liveness-url";
  public static final String LIVENESS_SECRET = "liveness-secret";
  public static final String LIVENESS_RESULT_WAIT_SECONDS = "liveness-result-wait-seconds";
  public static final String FACE_MATCH_URL = "face-match-url";
  public static final String FACE_MATCH_MIN_SIMILARITY = "face-match-min-similarity";

  public static final String DEFAULT_DOC_ID = "sequent.read-only.id-card-number";
  public static final String DEFAULT_DOC_ID_TYPE = "sequent.read-only.id-card-type";
  public static final String DEFAULT_USER_STATUS = "sequent.read-only.id-card-number-validated";
  public static final int DEFAULT_MAX_RETRIES = 3;
  public static final int DEFAULT_MAX_ATTEMPTS = 3;
  public static final int DEFAULT_VIDEO_SECONDS = 5;
  public static final int DEFAULT_MAX_IMAGE_BYTES = 2 * 1024 * 1024;
  public static final int DEFAULT_MAX_VIDEO_BYTES = 3 * 1024 * 1024;
  public static final int DEFAULT_LIVENESS_RESULT_WAIT_SECONDS = 15;

  static final String DEFAULT_CAPTURE_SIDES = "{\"default\": [\"front\", \"back\"]}";
  static final String DEFAULT_FACE_MATCH_MIN_SIMILARITY = "{\"default\": 0.67}";

  static final String DEFAULT_ATTRIBUTES_TO_VALIDATE =
      """
      {
        "default": [
          {
            "type": "equalValue",
            "equalValue": "true",
            "process": "liveness_plus",
            "attributePath": "/livenessCheck",
            "errorMsg": "scanovateVerificationFailedError"
          },
          {
            "type": "minValue",
            "minValue": "0.67",
            "process": "biometric_match",
            "attributePath": "/score",
            "errorMsg": "scanovateScoringError"
          },
          {
            "type": "equalAuthnoteAttributeId",
            "equalAuthnoteAttributeId": "sequent.read-only.id-card-number",
            "process": "ocr",
            "attributePath": "/idNumber",
            "errorMsg": "scanovateAttributesError"
          },
          {
            "type": "isBeforeDateValue",
            "isBeforeDateValue": "now",
            "process": "ocr",
            "attributePath": "/expiryDate",
            "sourceDateFormat": "dd.MM.yyyy",
            "errorMsg": "scanovateAttributesError"
          }
        ]
      }
      """;

  static final String DEFAULT_ATTRIBUTES_TO_STORE =
      """
      {
        "default": [
          {
            "UserAttribute": "firstName",
            "process": "ocr",
            "attributePath": "/firstName",
            "type": "text"
          },
          {
            "UserAttribute": "lastName",
            "process": "ocr",
            "attributePath": "/lastName",
            "type": "text"
          },
          {
            "UserAttribute": "dateOfBirth",
            "process": "ocr",
            "attributePath": "/dob",
            "type": "date",
            "sourceDateFormat": "dd.MM.yyyy",
            "storeDateFormat": "yyyy-MM-dd"
          }
        ]
      }
      """;

  private static final String RULES_HELP =
      " JSON object keyed by document type (the value of the document type auth note), or"
          + " \"default\" for any other document type. Each rule reads a value from the B-Trust"
          + " results with a JSON pointer (attributePath), relative to the last successful"
          + " attempt of the given process (ocr, liveness_plus, biometric_match,"
          + " document_liveness_plus, STT, age_gender_compare or mobileForm), or to the whole"
          + " response if no process is given.";

  private static final AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  private static final ScanovateAuthenticator SINGLETON = new ScanovateAuthenticator();

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Scanovate B-Trust Identity Verification";
  }

  @Override
  public String getHelpText() {
    return "Verifies the voter's identity document and liveness with Scanovate B-Trust.";
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
    return false;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return ProviderConfigurationBuilder.create()
        .property()
        .name(BASE_URL)
        .label("B-Trust API base URL")
        .helpText("Base URL of the B-Trust API, e.g. https://btrust.example.com")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(CLIENT_ID)
        .label("Client ID")
        .helpText("OAuth client id provided by Scanovate.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(CLIENT_SECRET)
        .label("Client secret")
        .helpText("OAuth client secret provided by Scanovate.")
        .type(ProviderConfigProperty.PASSWORD)
        .secret(true)
        .add()
        .property()
        .name(FLOW_ID)
        .label("Flow ID")
        .helpText("Numeric id of the B-Trust flow to launch.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(EXECUTION_MODE)
        .label("Execution mode")
        .helpText(
            "interactive redirects the voter to B-Trust. embedded captures the document, selfie"
                + " and video in Keycloak's own page, guided in the browser, and uploads them to"
                + " B-Trust through an endpoint pending Scanovate's confirmation; it needs the"
                + " sequent-ui login theme. auto-complete fetches the results right away without"
                + " redirecting the voter, and is only meant to be used against a mock server.")
        .type(ProviderConfigProperty.LIST_TYPE)
        .options(Arrays.stream(ExecutionMode.values()).map(ExecutionMode::value).toList())
        .defaultValue(ExecutionMode.INTERACTIVE.value())
        .add()
        .property()
        .name(SAVE_OPTION)
        .label("Save option")
        .helpText(
            "Data persistence requested to B-Trust. Empty uses the account default. With"
                + " do_not_save, B-Trust deletes the session data once the results are fetched.")
        .type(ProviderConfigProperty.LIST_TYPE)
        .options(Arrays.stream(SaveOption.values()).map(SaveOption::value).toList())
        .defaultValue(SaveOption.DEFAULT.value())
        .add()
        .property()
        .name(LINK_PARAMS)
        .label("Flow parameters")
        .helpText(
            "JSON object mapping B-Trust flow parameter names to the auth notes whose value is"
                + " sent, e.g. {\"country\": \"country\"}.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue("{}")
        .add()
        .property()
        .name(DOC_ID)
        .label("Document number auth note")
        .helpText("Auth note holding the document number, sent to B-Trust as id_number.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(DEFAULT_DOC_ID)
        .add()
        .property()
        .name(DOC_ID_TYPE)
        .label("Document type auth note")
        .helpText("Auth note holding the document type, used to pick the rules to apply.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(DEFAULT_DOC_ID_TYPE)
        .add()
        .property()
        .name(USER_STATUS)
        .label("Verification status attribute")
        .helpText(
            "Auth note set to VERIFIED once the verification succeeds. Users whose attribute with"
                + " this name is VERIFIED skip the verification.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(DEFAULT_USER_STATUS)
        .add()
        .property()
        .name(ATTRIBUTES_TO_VALIDATE)
        .label("Attributes to validate")
        .helpText(
            "Rules that must pass for the verification to succeed."
                + RULES_HELP
                + " Types: equalValue, equalAuthnoteAttributeId, minValue,"
                + " equalDateAuthnoteAttributeId and isBeforeDateValue. errorMsg sets the message"
                + " shown when the rule fails. Document types without rules are rejected.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_ATTRIBUTES_TO_VALIDATE)
        .add()
        .property()
        .name(ATTRIBUTES_TO_STORE)
        .label("Attributes to store")
        .helpText(
            "Values stored as auth notes (UserAttribute) and shown to the voter for confirmation."
                + RULES_HELP
                + " Types: text and date (with sourceDateFormat and storeDateFormat).")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_ATTRIBUTES_TO_STORE)
        .add()
        .property()
        .name(MAX_RETRIES)
        .label("Maximum API retries")
        .helpText("Attempts for each B-Trust API call, with exponential backoff from 1 second.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_MAX_RETRIES))
        .add()
        .property()
        .name(MAX_ATTEMPTS)
        .label("Maximum verification attempts")
        .helpText("Failed verifications allowed before the voter is rejected.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_MAX_ATTEMPTS))
        .add()
        .property()
        .name(CAPTURE_SIDES)
        .label("Document sides to capture")
        .helpText(
            "Embedded mode only. JSON object keyed by document type (the value of the document"
                + " type auth note), or \"default\" for any other document type, listing the"
                + " sides to capture: [\"front\"] or [\"front\", \"back\"]. Document types"
                + " without an entry capture both sides.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_CAPTURE_SIDES)
        .add()
        .property()
        .name(VIDEO_SECONDS)
        .label("Video length")
        .helpText("Embedded mode only. Seconds of video recorded while the voter holds the ID.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_VIDEO_SECONDS))
        .add()
        .property()
        .name(MAX_IMAGE_BYTES)
        .label("Maximum image size")
        .helpText(
            "Embedded mode only. Maximum size in bytes of each captured image (2 MiB by"
                + " default, at most 8 MiB). The capture page uploads each file in its own"
                + " request, which must fit in the body limit of any reverse proxy in front of"
                + " Keycloak.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_MAX_IMAGE_BYTES))
        .add()
        .property()
        .name(MAX_VIDEO_BYTES)
        .label("Maximum video size")
        .helpText(
            "Embedded mode only. Maximum size in bytes of the captured video (3 MiB by"
                + " default, at most 8 MiB). The capture page uploads each file in its own"
                + " request, which must fit in the body limit of any reverse proxy in front of"
                + " Keycloak.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_MAX_VIDEO_BYTES))
        .add()
        .property()
        .name(FACE_CAPTURE)
        .label("Face capture")
        .helpText(
            "Embedded mode only. photo: Keycloak's own page takes a photo of the voter and a video"
                + " of them holding the ID, and B-Trust checks them. liveness: Keycloak's own page"
                + " checks the voter's liveness with the on-premise Liveness Plus API, Keycloak"
                + " compares the voter's face with the ID and with a photo of them holding it using"
                + " the on-premise Face Match service, and only the ID photos are sent to"
                + " B-Trust.")
        .type(ProviderConfigProperty.LIST_TYPE)
        .options(Arrays.stream(FaceCapture.values()).map(FaceCapture::value).toList())
        .defaultValue(FaceCapture.PHOTO.value())
        .add()
        .property()
        .name(LIVENESS_URL)
        .label("Liveness Plus URL")
        .helpText(
            "Liveness face capture only. Base URL of the Liveness Plus service as the voter's"
                + " browser reaches it, https://<keycloak host>/biometric in our deployments. The"
                + " capture page calls its API under /liveness.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(LIVENESS_SECRET)
        .label("Liveness Plus callback secret")
        .helpText(
            "Liveness face capture only. Secret that Liveness Plus sends in the secret query"
                + " parameter of the onprem.token_verification_url and onprem.callback_url of its"
                + " service_config.json. The voter's browser never sees it.")
        .type(ProviderConfigProperty.PASSWORD)
        .secret(true)
        .add()
        .property()
        .name(LIVENESS_RESULT_WAIT_SECONDS)
        .label("Liveness result wait")
        .helpText(
            "Liveness face capture only. Seconds to wait for the result callback of Liveness"
                + " Plus once the voter is done.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_LIVENESS_RESULT_WAIT_SECONDS))
        .add()
        .property()
        .name(FACE_MATCH_URL)
        .label("Face Match URL")
        .helpText(
            "Liveness face capture only. Internal base URL of the on-premise Scanovate Face Match"
                + " service, e.g. http://scanovate-face-match:3000. Only Keycloak calls it.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(FACE_MATCH_MIN_SIMILARITY)
        .label("Face Match minimum similarity")
        .helpText(
            "Liveness face capture only. JSON object keyed by document type (the value of the"
                + " document type auth note), with a \"default\" key for any other document type,"
                + " giving the minimum similarity, above 0 and up to 1, for the voter's face to"
                + " match the ID and the photo holding it. The Face Match threshold applies if it"
                + " is stricter.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_FACE_MATCH_MIN_SIMILARITY)
        .add()
        .build();
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
