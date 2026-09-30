// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.net.URI;
import java.net.URLDecoder;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import sequent.keycloak.scanovate_authenticator.HttpTransport.MultipartPart;

/**
 * Client for the B-Trust (Scanovate) identity verification API.
 *
 * <p>Implements the flow described in the B-Trust v3.8.2 Identity Verification Tech Specs: obtain
 * an OAuth token, create a one-time session link, and once the voter has completed the flow,
 * exchange the process id for a session token and fetch the results.
 *
 * <p>The media upload used by the embedded mode ({@link #uploadMedia}) is not part of v3.8.2: it is
 * a proposed endpoint, pending Scanovate's confirmation.
 */
public class ScanovateClient {
  static final String AUTH_TOKEN_PATH = "/auth/token";
  static final String FLOW_LINK_PATH = "/flow/v3/link";
  static final String MOBILE_INTERACTION_PATH = "/api/v3/mobile_interaction/";
  static final String SESSION_TOKEN_SUFFIX = "/token";
  static final String FAST_RESULTS_SUFFIX = "/results_with_image_names";
  static final String MEDIA_SUFFIX = "/media";
  static final String PROCESS_ID_QUERY_PARAM = "process_id";

  /** Waits between retries; injectable so that tests don't sleep. */
  @FunctionalInterface
  public interface Sleeper {
    void sleep(long millis) throws InterruptedException;
  }

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final HttpTransport transport;
  private final String baseUrl;
  private final String clientId;
  private final String clientSecret;
  private final RetryingRequests requests;

  public ScanovateClient(
      HttpTransport transport,
      String baseUrl,
      String clientId,
      String clientSecret,
      int maxRetries,
      Sleeper sleeper) {
    this.transport = transport;
    this.baseUrl = baseUrl.endsWith("/") ? baseUrl.substring(0, baseUrl.length() - 1) : baseUrl;
    this.clientId = clientId;
    this.clientSecret = clientSecret;
    this.requests = new RetryingRequests(maxRetries, sleeper);
  }

  /** {@code POST /auth/token}. */
  public String fetchAccessToken() throws IOException {
    ObjectNode body = MAPPER.createObjectNode();
    body.put("client_id", clientId);
    body.put("client_secret", clientSecret);
    JsonNode response = post(AUTH_TOKEN_PATH, Map.of(), body);
    return requiredText(response, "access_token", AUTH_TOKEN_PATH);
  }

  /** {@code POST /flow/v3/link}. */
  public SessionLink createSessionLink(String accessToken, LinkRequest request) throws IOException {
    ObjectNode body = MAPPER.createObjectNode();
    body.put("flow_id", request.flowId());
    body.put("identifier_id", request.identifierId());
    if (request.idNumber() != null && !request.idNumber().isBlank()) {
      body.put("id_number", request.idNumber());
    }
    body.put("redirect_url", request.redirectUrl());
    // Where the desktop browser, which holds the Keycloak session, returns when the voter
    // continues the flow on a phone
    body.put("desktop_redirect_url", request.redirectUrl());
    ObjectNode params = body.putObject("params");
    request.params().forEach(params::put);
    if (request.saveOption() != SaveOption.DEFAULT) {
      body.put("save_option", request.saveOption().value());
    }

    JsonNode response = post(FLOW_LINK_PATH, bearer(accessToken), body);
    requireSuccess(response, FLOW_LINK_PATH);
    String url = requiredText(response, "data", FLOW_LINK_PATH);
    String processId = queryParam(url, PROCESS_ID_QUERY_PARAM).orElse(request.identifierId());
    return new SessionLink(url, processId);
  }

