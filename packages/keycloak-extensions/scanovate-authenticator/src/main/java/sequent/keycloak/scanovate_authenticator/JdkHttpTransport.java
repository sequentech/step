// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.Map;

/** {@link HttpTransport} backed by the JDK HTTP client. */
public class JdkHttpTransport implements HttpTransport {
  static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(10);
  static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(30);

  private static final HttpClient CLIENT =
      HttpClient.newBuilder()
          .connectTimeout(CONNECT_TIMEOUT)
          .followRedirects(HttpClient.Redirect.NEVER)
          .build();

  @Override
  public HttpResult get(String url, Map<String, String> headers) throws IOException {
    return send(request(url, headers).GET().build());
  }

  @Override
  public HttpResult postJson(String url, Map<String, String> headers, String body)
      throws IOException {
    return send(
        request(url, headers)
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(body))
            .build());
  }

  private static HttpRequest.Builder request(String url, Map<String, String> headers) {
    HttpRequest.Builder builder =
        HttpRequest.newBuilder(URI.create(url))
            .timeout(REQUEST_TIMEOUT)
            .header("Accept", "application/json");
    headers.forEach(builder::header);
    return builder;
  }

  private static HttpResult send(HttpRequest request) throws IOException {
    try {
      HttpResponse<String> response = CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
      return new HttpResult(response.statusCode(), response.body());
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IOException("Interrupted while calling " + request.uri(), e);
    } catch (IllegalArgumentException e) {
      throw new IOException("Invalid request to " + request.uri(), e);
    }
  }
}
