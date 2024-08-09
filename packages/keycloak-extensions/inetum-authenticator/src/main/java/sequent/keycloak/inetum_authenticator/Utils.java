// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2021,2022 Inventage AG
//
// SPDX-License-Identifier: AGPL-3.0-only

// Partially based in:
// https://github.dev/inventage/keycloak-custom/tree/tutorial-passkey/extensions/extension-passkey/src/main/java/com/inventage/keycloak/registration/Utils.java

package sequent.keycloak.inetum_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import freemarker.template.Template;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.io.StringWriter;
import java.io.Writer;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.events.Details;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
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
  public final String ATTRIBUTES_TO_VALIDATE = "attributes-to-validate";

  public final String API_TRANSACTION_NEW = "/transaction/new";

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

  private static final String KEYS_USERDATA = "keyUserdata";
  private static final String KEYS_USERDATA_SEPARATOR = ";";
  private static final List<String> DEFAULT_KEYS_USERDATA =
      List.of(UserModel.FIRST_NAME, UserModel.LAST_NAME, UserModel.EMAIL, UserModel.USERNAME);
  private static final String USER_ID = "userId";

  private static final String INETUM_PATH_SEPARATOR = ".";
  private static JsonNode asJson;

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

    log.info(
        "storeUserDataInAuthSessionNotes: setAuthNote(" + Utils.KEYS_USERDATA + ", " + keys + ")");
    sessionModel.setAuthNote(Utils.KEYS_USERDATA, keys);

    formData.forEach(
        (key, value) -> {
          String values = Utils.serializeUserdataKeys(formData.get(key));
          log.info("storeUserDataInAuthSessionNotes: setAuthNote(" + key + ", " + values + ")");
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
}
