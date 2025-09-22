// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;
import jakarta.ws.rs.core.MultivaluedMap;
import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.StringJoiner;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.apache.http.HttpResponse;
import org.apache.http.NameValuePair;
import org.apache.http.client.HttpClient;
import org.apache.http.client.entity.UrlEncodedFormEntity;
import org.apache.http.client.methods.HttpPost;
import org.apache.http.message.BasicNameValuePair;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.connections.httpclient.HttpClientProvider;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.util.JsonSerialization;

@UtilityClass
@JBossLog
public class Utils {
  public static final String USERNAME_ATTRIBUTES = "usernameAttributes";
  public static final List<String> USERNAME_ATTRIBUTES_DEFAULT =
      Collections.unmodifiableList(Arrays.asList("username"));
  public final String ATTEMPTED_EMAIL = "ATTEMPTED_EMAIL";
  public final String DISABLE_PASSWORD_ATTRIBUTE = "disablePassword";
  public final String HIDE_USER_NOT_FOUND = "hideUserNotFound";
  public final String PASSWORD_CHARS = "passwordChars";
  public final String PASSWORD_CHARS_DEFAULT =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.-!¡?¿*:;&()=@#$%";
  public final String PASSWORD_LENGTH = "passwordLength";
  public final String PASSWORD_LENGTH_DEFAULT = "12";
  public final String PASSWORD_EXPIRATION_SECONDS = "passwordExpiration";
  public final String PASSWORD_EXPIRATION_SECONDS_DEFAULT = "7200";
  public final String PASSWORD_EXPIRATION_USER_ATTRIBUTE = "passwordExpirationUserAttribute";
  public final String PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT =
      "sequent.read-only.expirationDate";
  public final String NEW_PASSWORD_EMAIL_SUBJECT = "newPassword.email.subject";
  public final String NEW_PASSWORD_EMAIL_FTL = "forgot-password-send-new-password.ftl";

  public final String RECAPTCHA_G_RESPONSE = "g-recaptcha-response";
  public final String RECAPTCHA_API_JS_URL = "https://www.google.com/recaptcha/api.js";
  public final String RECAPTCHA_SITE_VERIFY_URL = "https://www.google.com/recaptcha/api/siteverify";

  public final String RECAPTCHA_ACTION_NAME_ATTRIBUTE = "recaptchaActionName";
  public final String RECAPTCHA_ACTION_NAME_ATTRIBUTE_DEFAULT = "login";
  public final String RECAPTCHA_ACTION_NAME_FORGOT_ATTRIBUTE_DEFAULT = "login";

  public final String RECAPTCHA_SITE_KEY_ATTRIBUTE = "recaptchaSiteKey";
  public final String RECAPTCHA_SITE_SECRET_ATTRIBUTE = "siteSecret";
  public final String RECAPTCHA_ENABLED_ATTRIBUTE = "recaptchaEnabled";
  public final String RECAPTCHA_MIN_SCORE_ATTRIBUTE = "recaptchaMinScore";

  public String getString(AuthenticatorConfigModel config, String configKey) {
    return getString(config, configKey, "");
  }

  public String getString(AuthenticatorConfigModel config, String configKey, String defaultValue) {
    log.debugv("getString(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.debugv("getString(): NULL config={0}", config);
      return defaultValue;
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.debugv("getString(): NullOrNotFound mapConfig={0}", mapConfig);
      return defaultValue;
    }
    return mapConfig.get(configKey);
  }

  public List<String> getMultivalueString(
      AuthenticatorConfigModel config, String configKey, List<String> defaultValue) {
    log.debugv("getMultivalueString(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.debugv("getMultivalueString(): NULL config={0}", config);
      return defaultValue;
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.debugv("getMultivalueString(): NullOrNotFound mapConfig={0}", mapConfig);
      return defaultValue;
    }

    log.debugv("getMultivalueString(): value={0}", mapConfig.get(configKey));

    return Arrays.asList(mapConfig.get(configKey).split("##"));
  }

