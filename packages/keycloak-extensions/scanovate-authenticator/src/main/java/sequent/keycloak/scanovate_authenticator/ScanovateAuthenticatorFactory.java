// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.google.auto.service.AutoService;
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

  public static final String OCR_URL = "ocr-url";
  public static final String OCR_TYPES = "ocr-types";
  public static final String DOC_ID_TYPE = "doc-id-type";
  public static final String USER_STATUS = "user-status";
  public static final String ATTRIBUTES_TO_VALIDATE = "attributes-to-validate";
  public static final String ATTRIBUTES_TO_STORE = "attributes-to-store";
  public static final String MAX_ATTEMPTS = "max-attempts";
  public static final String CAPTURE_SIDES = "capture-sides";
  public static final String VIDEO_SECONDS = "video-seconds";
  public static final String MAX_IMAGE_BYTES = "max-image-bytes";
  public static final String LIVENESS_URL = "liveness-url";
  public static final String LIVENESS_SECRET = "liveness-secret";
  public static final String LIVENESS_RESULT_WAIT_SECONDS = "liveness-result-wait-seconds";
  public static final String FACE_MATCH_URL = "face-match-url";
  public static final String FACE_MATCH_MIN_SIMILARITY = "face-match-min-similarity";

  public static final String DEFAULT_DOC_ID_TYPE = "sequent.read-only.id-card-type";
  public static final String DEFAULT_USER_STATUS = "sequent.read-only.id-card-number-validated";
  public static final int DEFAULT_MAX_RETRIES = 3;
  public static final int DEFAULT_MAX_ATTEMPTS = 3;
  public static final int DEFAULT_VIDEO_SECONDS = 5;
  public static final int DEFAULT_MAX_IMAGE_BYTES = 2 * 1024 * 1024;
  public static final int DEFAULT_LIVENESS_RESULT_WAIT_SECONDS = 15;

  static final String DEFAULT_OCR_TYPES = "{\"default\": \"passport\"}";
  static final String DEFAULT_CAPTURE_SIDES = "{\"default\": [\"front\", \"back\"]}";
  static final String DEFAULT_FACE_MATCH_MIN_SIMILARITY = "{\"default\": 0.67}";

  static final String DEFAULT_ATTRIBUTES_TO_VALIDATE =
      """
      {
        "default": [
          {
            "type": "isBeforeDateValue",
            "isBeforeDateValue": "now",
            "process": "ocr",
            "attributePath": "/date_of_expiry",
            "sourceDateFormat": "yyyy-MM-dd",
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
            "attributePath": "/first_name_english",
            "type": "text"
          },
          {
            "UserAttribute": "lastName",
            "process": "ocr",
            "attributePath": "/last_name_english",
            "type": "text"
          },
          {
            "UserAttribute": "dateOfBirth",
            "process": "ocr",
            "attributePath": "/date_of_birth",
            "type": "date",
            "sourceDateFormat": "yyyy-MM-dd",
            "storeDateFormat": "yyyy-MM-dd"
          }
        ]
      }
      """;

  private static final String RULES_HELP =
      " JSON object keyed by document type (the value of the document type auth note), or"
          + " \"default\" for any other document type. Each rule reads a value from the results"
          + " of the OCR service with a JSON pointer (attributePath), relative to the fields read"
          + " from the document (process ocr) or to the checks of the service (process"
          + " authentications), or to the whole results if no process is given. Dates of the"
          + " machine readable zone are given as yyyy-MM-dd.";

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
    return "Scanovate Identity Verification";
  }

  @Override
  public String getHelpText() {
    return "Verifies the voter's identity document, liveness and face with the Scanovate"
        + " services hosted on premise.";
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
        .name(OCR_URL)
        .label("OCR URL")
        .helpText(
            "Internal base URL of the on-premise Scanovate OCR service, e.g."
                + " http://scanovate-ocr:5040. Only Keycloak calls it.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(OCR_TYPES)
        .label("OCR types")
        .helpText(
            "JSON object keyed by document type (the value of the document type auth note), with"
                + " a \"default\" key for any other document type, giving how the OCR service reads"
                + " it, e.g. {\"philippinePassport\": \"passport\"}. Document types without one are"
                + " rejected.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_OCR_TYPES)
        .add()
        .property()
        .name(LIVENESS_URL)
        .label("Liveness Plus URL")
        .helpText(
            "Base URL of the on-premise Scanovate Liveness Plus service as the voter's browser"
                + " reaches it, https://<keycloak host>/biometric in our deployments. The capture"
                + " page calls its API under /liveness.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(LIVENESS_SECRET)
        .label("Liveness Plus callback secret")
        .helpText(
            "Secret that Liveness Plus sends in the secret query parameter of the"
                + " onprem.token_verification_url and onprem.callback_url of its"
                + " service_config.json. The voter's browser never sees it.")
        .type(ProviderConfigProperty.PASSWORD)
        .secret(true)
        .add()
        .property()
        .name(LIVENESS_RESULT_WAIT_SECONDS)
        .label("Liveness result wait")
        .helpText(
            "Seconds to wait for the result callback of Liveness Plus once the voter is done.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_LIVENESS_RESULT_WAIT_SECONDS))
        .add()
        .property()
        .name(FACE_MATCH_URL)
        .label("Face Match URL")
        .helpText(
            "Internal base URL of the on-premise Scanovate Face Match service, e.g."
                + " http://scanovate-face-match:3000. Only Keycloak calls it.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .add()
        .property()
        .name(FACE_MATCH_MIN_SIMILARITY)
        .label("Face Match minimum similarity")
        .helpText(
            "JSON object keyed by document type (the value of the document type auth note), with"
                + " a \"default\" key for any other document type, giving the minimum similarity,"
                + " above 0 and up to 1, for the voter's face to match the ID and the photo holding"
                + " it. The Face Match threshold applies if it is stricter.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_FACE_MATCH_MIN_SIMILARITY)
        .add()
        .property()
        .name(DOC_ID_TYPE)
        .label("Document type auth note")
        .helpText("Auth note holding the document type, used to pick the settings to apply.")
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
            "JSON object keyed by document type (the value of the document type auth note), or"
                + " \"default\" for any other document type, listing the sides to capture:"
                + " [\"front\"] or [\"front\", \"back\"]. Document types without an entry capture"
                + " both sides.")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .defaultValue(DEFAULT_CAPTURE_SIDES)
        .add()
        .property()
        .name(VIDEO_SECONDS)
        .label("Holding time")
        .helpText("Seconds the voter holds the ID next to their face before its photo is taken.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_VIDEO_SECONDS))
        .add()
        .property()
        .name(MAX_IMAGE_BYTES)
        .label("Maximum image size")
        .helpText(
            "Maximum size in bytes of each captured photo (2 MiB by default, at most 8 MiB). The"
                + " capture page uploads each photo in its own request, which must fit in the body"
                + " limit of any reverse proxy in front of Keycloak.")
        .type(ProviderConfigProperty.STRING_TYPE)
        .defaultValue(String.valueOf(DEFAULT_MAX_IMAGE_BYTES))
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
