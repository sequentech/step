// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.container.ContainerRequestContext;
import jakarta.ws.rs.container.ContainerRequestFilter;
import jakarta.ws.rs.container.PreMatching;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.ext.Provider;
import java.io.ByteArrayOutputStream;
import java.net.URI;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Pattern;

/**
 * Validates login hints before Keycloak's authorization-request parser copies additional parameters
 * into client notes.
 *
 * <p>This filter deliberately reads the raw query rather than {@code UriInfo#getQueryParameters}:
 * duplicate parameters and malformed percent escapes are security-relevant and must not be lost
 * during framework decoding. Hint values are never included in responses or logs.
 */
@Provider
@PreMatching
public final class LoginHintAuthorizationRequestFilter implements ContainerRequestFilter {

  static final String INVALID_REQUEST_ERROR = "invalid_request";
  static final String INVALID_REQUEST_DESCRIPTION = "Invalid login hint parameters";

  private static final Pattern AUTHORIZATION_PATH =
      Pattern.compile("/realms/[^/]+/protocol/openid-connect/(?:auth|registrations)/?");

  @Override
  public void filter(ContainerRequestContext requestContext) {
    URI requestUri = requestContext.getUriInfo().getRequestUri();
    if (!isAuthorizationEndpoint(requestUri.getRawPath())) {
      return;
    }

    try {
      validateRawQuery(requestUri.getRawQuery());
    } catch (IllegalArgumentException invalidHints) {
      requestContext.abortWith(invalidRequestResponse());
    }
  }

  static boolean isAuthorizationEndpoint(String rawPath) {
    if (rawPath == null) {
      return false;
    }
    // JAX-RS ignores matrix parameters while matching resource paths, so classify the same path
    // here or a routed auth endpoint could bypass this raw-query trust boundary.
    String pathWithoutMatrixParameters = rawPath.replaceAll(";[^/]*", "");
    String normalizedPath =
        pathWithoutMatrixParameters.startsWith("/")
            ? pathWithoutMatrixParameters
            : "/" + pathWithoutMatrixParameters;
    int realmsSegment = normalizedPath.lastIndexOf("/realms/");
    if (realmsSegment < 0 || normalizedPath.substring(0, realmsSegment).endsWith("/admin")) {
      return false;
    }
    return AUTHORIZATION_PATH.matcher(normalizedPath.substring(realmsSegment)).matches();
  }

  /**
   * Validates the complete raw hint set atomically while preserving duplicate query components.
   *
   * @param rawQuery undecoded request query, without the leading question mark
   * @return validated hints, primarily exposed for focused tests
   * @throws IllegalArgumentException when any hint is malformed or outside the contract bounds
   */
  static Map<String, String> validateRawQuery(String rawQuery) {
    if (rawQuery == null || rawQuery.isEmpty()) {
      return Map.of();
    }

    Map<String, String> hints = new LinkedHashMap<>();
    for (String component : rawQuery.split("&", -1)) {
      int separator = component.indexOf('=');
      String rawName = separator < 0 ? component : component.substring(0, separator);
      String rawValue = separator < 0 ? "" : component.substring(separator + 1);

      String parameterName = decodeQueryComponent(rawName);
      if (!parameterName.startsWith(LoginHintPrefill.HINT_PREFIX)) {
        continue;
      }

      String field = parameterName.substring(LoginHintPrefill.HINT_PREFIX.length());
      String value = decodeQueryComponent(rawValue);
      if (hints.containsKey(field)
          || hints.size() == LoginHintPrefill.MAX_HINT_COUNT
          || !LoginHintPrefill.isValidHint(field, value)) {
        throw invalidHints();
      }
      hints.put(field, value);
    }

    return Map.copyOf(hints);
  }

  static Response invalidRequestResponse() {
    return Response.status(Response.Status.BAD_REQUEST)
        .type(MediaType.APPLICATION_JSON_TYPE)
        .header("Cache-Control", "no-store")
        .header("Pragma", "no-cache")
        .entity(
            Map.of(
                "error", INVALID_REQUEST_ERROR,
                "error_description", INVALID_REQUEST_DESCRIPTION))
        .build();
  }

  private static String decodeQueryComponent(String rawComponent) {
    ByteArrayOutputStream decodedBytes = new ByteArrayOutputStream(rawComponent.length());

    for (int offset = 0; offset < rawComponent.length(); ) {
      int codePoint = rawComponent.codePointAt(offset);
      if (codePoint == '%') {
        if (offset + 2 >= rawComponent.length()) {
          throw invalidHints();
        }
        int high = Character.digit(rawComponent.charAt(offset + 1), 16);
        int low = Character.digit(rawComponent.charAt(offset + 2), 16);
        if (high < 0 || low < 0) {
          throw invalidHints();
        }
        decodedBytes.write((high << 4) | low);
        offset += 3;
      } else if (codePoint == '+') {
        decodedBytes.write(' ');
        offset++;
      } else {
        decodedBytes.writeBytes(
            new String(Character.toChars(codePoint)).getBytes(StandardCharsets.UTF_8));
        offset += Character.charCount(codePoint);
      }
    }

    try {
      return StandardCharsets.UTF_8
          .newDecoder()
          .onMalformedInput(CodingErrorAction.REPORT)
          .onUnmappableCharacter(CodingErrorAction.REPORT)
          .decode(ByteBuffer.wrap(decodedBytes.toByteArray()))
          .toString();
    } catch (CharacterCodingException malformedEncoding) {
      throw invalidHints();
    }
  }

  private static IllegalArgumentException invalidHints() {
    return new IllegalArgumentException(INVALID_REQUEST_DESCRIPTION);
  }
}