  public int getInt(AuthenticatorConfigModel config, String configKey, String defaultValue) {
    log.debugv("getInt(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.debugv("getInt(): NULL config={0}", config);
      return Integer.parseInt(defaultValue);
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.debugv("getInt(): NullOrNotFound mapConfig={0}", mapConfig);
      return Integer.parseInt(defaultValue);
    }
    return Integer.parseInt(mapConfig.get(configKey));
  }

  public boolean getBoolean(
      AuthenticatorConfigModel config, String configKey, boolean defaultValue) {
    log.debugv("getBoolean(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.debugv("getBoolean(): NULL config={0}", config);
      return defaultValue;
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.debugv("getBoolean(): NullOrNotFound mapConfig={0}", mapConfig);
      return defaultValue;
    }
    return Boolean.parseBoolean(mapConfig.get(configKey));
  }

  int getPasswordLength(AuthenticatorConfigModel config) {
    return getInt(config, Utils.PASSWORD_LENGTH, Utils.PASSWORD_LENGTH_DEFAULT);
  }

  String getPasswordChars(AuthenticatorConfigModel config) {
    return getString(config, Utils.PASSWORD_CHARS, Utils.PASSWORD_CHARS_DEFAULT);
  }

  int getPasswordExpirationSeconds(AuthenticatorConfigModel config) {
    return getInt(
        config, Utils.PASSWORD_EXPIRATION_SECONDS, Utils.PASSWORD_EXPIRATION_SECONDS_DEFAULT);
  }

  String getPasswordExpirationUserAttribute(AuthenticatorConfigModel config) {
    return getString(
        config,
        Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE,
        Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT);
  }

  Optional<AuthenticatorConfigModel> getConfig(RealmModel realm) {
    // Using streams to find the first matching configuration
    // TODO: We're assuming there's only one instance in this realm of this
    // authenticator
    Optional<AuthenticatorConfigModel> configOptional =
        realm
            .getAuthenticationFlowsStream()
            .flatMap(flow -> realm.getAuthenticationExecutionsStream(flow.getId()))
            .filter(
                model -> {
                  boolean ret =
                      (model.getAuthenticator() != null
                          && model.getAuthenticator().equals(ChooseUser.PROVIDER_ID));
                  return ret;
                })
            .map(model -> realm.getAuthenticatorConfigById(model.getAuthenticatorConfig()))
            .findFirst();
    return configOptional;
  }

  public static String getRealmName(RealmModel realm) {
    return Strings.isNullOrEmpty(realm.getDisplayName()) ? realm.getName() : realm.getDisplayName();
  }

