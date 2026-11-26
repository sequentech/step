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
import java.util.regex.Pattern;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.services.resources.LoginActionsService;

/** Policy on enrollment documents, not on the realm or on its ordinary sign-in pages. */
public final class EnrollmentDocumentPolicy {
  public static final String HEADER = "Permissions-Policy";
  public static final String NO_DISPLAY_CAPTURE = "display-capture=()";
  private static final String DISPLAY_CAPTURE = "display-capture";
  // Permissions-Policy is a Structured Fields dictionary. Admit whole dictionary members,
  // not an unbalanced/poisoning fragment that could make a browser ignore the entire field.
  private static final String KEY = "[a-z*][a-z0-9_.*-]*";
  private static final String TOKEN = "[A-Za-z*][A-Za-z0-9!#$%&'*+.^_`|~:/-]*+";
  private static final String TEXT = "\"(?:[\\x20-\\x21\\x23-\\x5b\\x5d-\\x7e]++|\\\\[\"\\\\])*+\"";
  private static final String NUMBER = "-?(?:[0-9]{1,12}\\.[0-9]{1,3}|[0-9]{1,15})";
  private static final String BYTES =
      ":(?:[A-Za-z0-9+/]{4})*+(?:[A-Za-z0-9+/]{2}(?:==)?|[A-Za-z0-9+/]{3}=?)?:";
  private static final String BARE =
      "(?:" + TEXT + "|" + TOKEN + "|" + NUMBER + "|\\?[01]|" + BYTES + ")";
  private static final String PARAMS = "(?:; *" + KEY + "(?:=" + BARE + ")?)*+";
  private static final String ITEM = BARE + PARAMS;
  private static final String INNER_LIST = "\\( *(?:" + ITEM + "(?: +" + ITEM + ")*+)? *\\)";
  private static final Pattern MEMBER =
      Pattern.compile("(" + KEY + ")(?:=(?:" + BARE + "|" + INNER_LIST + "))?" + PARAMS);

  private EnrollmentDocumentPolicy() {}

  /** Both pinned Keycloak entry points: OIDC sign-up and the registration form action. */
  public static boolean isRegistrationRoute(UriInfo uri) {
    if (uri == null) {
      return false;
    }
    // Check the served request path, not a query parameter or a stale authentication-session note.
    // A configured relative path (e.g. /auth) can precede /realms; a trailing slash is equivalent.
    List<PathSegment> path = uri.getPathSegments();
    int end = path.size();
    while (end > 0 && path.get(end - 1).getPath().isEmpty()) {
      end--;
    }
    return (realmAt(path, end - 4)
            && "login-actions".equals(path.get(end - 2).getPath())
            && LoginActionsService.REGISTRATION_PATH.equals(path.get(end - 1).getPath()))
        || (realmAt(path, end - 5)
            && "protocol".equals(path.get(end - 3).getPath())
            && "openid-connect".equals(path.get(end - 2).getPath())
            && "registrations".equals(path.get(end - 1).getPath()));
  }

  private static boolean realmAt(List<PathSegment> path, int i) {
    return i >= 0 && "realms".equals(path.get(i).getPath()) && !path.get(i + 1).getPath().isBlank();
  }

  /** A step like OTP can also run in ordinary login. Leave those responses alone. */
  public static Response forRegistrationRoute(
      AuthenticationFlowContext context, Response response) {
    return isRegistrationRoute(context.getUriInfo()) ? protectHtml(response) : response;
  }

  /**
   * Deny calls to getDisplayMedia from this enrollment document and its frames. This is not
   * protection against OS screenshots, recordings outside this document, or physical cameras.
   */
  static Response protectHtml(Response response) {
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

  /** Keep valid unrelated members, including origins/parameters containing quoted commas. */
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
      if (!quoted && !escaped) {
        keepDirective(directives, policy.substring(start));
      }
    }
    // Replace any previous value for this feature, even if it allowed display capture.
    directives.add(NO_DISPLAY_CAPTURE);
    return String.join(", ", directives);
  }

  private static void keepDirective(List<String> directives, String value) {
    String directive = value.strip();
    var member = MEMBER.matcher(directive);
    if (member.matches() && !DISPLAY_CAPTURE.equals(member.group(1))) {
      directives.add(directive);
    }
  }
}
