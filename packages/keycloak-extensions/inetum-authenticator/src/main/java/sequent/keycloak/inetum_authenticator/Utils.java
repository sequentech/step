// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2021,2022 Inventage AG
//
// SPDX-License-Identifier: AGPL-3.0-only

// Partially based in:
// https://github.dev/inventage/keycloak-custom/tree/tutorial-passkey/extensions/extension-passkey/src/main/java/com/inventage/keycloak/registration/Utils.java

package sequent.keycloak.inetum_authenticator;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import freemarker.template.Template;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.io.StringWriter;
import java.io.Writer;
import java.util.Collection;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.credential.hash.Pbkdf2PasswordHashProvider;
import org.keycloak.events.Details;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

@UtilityClass
@JBossLog
public class Utils {
  public final String DOC_ID_ATTRIBUTE = "doc-id";
  public final String DOC_ID_TYPE_ATTRIBUTE = "doc-id-type";
  public final String USER_STATUS_ATTRIBUTE = "user-status";
  public final String USER_STATUS_VERIFIED = "VERIFIED";
  public final String USER_STATUS_NOT_VERIFIED = "NOT-VERIFIED";
  public final String SDK_ATTRIBUTE = "sdk";
  public final String API_KEY_ATTRIBUTE = "api-key";
  public final String APP_ID_ATTRIBUTE = "app-id";
  public final String CLIENT_ID_ATTRIBUTE = "client-id";
  public final String ENV_CONFIG_ATTRIBUTE = "env-config";
  public final String BASE_URL_ATTRIBUTE = "base-url";
  public final String TRANSACTION_NEW_ATTRIBUTE = "transaction-new";
  public final String INETUM_FORM = "inetum-authenticator.ftl";
  public final String INETUM_ERROR = "inetum-error.ftl";
  public final String INETUM_CONFIRM = "inetum-confirmation.ftl";
  public final String ATTRIBUTES_TO_VALIDATE = "attributes-to-validate";
  public final String ATTRIBUTES_TO_STORE = "attributes-to-store";
  public static final String TEST_MODE_ATTRIBUTE = "testMode";
  public static final String TEST_MODE_SERVER_URL = "testModeServerUrl";
  public final String API_TRANSACTION_NEW = "/transaction/new";

  public final String CODE_ID = "code_id";
  public final String FTL_ERROR = "error";
  public final String FTL_REALM = "realm";
  public final String FTL_USER_ID = "user_id";
  public final String FTL_TOKEN_DOB = "token_dob";
  public final String FTL_API_KEY = "api_key";
  public final String FTL_APP_ID = "app_id";
  public final String FTL_CLIENT_ID = "client_id";
  public final String FTL_BASE_URL = "base_url";
  public final String FTL_ENV_CONFIG = "env_config";
  public final String FTL_DOC_ID = "doc_id";
  public final String FTL_DOC_ID_TYPE = "doc_id_type";
  public final String FTL_ERROR_INTERNAL = "internalInetumError";
  public final String FTL_ERROR_AUTH_INVALID = "internalInetumError";
  public final String FTL_ERROR_INVALID_SCORE = "scoringInetumError";
  public final String FTL_ERROR_INVALID_ATTRIBUTES = "attributesInetumError";
  public final String FTL_ERROR_MAX_RETRIES = "maxRetriesError";

  private static final String KEYS_USERDATA = "keyUserdata";
  private static final String KEYS_USERDATA_SEPARATOR = ";";
  private static final List<String> DEFAULT_KEYS_USERDATA =
      List.of(UserModel.FIRST_NAME, UserModel.LAST_NAME, UserModel.EMAIL, UserModel.USERNAME);
  private static final String USER_ID = "userId";
  public static final String MULTIVALUE_SEPARATOR = "##";
  public static final String ATTRIBUTE_TO_VALIDATE_SEPARATOR = ":";
  public static final String ERROR_MESSAGE_NOT_SENT = "messageNotSent";
  public static final String ERROR_USER_NOT_FOUND = "userNotFound";
  public static final String ERROR_MESSAGE_USER_NOT_FOUND = "User not found";
  public static final String ERROR_USER_HAS_CREDENTIALS = "User already has credentials";
  public static final String ERROR_USER_HAS_CREDENTIALS_ERROR = "userAlreadyHasCredentials";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNSET = "User Attributes Not Unset";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNSET_ERROR =
      "userShouldHaveUnsetAttributes";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNIQUE = "User Attributes Not Unique";
  public static final String UPLOAD_AND_CHECK_EXCEPTION = "Exception during Upload and Check";
  public static final String PHONE_NUMBER = "phone_number";
  public static final String PHONE_NUMBER_ATTRIBUTE = "sequent.read-only.mobile-number";
  public static final String ID_NUMBER_ATTRIBUTE = "sequent.read-only.id-card-number";
  public static final String ID_NUMBER = "ID_number";
  public static final String USER_PROFILE_ATTRIBUTES = "user_profile_attributes";
  public static final String AUTHENTICATOR_CLASS_NAME = "authenticator_class_name";
  public static final String SESSION_ID = "session_id";
  public static final String MAX_RETRIES = "max-retries";
  public static final String EVENT_TYPE_COMMUNICATIONS = "communications";
  public static final int DEFAULT_MAX_RETRIES = 3;
  public static final int BASE_RETRY_DELAY = 1_000;
  public static final String ERROR_GENERATING_APPROVAL = "approvalGenerationError";
  public static final String SDK_VERSION = "sdk-version";
  public static final String FTL_SDK_VERSION = "sdk_version";
  public static final String DEFAULT_SDK_VERSION = "4.0.3";