  /**
   * {@code POST /api/v3/mobile_interaction/{session}/media}: uploads the files captured in the
   * voter's browser to the session, as {@code multipart/form-data} with the parts {@code
   * front_image}, {@code back_image} (only for documents with a back side), {@code face_image} and
   * {@code scan_video}. The response is {@code {"success": true, "errorCode": 0}}.
   *
   * <p><b>Proposed endpoint, pending Scanovate's confirmation.</b> B-Trust v3.8.2 does not document
   * how to submit media captured outside its own flow UI; the e2e mock server implements this
   * proposal.
   */
  public void uploadMedia(String accessToken, String processId, CaptureMedia media)
      throws IOException {
    String path = MOBILE_INTERACTION_PATH + encodePathSegment(processId) + MEDIA_SUFFIX;
    List<MultipartPart> parts =
        media.files().entrySet().stream()
            .map(
                file ->
                    new MultipartPart(
                        file.getKey().uploadPart(),
                        file.getKey().uploadPart() + "." + file.getValue().format().extension(),
                        file.getValue().format().contentType(),
                        file.getValue().content()))
            .toList();
    JsonNode response =
        requests.execute(
            path, () -> transport.postMultipart(baseUrl + path, bearer(accessToken), parts));
    requireSuccess(response, path);
  }

  /** {@code GET /api/v3/mobile_interaction/{session}/token}. */
  public String fetchSessionToken(String accessToken, String processId) throws IOException {
    String path = MOBILE_INTERACTION_PATH + encodePathSegment(processId) + SESSION_TOKEN_SUFFIX;
    return requiredText(get(path, bearer(accessToken)), "token", path);
  }

  /** {@code GET /api/v3/mobile_interaction/v2/{sessionToken}/results_with_image_names}. */
  public JsonNode fetchResults(String sessionToken) throws IOException {
    String path =
        MOBILE_INTERACTION_PATH + "v2/" + encodePathSegment(sessionToken) + FAST_RESULTS_SUFFIX;
    return get(path, bearer(sessionToken));
  }

  /**
   * Fetches the results of a session from its process id.
   *
   * <p>The session token is always obtained server to server instead of trusting the one that
   * B-Trust appends to the redirect URL, as the latter goes through the voter's browser.
   */
  public JsonNode fetchResultsForProcess(String processId) throws IOException {
    String accessToken = fetchAccessToken();
    String sessionToken = fetchSessionToken(accessToken, processId);
    return fetchResults(sessionToken);
  }

  private JsonNode get(String path, Map<String, String> headers) throws IOException {
    return requests.execute(path, () -> transport.get(baseUrl + path, headers));
  }

  private JsonNode post(String path, Map<String, String> headers, JsonNode body)
      throws IOException {
    String payload = MAPPER.writeValueAsString(body);
    return requests.execute(path, () -> transport.postJson(baseUrl + path, headers, payload));
  }

  private static void requireSuccess(JsonNode response, String path) throws IOException {
    if (!response.path("success").asBoolean(false) || response.path("errorCode").asInt(-1) != 0) {
      throw new IOException(
          String.format(
              "%s failed: errorCode=%s data=%.200s",
              path, response.path("errorCode").asText(), response.path("data").asText()));
    }
  }

  private static Map<String, String> bearer(String token) {
    return Map.of("Authorization", "Bearer " + token);
  }

  private static String requiredText(JsonNode response, String field, String path)
      throws IOException {
    JsonNode value = response.get(field);
    if (value == null || !value.isTextual() || value.asText().isBlank()) {
      throw new IOException(path + " response is missing " + field);
    }
    return value.asText();
  }

  private static String encodePathSegment(String segment) {
    return URLEncoder.encode(segment, StandardCharsets.UTF_8).replace("+", "%20");
  }

  static Optional<String> queryParam(String url, String name) {
    String query;
    try {
      query = URI.create(url).getRawQuery();
    } catch (IllegalArgumentException e) {
      return Optional.empty();
    }
    if (query == null) {
      return Optional.empty();
    }
    for (String pair : query.split("&")) {
      int separator = pair.indexOf('=');
      if (separator > 0 && pair.substring(0, separator).equals(name)) {
        String value = URLDecoder.decode(pair.substring(separator + 1), StandardCharsets.UTF_8);
        return value.isBlank() ? Optional.empty() : Optional.of(value);
      }
    }
    return Optional.empty();
  }
}
