// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.PathSegment;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;
import java.util.ArrayList;
import java.util.List;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.services.resources.LoginActionsService;

/** Policy on enrollment documents, not on the realm or on its ordinary sign-in pages. */
public final class EnrollmentDocumentPolicy {
  public static final String HEADER = "Permissions-Policy";
  public static final String NO_DISPLAY_CAPTURE = "display-capture=()";
  private static final String DISPLAY_CAPTURE = "display-capture";

  private EnrollmentDocumentPolicy() {}

  /** An enrollment step (for example OTP) can also be used in ordinary login. */
  static Response forRegistrationRoute(AuthenticationFlowContext context, Response response) {
    UriInfo uri = context.getUriInfo();
    if (uri == null) {
      return response;
    }
    // Match Keycloak's protocol-independent registration endpoint (including under /auth),
    // not the authentication endpoint, a query parameter, or a previously used session flow.
    List<PathSegment> path = uri.getPathSegments();
    int n = path.size();
    if (n < 4
        || !"realms".equals(path.get(n - 4).getPath())
        || !"login-actions".equals(path.get(n - 2).getPath())
        || !LoginActionsService.REGISTRATION_PATH.equals(path.get(n - 1).getPath())) {
      return response;
    }
    return protectHtml(response);
  }

  /**
   * Deny calls to getDisplayMedia from this enrollment document and its frames. This is not
   * protection against OS screenshots, recordings outside this document, or physical cameras.
   */
  public static Response protectHtml(Response response) {
    if (response == null
        || !response.hasEntity()
        || response.getStatusInfo().getFamily() == Response.Status.Family.REDIRECTION
        || !MediaType.TEXT_HTML_TYPE.isCompatible(response.getMediaType())) {
      return response;
    }

    var builder = Response.fromResponse(response);
    List<String> policies = new ArrayList<>();
    response
        .getStringHeaders()
        .forEach(
            (name, values) -> {
              if (HEADER.equalsIgnoreCase(name)) {
                policies.addAll(values);
                builder.header(name, null);
              }
            });
    return builder.header(HEADER, restrict(policies)).build();
  }

  /** Keep every other directive, including origins/parameters containing quoted commas. */
  static String restrict(List<String> policies) {
    List<String> directives = new ArrayList<>();
    for (String policy : policies) {
      int start = 0;
      boolean quoted = false;
      boolean escaped = false;
      for (int i = 0; i < policy.length(); i++) {
        char ch = policy.charAt(i);
        if (escaped) {
          escaped = false;
        } else if (ch == '"') {
          quoted = !quoted;
        } else if (quoted && ch == '\\') {
          escaped = true;
        } else if (!quoted && ch == ',') {
          keepDirective(directives, policy.substring(start, i));
          start = i + 1;
        }
      }
      keepDirective(directives, policy.substring(start));
    }
    // Replace any previous value for this feature, even if it allowed display capture.
    directives.add(NO_DISPLAY_CAPTURE);
    return String.join(", ", directives);
  }

  private static void keepDirective(List<String> directives, String value) {
    String directive = value.strip();
    int separator = directive.indexOf('=');
    String name = separator < 0 ? directive : directive.substring(0, separator).strip();
    if (!directive.isEmpty() && !DISPLAY_CAPTURE.equalsIgnoreCase(name)) {
      directives.add(directive);
    }
  }
}
