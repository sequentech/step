// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.net.URI;
import java.net.URISyntaxException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;

/**
 * How the Liveness Plus iframe is opened, for the {@link FaceCapture#LIVENESS} face capture.
 *
 * @param url browser-facing base URL of the Liveness Plus service
 * @param secret shared with Liveness Plus, which sends it in the token verification and callback
 *     URLs of its service_config.json; never sent to the browser
 * @param uiTheme Liveness Plus UI configuration (ui_theme) giving the iframe the Sequent look
 * @param translationVariant Liveness Plus translation variant with the Sequent wording
 * @param languages languages of the translation variant; the voter's language is used if listed
 * @param resultWaitSeconds how long to wait for the result callback once the voter is done
 */
public record LivenessSettings(
    URI url,
    String secret,
    String uiTheme,
    String translationVariant,
    List<String> languages,
    int resultWaitSeconds) {
  static final String LIVENESS_PATH = "/liveness/";
  private static final Set<String> SCHEMES = Set.of("http", "https");

  public LivenessSettings {
    languages = List.copyOf(languages);
  }

  /**
   * Reads the settings from the authenticator configuration.
   *
   * @throws ScanovateException if the URL or the secret are missing, or a setting is malformed
   */
  public static LivenessSettings fromConfig(Map<String, String> config) throws ScanovateException {
    String rawUrl = config.getOrDefault(ScanovateAuthenticatorFactory.LIVENESS_URL, "").trim();
    URI url;
    try {
      url = new URI(rawUrl.endsWith("/") ? rawUrl.substring(0, rawUrl.length() - 1) : rawUrl);
    } catch (URISyntaxException e) {
      throw new ScanovateException("Invalid liveness URL: " + rawUrl, e);
    }
    if (url.getScheme() == null || !SCHEMES.contains(url.getScheme()) || url.getHost() == null) {
      throw new ScanovateException("Invalid liveness URL: " + rawUrl);
    }
    String secret = config.get(ScanovateAuthenticatorFactory.LIVENESS_SECRET);
    if (secret == null || secret.isBlank()) {
      throw new ScanovateException("Missing liveness callback secret");
    }
    return new LivenessSettings(
        url,
        secret,
        orDefault(
            config,
            ScanovateAuthenticatorFactory.LIVENESS_UI_THEME,
            ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_UI_THEME),
        orDefault(
            config,
            ScanovateAuthenticatorFactory.LIVENESS_TRANSLATION_VARIANT,
            ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_TRANSLATION_VARIANT),
        Arrays.stream(
                orDefault(
                        config,
                        ScanovateAuthenticatorFactory.LIVENESS_LANGUAGES,
                        ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_LANGUAGES)
                    .split(","))
            .map(String::trim)
            .filter(language -> !language.isEmpty())
            .toList(),
        CaptureSettings.positiveInt(
            config,
            ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS,
            ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_RESULT_WAIT_SECONDS));
  }

  /** Origin of the iframe, allowed in Keycloak's Content Security Policy. */
  public String origin() {
    return url.getScheme()
        + "://"
        + url.getHost()
        + (url.getPort() == -1 ? "" : ":" + url.getPort());
  }

  /**
   * URL of the Liveness Plus UI for a token. The browser adds translation_language with the voter's
   * language when it is one of {@link #languages()}.
   */
  public String iframeUrl(String token, String caseId) {
    Map<String, String> query = new LinkedHashMap<>();
    query.put("scan_config", "scan_config");
    query.put("video_config", "video_config");
    query.put("translation_variant", translationVariant);
    query.put("ui_theme", uiTheme);
    query.put("case_id", caseId);
    query.put("token", token);
    return url
        + LIVENESS_PATH
        + "?"
        + query.entrySet().stream()
            .map(entry -> entry.getKey() + "=" + encode(entry.getValue()))
            .collect(Collectors.joining("&"));
  }

  private static String encode(String value) {
    return URLEncoder.encode(value, StandardCharsets.UTF_8).replace("+", "%20");
  }

  private static String orDefault(Map<String, String> config, String key, String defaultValue) {
    String value = config.get(key);
    return value == null || value.isBlank() ? defaultValue : value.trim();
  }
}