  String escapeJson(String value) {
    return value != null
        ? value.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
        : null;
  }

  public static Locale getLocale(AuthenticationFlowContext context) {
    RealmModel realm = context.getRealm();
    KeycloakSession session = context.getSession();
    UserModel user = context.getUser();

    Locale locale;
    if (user != null) {
      locale = session.getContext().resolveLocale(user);
    } else {
      locale = session.getContext().resolveLocale(null);
      if (locale == null) {
        String defaultLocale = realm.getDefaultLocale();
        if (defaultLocale != null) {
          locale = Locale.forLanguageTag(defaultLocale);
        } else {
          locale = Locale.getDefault();
        }
      }
    }
    return locale;
  }

  /**
   * We store the user data entered in the registration form in the session notes. This information
   * will later be retrieved to create a user account.
   */
  static void storeUserDataInAuthSessionNotes(
      FormContext context, List<String> searchAttributesList) {
    log.info("storeUserDataInAuthSessionNotes: start");
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();

    // Lookup user by attributes using form data
    UserModel user = Utils.lookupUserByFormData(context, searchAttributesList, formData);

    // We store each key
    String keys = Utils.serializeUserdataKeys(formData.keySet());

    log.debug(
        "storeUserDataInAuthSessionNotes: setAuthNote(" + Utils.KEYS_USERDATA + ", " + keys + ")");
    sessionModel.setAuthNote(Utils.KEYS_USERDATA, keys);

    formData.forEach(
        (key, value) -> {
          String values = Utils.serializeUserdataKeys(formData.get(key));
          log.debug("storeUserDataInAuthSessionNotes: setAuthNote(" + key + ", " + values + ")");
          sessionModel.setAuthNote(key, values);
        });

    sessionModel.setAuthNote(USER_ID, user.getId());
  }

  /** We retrieve the user data stored in the session notes and create a new user in this realm. */
  static void createUserFromAuthSessionNotes(AuthenticationFlowContext context) {
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    MultivaluedMap<String, String> userAttributes = new MultivaluedHashMap<>();

    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
    List<String> keysUserdata =
        Utils.deserializeUserdataKeys(authenticationSession.getAuthNote(Utils.KEYS_USERDATA));

    // keys userdata is transmitted from the UserCreationPasskeyAction class.
    if (keysUserdata != null) {
      for (String key : keysUserdata) {
        String value = authenticationSession.getAuthNote(key);
        if (value != null) {
          userAttributes.add(key, value);
        }
      }
    } // In case that another custom FormAction than UserCreationPasskey is used.
    else {
      for (String key : DEFAULT_KEYS_USERDATA) {
        String value = authenticationSession.getAuthNote(key);
        if (value != null) {
          userAttributes.add(key, value);
        }
      }
    }

    String email = formData.getFirst(UserModel.EMAIL);
    String username = formData.getFirst(UserModel.USERNAME);

    if (context.getRealm().isRegistrationEmailAsUsername()) {
      username = email;
    }

    context
        .getEvent()
        .detail(Details.USERNAME, username)
        .detail(Details.REGISTER_METHOD, "form")
        .detail(Details.EMAIL, email);

    KeycloakSession session = context.getSession();

    UserProfileProvider profileProvider = session.getProvider(UserProfileProvider.class);
    UserProfile profile = profileProvider.create(UserProfileContext.REGISTRATION, userAttributes);
    UserModel user = profile.create();

    user.setEnabled(true);
    context.setUser(user);

    context.getAuthenticationSession().setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, username);

