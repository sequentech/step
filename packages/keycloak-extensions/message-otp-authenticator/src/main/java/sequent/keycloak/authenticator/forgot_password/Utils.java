// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.userprofile.UserProfileProvider;
import org.keycloak.util.JsonSerialization;

@UtilityClass
@JBossLog
public class Utils {
  public static final String USERNAME_ATTRIBUTES = "usernameAttributes";
  public static final List<String> USERNAME_ATTRIBUTES_DEFAULT =
      Collections.unmodifiableList(Arrays.asList("username"));
  public static final String MATCH_ATTRIBUTES = "matchAttributes";
  public static final List<String> MATCH_ATTRIBUTES_DEFAULT = Collections.emptyList();
  public static final String MAX_CANDIDATES = "maxCandidates";
  public static final String MAX_CANDIDATES_DEFAULT = "10";
  public static final String TUPLE_MAX_FAILURES = "tupleMaxFailures";
  public static final String TUPLE_MAX_FAILURES_DEFAULT = "10";
  public static final String TUPLE_FAILURE_WINDOW_SECONDS = "tupleFailureWindowSeconds";
  public static final String TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT = "60";

  /**
   * Selects {@link MultiAttributeCredentialResolver.MatchPolicy}. Defaults to the safe {@code
   * REJECT_AMBIGUOUS} - {@code FIRST_MATCH} must only be enabled when passwords are guaranteed
   * unique across every candidate a request could match, since it authenticates as the first
   * candidate whose password matches without checking for other matches.
   */
  public static final String MATCH_POLICY = "matchPolicy";

  public static final String MATCH_POLICY_DEFAULT =
      MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS.name();
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

  /**
   * Reads the DoS-mitigation config shared by {@link MultiAttributePasswordAuthenticator} and
   * {@link MultiAttributePasswordDirectGrantAuthenticator} - see {@link
   * MultiAttributeCredentialResolver.ThrottleConfig}.
   */
  public MultiAttributeCredentialResolver.ThrottleConfig getThrottleConfig(
      AuthenticatorConfigModel config) {
    return new MultiAttributeCredentialResolver.ThrottleConfig(
        getInt(config, MAX_CANDIDATES, MAX_CANDIDATES_DEFAULT),
        getInt(config, TUPLE_MAX_FAILURES, TUPLE_MAX_FAILURES_DEFAULT),
        getInt(config, TUPLE_FAILURE_WINDOW_SECONDS, TUPLE_FAILURE_WINDOW_SECONDS_DEFAULT));
  }

  /** Reads {@link #MATCH_POLICY}, defaulting to the safe {@code REJECT_AMBIGUOUS}. */
  public MultiAttributeCredentialResolver.MatchPolicy getMatchPolicy(
      AuthenticatorConfigModel config) {
    return MultiAttributeCredentialResolver.MatchPolicy.fromString(
        getString(config, MATCH_POLICY, MATCH_POLICY_DEFAULT));
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

  /**
   * Fetches the realm's User Profile attribute declarations, tolerating a missing {@link
   * UserProfileProvider}, a missing configuration, or a missing attribute list - any of which
   * yields an empty list rather than throwing, so callers never need their own null checks.
   */
  public List<UPAttribute> getRealmUserProfileAttributes(KeycloakSession session) {
    UserProfileProvider userProfileProvider = session.getProvider(UserProfileProvider.class);
    if (userProfileProvider == null || userProfileProvider.getConfiguration() == null) {
      return Collections.emptyList();
    }
    List<UPAttribute> attributes = userProfileProvider.getConfiguration().getAttributes();
    return attributes == null ? Collections.emptyList() : attributes;
  }

  /**
   * Looks up the HTML5 input type ({@code date}, {@code email}, ...) that {@code attributeName}
   * declares via its {@code inputType} annotation (e.g. {@code html5-date}) among the given User
   * Profile attributes - the same source registration/profile forms (user-profile-commons.ftl)
   * already render from; see {@link #getRealmUserProfileAttributes}. Falls back to {@code text}
   * when the attribute has no User Profile entry, or its declared input type isn't an {@code
   * html5-*} one (e.g. {@code select}, {@code textarea} - not meaningful for a single login lookup
   * field).
   */
  public String resolveHtml5InputType(List<UPAttribute> attributes, String attributeName) {
    Object inputType =
        attributes.stream()
            .filter(attribute -> attributeName.equals(attribute.getName()))
            .findFirst()
            .map(UPAttribute::getAnnotations)
            .map(annotations -> annotations.get("inputType"))
            .orElse(null);
    if (!(inputType instanceof String) || !((String) inputType).startsWith("html5-")) {
      return "text";
    }
    return ((String) inputType).substring("html5-".length());
  }

  /**
   * Reformats a date-like value into the canonical {@code YYYY-MM-DD} form user attributes are
   * stored/searched in (the same format an HTML5 date input always submits), given the digit order
   * it arrived in (e.g. {@code "YYYY-MM-DD"} for an HTML5 date input - a no-op reformat - or {@code
   * "MMDDYYYY"} for 8 raw IVR DTMF digits). Non-digit characters in {@code rawValue} (separators)
   * are stripped before reordering, so callers don't need to pre-clean input. Returns {@code
   * rawValue} unchanged if its digit count doesn't match {@code sourceFormat}'s, rather than
   * guessing.
   */
  public String normalizeDate(String rawValue, String sourceFormat) {
    if (rawValue == null) {
      return null;
    }
    String digits = rawValue.replaceAll("[^0-9]", "");
    String formatLetters = sourceFormat.replaceAll("[^YMDymd]", "").toUpperCase();
    if (digits.length() != formatLetters.length()) {
      return rawValue;
    }

    StringBuilder year = new StringBuilder();
    StringBuilder month = new StringBuilder();
    StringBuilder day = new StringBuilder();
    for (int i = 0; i < formatLetters.length(); i++) {
      switch (formatLetters.charAt(i)) {
        case 'Y' -> year.append(digits.charAt(i));
        case 'M' -> month.append(digits.charAt(i));
        case 'D' -> day.append(digits.charAt(i));
        default -> {
          // unrecognized letter in sourceFormat; ignore this position
        }
      }
    }
    return year + "-" + month + "-" + day;
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