  public static void sendNewPasswordNotification(
      KeycloakSession session, UserModel user, String temporaryPassword) throws EmailException {
    log.infov("sendNewPasswordNotification(): to user with email={0}", user.getEmail());
    RealmModel realm = session.getContext().getRealm();
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);
    String realmName = getRealmName(realm);
    List<Object> subjAttr = ImmutableList.of(realmName);
    Map<String, Object> bodyAttr = Maps.newHashMap();
    bodyAttr.put("realmName", realmName);
    bodyAttr.put("temporaryPassword", temporaryPassword);
    emailTemplateProvider
        .setRealm(realm)
        .setUser(user)
        .setAttribute("realmName", realmName)
        .send(Utils.NEW_PASSWORD_EMAIL_SUBJECT, subjAttr, Utils.NEW_PASSWORD_EMAIL_FTL, bodyAttr);
  }

  public boolean validateRecaptcha(
      AuthenticationFlowContext context,
      boolean success,
      String captcha,
      String secret,
      Double minScore) {
    log.info("validateRecaptcha()");
    HttpClient httpClient =
        context.getSession().getProvider(HttpClientProvider.class).getHttpClient();
    HttpPost post = new HttpPost(Utils.RECAPTCHA_SITE_VERIFY_URL);
    List<NameValuePair> formparams = new LinkedList<>();
    formparams.add(new BasicNameValuePair("secret", secret));
    formparams.add(new BasicNameValuePair("response", captcha));
    formparams.add(new BasicNameValuePair("remoteip", context.getConnection().getRemoteAddr()));
    log.debugv("validateRecaptcha(): secret={0},  captcha={1}", secret, captcha);
    try {
      UrlEncodedFormEntity form = new UrlEncodedFormEntity(formparams, "UTF-8");
      post.setEntity(form);
      HttpResponse response = httpClient.execute(post);
      InputStream content = response.getEntity().getContent();
      InputStreamReader isr = new InputStreamReader(content);
      BufferedReader br = new BufferedReader(isr);
      StringBuilder result = new StringBuilder();
      String line;
      while ((line = br.readLine()) != null) {
        result.append(line);
      }
      log.debugv("recaptcha result = {0}", result.toString());
      try {
        Object scoreObj = JsonSerialization.readValue(result.toString(), Map.class).get("score");
        Double userScore = Double.parseDouble((scoreObj != null) ? scoreObj.toString() : "0");
        log.infov(
            "validateRecaptcha() userScore[{0}] > minScore[{1}] = [{2}]",
            userScore, minScore, (userScore > minScore));
        if (userScore > minScore) {
          success = true;
        } else {
          success = false;
        }
      } finally {
        content.close();
      }
    } catch (Exception error) {
      log.infov("validateRecaptcha(): error {0}", error);
    }
    return success;
  }

  public String buildURLWithParams(String baseURL, Map<String, String> params) {
    StringJoiner query = new StringJoiner("&");

    for (Map.Entry<String, String> entry : params.entrySet()) {
      String encodedKey = URLEncoder.encode(entry.getKey(), StandardCharsets.UTF_8);
      String encodedValue = URLEncoder.encode(entry.getValue(), StandardCharsets.UTF_8);
      query.add(encodedKey + "=" + encodedValue);
    }

    return baseURL + "?" + query.toString();
  }

  public void addRecaptchaChallenge(
      AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    boolean recaptchaEnabled =
        Utils.getBoolean(authConfig, Utils.RECAPTCHA_ENABLED_ATTRIBUTE, false);

    LoginFormsProvider forms = context.form();
    if (recaptchaEnabled) {
      String recaptchaSiteKey =
          Utils.getString(authConfig, Utils.RECAPTCHA_SITE_KEY_ATTRIBUTE).strip();
      String recaptchaActionName =
          Utils.getString(authConfig, Utils.RECAPTCHA_ACTION_NAME_ATTRIBUTE).strip();
      forms.setAttribute(Utils.RECAPTCHA_ENABLED_ATTRIBUTE, true);
      forms.setAttribute(Utils.RECAPTCHA_ACTION_NAME_ATTRIBUTE, recaptchaActionName);
      forms.setAttribute(Utils.RECAPTCHA_SITE_KEY_ATTRIBUTE, recaptchaSiteKey);
      String userLanguageTag =
          context.getSession().getContext().resolveLocale(context.getUser()).toLanguageTag();
      Map<String, String> params = new HashMap<>();
      params.put("hl", userLanguageTag);
      params.put("render", recaptchaSiteKey);
      params.put("onload", "onRecaptchaLoaded");

      String apiJsUrl = buildURLWithParams(RECAPTCHA_API_JS_URL, params);
      forms.addScript(apiJsUrl);
    }
  }

  /** We use constant time comparison for security reasons, to avoid timing attacks */
  boolean constantTimeIsEqual(byte[] digesta, byte[] digestb) {
    if (digesta.length != digestb.length) {
      return false;
    }

    int result = 0;
    // time-constant comparison
    for (int i = 0; i < digesta.length; i++) {
      result |= digesta[i] ^ digestb[i];
    }
    return result == 0;
  }
}