    context.getEvent().user(user);
    context.getEvent().success();
    context.newEvent().event(EventType.LOGIN);
    context
        .getEvent()
        .client(context.getAuthenticationSession().getClient().getClientId())
        .detail(Details.REDIRECT_URI, context.getAuthenticationSession().getRedirectUri())
        .detail(Details.AUTH_METHOD, context.getAuthenticationSession().getProtocol());
    String authType = context.getAuthenticationSession().getAuthNote(Details.AUTH_TYPE);
    if (authType != null) {
      context.getEvent().detail(Details.AUTH_TYPE, authType);
    }
  }

  static UserModel lookupUserByFormData(
      FormContext context, List<String> attributes, MultivaluedMap<String, String> formData) {
    log.info("lookupUserByFormData(): start");
    KeycloakSession session = context.getSession();
    RealmModel realm = context.getRealm();

    Map<String, String> firstValueFormData =
        formData.entrySet().stream()
            .filter(e -> attributes.contains(e.getKey()))
            .collect(Collectors.toMap(Map.Entry::getKey, e -> e.getValue().get(0).trim()));

    Stream<UserModel> userStream = session.users().searchForUserStream(realm, firstValueFormData);

    // Return the first user that matches all attributes, if any
    Optional<UserModel> userOptional = userStream.findFirst();
    return userOptional.orElse(null);
  }

  /**
   * Processes a string template with FreeMarker library.
   *
   * <p>Doesn't support importing other templates from within the given template.
   *
   * @param data Map with the data to be used in the template
   * @param sourceCode String with the template
   * @return
   * @throws Exception
   */
  static String processStringTemplate(Object data, String sourceCode) throws Exception {
    Template template = new Template("string-template", sourceCode, null);
    Writer out = new StringWriter();
    template.process(data, out);
    return out.toString();
  }

  private static String serializeUserdataKeys(Collection<String> keys, String separator) {
    final StringBuilder key = new StringBuilder();
    keys.forEach((s -> key.append(separator).append(s)));
    return key.deleteCharAt(0).toString();
  }

  private static String serializeUserdataKeys(Collection<String> keys) {
    return serializeUserdataKeys(keys, KEYS_USERDATA_SEPARATOR);
  }

  private static List<String> deserializeUserdataKeys(String key, String separator) {
    if (key == null) {
      return Collections.emptyList();
    }
    return List.of(key.split(separator));
  }

  private static List<String> deserializeUserdataKeys(String key) {
    return deserializeUserdataKeys(key, KEYS_USERDATA_SEPARATOR);
  }

  /**
   * Recovers the values of an attribute stored in the auth notes.
   *
   * @param context
   * @param attribute attribute to recover the values from
   * @return a collection of values in string format
   */
  public static List<String> getAttributeValuesFromAuthNote(
      AuthenticationFlowContext context, String attribute) {
    return Utils.deserializeUserdataKeys(context.getAuthenticationSession().getAuthNote(attribute));
  }

  /** Recovers the user stored in the auth notes. */
  public static UserModel lookupUserByAuthNotes(AuthenticationFlowContext context) {
    String userId = context.getAuthenticationSession().getAuthNote(USER_ID);

    return context.getSession().users().getUserById(context.getRealm(), userId);
  }

  public String getUserAttributesString(UserModel user) {
    if (user != null) {
      Map<String, List<String>> attributes = user.getAttributes();
      ObjectMapper mapper = new ObjectMapper();
      ObjectNode attributesJson = mapper.createObjectNode();

      for (String attributeName : attributes.keySet()) {
        String value = attributes.get(attributeName).get(0);
        if (value != null) {
          attributesJson.put(attributeName, value);
        }
      }

      return attributesJson.toString();
    }
    return null;
  }

  public Map<String, String> buildApplicantData(
      KeycloakSession session, AuthenticationSessionModel authSession)
      throws JsonProcessingException {
    List<UPAttribute> realmsAttributes = getRealmUserProfileAttributes(session);
    Map<String, String> applicantData = new HashMap<>();

    for (UPAttribute attribute : realmsAttributes) {
      String authNoteValue = authSession.getAuthNote(attribute.getName());

      if (authNoteValue != null && !authNoteValue.isBlank())
        applicantData.put(attribute.getName(), authNoteValue);
    }

    return applicantData;
  }

  public PasswordCredentialModel buildPassword(KeycloakSession session, String rawPassword) {
    RealmModel realm = session.getContext().getRealm();

    // Use the Pbkdf2PasswordHashProvider
    Pbkdf2PasswordHashProvider hashProvider =
        (Pbkdf2PasswordHashProvider)
            session.getProvider(PasswordHashProvider.class, "pbkdf2-sha256");

    int hashIterations = realm.getPasswordPolicy().getHashIterations();

    // Create a PasswordCredentialModel
    return hashProvider.encodedCredential(rawPassword, hashIterations);
  }

  public void buildEventDetails(
      EventBuilder builder,
      AuthenticationSessionModel authSession,
      UserModel user,
      KeycloakSession session,
      String className) {
    List<UPAttribute> realmsAttributes = getRealmUserProfileAttributes(session);
    for (UPAttribute attribute : realmsAttributes) {
      String authNoteValue = authSession.getAuthNote(attribute.getName());
      builder.detail(attribute.getName(), authNoteValue);
    }
    if (user != null) {
      builder.user(user.getId());
      builder.detail(USER_PROFILE_ATTRIBUTES, getUserAttributesString(user));
    } else {
      String userId = authSession.getAuthNote(USER_ID);
      builder.user(userId);
    }
    builder.detail(AUTHENTICATOR_CLASS_NAME, className);
    builder.detail(SESSION_ID, authSession.getParentSession().getId());
  }

  public List<UPAttribute> getRealmUserProfileAttributes(KeycloakSession session) {
    UserProfileProvider userProfileProvider = session.getProvider(UserProfileProvider.class);
    UPConfig userProfileMetadata = userProfileProvider.getConfiguration();
    return userProfileMetadata.getAttributes();
  }

  public static int parseInt(String s, int defaultValue) {
    if (s == null) return defaultValue;
    try {
      return Integer.parseInt(s);
    } catch (NumberFormatException x) {
      return defaultValue;
    }
  }
}
