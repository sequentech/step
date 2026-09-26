// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import jakarta.ws.rs.core.MultivaluedMap;
import java.net.URI;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;

/**
 * URL the voter returns to from B-Trust.
 *
 * <p>B-Trust appends {@code processId} and {@code token} to the redirect URL. Keycloak's
 * registration endpoint reads {@code token} as an action token and fails, so B-Trust redirects to
 * {@link ScanovateReturnResource} instead, which forwards only the Keycloak parameters and the
 * process id to the login actions URL of the same realm.
 */
final class ReturnUrl {
  static final String FLOW_PARAM = "flow";

  private static final String LOGIN_ACTIONS_SEGMENT = "/login-actions/";
  private static final List<String> FORWARDED_PARAMS =
      List.of(
          "session_code",
          "execution",
          "client_id",
          "tab_id",
          "client_data",
          ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM);

  private enum LoginActionsFlow {
    AUTHENTICATE("authenticate"),
    REGISTRATION("registration"),
    RESET_CREDENTIALS("reset-credentials"),
    FIRST_BROKER_LOGIN("first-broker-login"),
    POST_BROKER_LOGIN("post-broker-login");

    private final String path;

    LoginActionsFlow(String path) {
      this.path = path;
    }

    static Optional<LoginActionsFlow> fromPath(String path) {
      return Arrays.stream(values()).filter(flow -> flow.path.equals(path)).findFirst();
    }
  }

  private ReturnUrl() {}

  static URI fromActionUrl(URI actionUrl) throws ScanovateException {
    String path = actionUrl.getRawPath();
    int index = path == null ? -1 : path.lastIndexOf(LOGIN_ACTIONS_SEGMENT);
    if (index < 0) {
      throw new ScanovateException("Not a login actions URL: " + actionUrl);
    }
    String flow = path.substring(index + LOGIN_ACTIONS_SEGMENT.length());
    String query = FLOW_PARAM + "=" + encode(flow);
    if (actionUrl.getRawQuery() != null) {
      query += "&" + actionUrl.getRawQuery();
    }
    return URI.create(
        actionUrl.getScheme()
            + "://"
            + actionUrl.getRawAuthority()
            + path.substring(0, index)
            + "/"
            + ScanovateReturnResourceFactory.PROVIDER_ID
            + "/"
            + ScanovateReturnResource.RETURN_PATH
            + "?"
            + query);
  }

  static Optional<URI> toActionUrl(URI loginActionsBase, MultivaluedMap<String, String> params) {
    return LoginActionsFlow.fromPath(params.getFirst(FLOW_PARAM))
        .map(
            flow -> {
              String query =
                  FORWARDED_PARAMS.stream()
                      .filter(param -> params.getFirst(param) != null)
                      .map(param -> param + "=" + encode(params.getFirst(param)))
                      .collect(Collectors.joining("&"));
              return URI.create(
                  loginActionsBase + "/" + flow.path + (query.isEmpty() ? "" : "?" + query));
            });
  }

  private static String encode(String value) {
    return URLEncoder.encode(value, StandardCharsets.UTF_8);
  }
}
