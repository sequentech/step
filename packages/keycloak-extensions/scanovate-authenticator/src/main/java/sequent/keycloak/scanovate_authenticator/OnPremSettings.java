// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.net.URI;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.stream.Collectors;

/**
 * How the voter's browser reaches the capture pages of the on-premise Scanovate services, OCR and
 * Liveness Plus, as Scanovate integrates them: the capture page embeds each service's own page,
 * which starts a session with a one-time token issued by Keycloak. The verdict reaches Keycloak
 * server to server, see {@link OnPremSessions}.
 *
 * <p>Both services are served under the same browser-facing base URL, at {@code <url>/ocr/} and
 * {@code <url>/liveness/}: the {@code CLIENT_BASE_URL_PREFIX} of the services, behind a reverse
 * proxy on Keycloak's origin, so that the pages can use the camera inside the iframes.
 *
 * @param url browser-facing base URL of the services
 * @param secret shared with both services, which send it in the token verification and callback
 *     URLs of their service_config.json; never sent to the browser
 * @param resultWaitSeconds how long to wait for a result callback once the voter is done
 */
public record OnPremSettings(URI url, String secret, int resultWaitSeconds) {
  /**
   * Reads the settings from the authenticator configuration.
   *
   * @throws ScanovateException if the URL or the secret are missing, or a setting is malformed
   */
  public static OnPremSettings fromConfig(Map<String, String> config) throws ScanovateException {
    URI url = ServiceUrls.parse(config, ScanovateAuthenticatorFactory.LIVENESS_URL);
    String secret = config.get(ScanovateAuthenticatorFactory.LIVENESS_SECRET);
    if (secret == null || secret.isBlank()) {
      throw new ScanovateException("Missing Scanovate callback secret");
    }
    return new OnPremSettings(
        url,
        secret,
        positiveInt(
            config,
            ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS,
            ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_RESULT_WAIT_SECONDS));
  }

  /**
   * URL of the service's capture page for a session of the case, with the token the service checks
   * with Keycloak. The other parameters of the page, such as its configuration files and language,
   * default to the {@code onprem.default_parameters} of the service.
   *
   * @param parameters other query parameters of the page, such as OCR's {@code ocr_type}
   */
  public String pageUrl(
      OnPremService service, String caseId, String token, Map<String, String> parameters) {
    Map<String, String> query = new LinkedHashMap<>(parameters);
    query.put("case_id", caseId);
    query.put("token", token);
    return url
        + "/"
        + service.path()
        + "/?"
        + query.entrySet().stream()
            .map(entry -> encode(entry.getKey()) + "=" + encode(entry.getValue()))
            .collect(Collectors.joining("&"));
  }

  /** Origin of the services' pages, which the capture page embeds. */
  public String origin() {
    return url.getScheme() + "://" + url.getRawAuthority();
  }

  private static String encode(String value) {
    return URLEncoder.encode(value, StandardCharsets.UTF_8).replace("+", "%20");
  }

  static int positiveInt(Map<String, String> config, String key, int defaultValue)
      throws ScanovateException {
    String value = config.get(key);
    if (value == null || value.isBlank()) {
      return defaultValue;
    }
    int parsed;
    try {
      parsed = Integer.parseInt(value.trim());
    } catch (NumberFormatException e) {
      throw new ScanovateException("Invalid " + key + ": " + value, e);
    }
    if (parsed <= 0) {
      throw new ScanovateException("Invalid " + key + ": " + value);
    }
    return parsed;
  }
}
