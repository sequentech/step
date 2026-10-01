// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.net.URI;
import java.util.Map;

/**
 * How the capture page reaches Liveness Plus, for the {@link FaceCapture#LIVENESS} face capture.
 *
 * <p>There is no Liveness Plus UI in on-premise deployments: the capture page calls the Liveness
 * Plus API itself, starting a session with a one-time token issued by Keycloak and sending it the
 * frames of the voter's face. The verdict reaches Keycloak server to server, see {@link
 * LivenessSessions}.
 *
 * @param url browser-facing base URL of the Liveness Plus service
 * @param secret shared with Liveness Plus, which sends it in the token verification and callback
 *     URLs of its service_config.json; never sent to the browser
 * @param resultWaitSeconds how long to wait for the result callback once the voter is done
 */
public record LivenessSettings(URI url, String secret, int resultWaitSeconds) {
  static final String API_PATH = "/liveness";

  /**
   * Reads the settings from the authenticator configuration.
   *
   * @throws ScanovateException if the URL or the secret are missing, or a setting is malformed
   */
  public static LivenessSettings fromConfig(Map<String, String> config) throws ScanovateException {
    URI url = ServiceUrls.parse(config, ScanovateAuthenticatorFactory.LIVENESS_URL);
    String secret = config.get(ScanovateAuthenticatorFactory.LIVENESS_SECRET);
    if (secret == null || secret.isBlank()) {
      throw new ScanovateException("Missing liveness callback secret");
    }
    return new LivenessSettings(
        url,
        secret,
        CaptureSettings.positiveInt(
            config,
            ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS,
            ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_RESULT_WAIT_SECONDS));
  }

  /** Base URL of the Liveness Plus API (create_session, check_liveness, ...) for the browser. */
  public String apiUrl() {
    return url + API_PATH;
  }
}
