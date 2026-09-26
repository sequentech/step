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
import java.util.Map;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import sequent.keycloak.scanovate_authenticator.HttpTransport.HttpResult;

/**
 * Client for the B-Trust (Scanovate) identity verification API.
 *
 * <p>Implements the flow described in the B-Trust v3.8.2 Identity Verification Tech Specs: obtain
 * an OAuth token, create a one-time session link, and once the voter has completed the flow,
 * exchange the process id for a session token and fetch the results.
 */
@JBossLog
public class ScanovateClient {
  static final String AUTH_TOKEN_PATH = "/auth/token";
  static final String FLOW_LINK_PATH = "/flow/v3/link";
  static final String MOBILE_INTERACTION_PATH = "/api/v3/mobile_interaction/";
  static final String SESSION_TOKEN_SUFFIX = "/token";
  static final String FAST_RESULTS_SUFFIX = "/results_with_image_names";
  static final String PROCESS_ID_QUERY_PARAM = "process_id";
  static final long BASE_RETRY_DELAY_MS = 1_000;

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
  private final int maxRetries;
  private final Sleeper sleeper;

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
    this.maxRetries = Math.max(1, maxRetries);
    this.sleeper = sleeper;
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
    ObjectNode params = body.putObject("params");
    request.params().forEach(params::put);
    if (request.saveOption() != SaveOption.DEFAULT) {
      body.put("save_option", request.saveOption().value());
    }

    JsonNode response = post(FLOW_LINK_PATH, bearer(accessToken), body);
    if (!response.path("success").asBoolean(false) || response.path("errorCode").asInt(-1) != 0) {
      throw new IOException(
          String.format(
              "%s failed: errorCode=%s data=%.200s",
              FLOW_LINK_PATH, response.path("errorCode").asText(), response.path("data").asText()));
    }
    String url = requiredText(response, "data", FLOW_LINK_PATH);
    String processId = queryParam(url, PROCESS_ID_QUERY_PARAM).orElse(request.identifierId());
    return new SessionLink(url, processId);
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
    return execute(path, () -> transport.get(baseUrl + path, headers));
  }

  private JsonNode post(String path, Map<String, String> headers, JsonNode body)
      throws IOException {
    String payload = MAPPER.writeValueAsString(body);
    return execute(path, () -> transport.postJson(baseUrl + path, headers, payload));
  }

  @FunctionalInterface
  private interface Request {
    HttpResult send() throws IOException;
  }

  /** Sends a request, retrying with exponential backoff on network and server errors. */
  private JsonNode execute(String path, Request request) throws IOException {
    IOException lastError = null;
    for (int attempt = 0; attempt < maxRetries; attempt++) {
      if (attempt > 0) {
        backoff(attempt);
      }
      HttpResult result;
      try {
        result = request.send();
      } catch (IOException e) {
        log.warnv("{0}: attempt {1} failed: {2}", path, attempt + 1, e.getMessage());
        lastError = e;
        continue;
      }
      if (result.status() >= 500) {
        log.warnv("{0}: attempt {1} got status {2}", path, attempt + 1, result.status());
        lastError = new IOException(path + " returned status " + result.status());
        continue;
      }
      if (result.status() != 200) {
        throw new IOException(
            String.format("%s returned status %d: %.200s", path, result.status(), result.body()));
      }
      try {
        return MAPPER.readTree(result.body());
      } catch (IOException e) {
        throw new IOException(path + " returned an invalid JSON body", e);
      }
    }
    throw new IOException(path + " failed after " + maxRetries + " attempts", lastError);
  }

  private void backoff(int attempt) throws IOException {
    try {
      sleeper.sleep(BASE_RETRY_DELAY_MS * (1L << (attempt - 1)));
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IOException("Interrupted while waiting to retry", e);
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
