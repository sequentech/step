// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.time.Duration;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;

/** {@link HttpTransport} backed by the JDK HTTP client. */
public class JdkHttpTransport implements HttpTransport {
  static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(10);
  static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(30);
  static final String BOUNDARY_PREFIX = "scanovate-";
  private static final String CRLF = "\r\n";
  private static final SecureRandom RANDOM = new SecureRandom();

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

  @Override
  public HttpResult postMultipart(
      String url, Map<String, String> headers, List<MultipartPart> parts) throws IOException {
    String boundary = newBoundary();
    byte[] body;
    try {
      body = multipartBody(parts, boundary);
    } catch (IllegalArgumentException e) {
      throw new IOException("Invalid multipart request to " + url, e);
    }
    return send(
        request(url, headers)
            .header("Content-Type", "multipart/form-data; boundary=" + boundary)
            .POST(HttpRequest.BodyPublishers.ofByteArray(body))
            .build());
  }

  /** A random boundary, which in practice never appears inside the parts. */
  static String newBoundary() {
    byte[] random = new byte[24];
    RANDOM.nextBytes(random);
    return BOUNDARY_PREFIX + HexFormat.of().formatHex(random);
  }

  /** Encodes the parts as a {@code multipart/form-data} body (RFC 7578). */
  static byte[] multipartBody(List<MultipartPart> parts, String boundary) {
    ByteArrayOutputStream body = new ByteArrayOutputStream();
    for (MultipartPart part : parts) {
      writeAscii(
          body,
          "--"
              + boundary
              + CRLF
              + "Content-Disposition: form-data; name=\""
              + headerValue(part.name())
              + "\"; filename=\""
              + headerValue(part.filename())
              + "\""
              + CRLF
              + "Content-Type: "
              + headerValue(part.contentType())
              + CRLF
              + CRLF);
      body.writeBytes(part.content());
      writeAscii(body, CRLF);
    }
    writeAscii(body, "--" + boundary + "--" + CRLF);
    return body.toByteArray();
  }

  private static String headerValue(String value) {
    if (value.chars().anyMatch(c -> c == '"' || c == '\r' || c == '\n' || c > 0x7E)) {
      throw new IllegalArgumentException("Invalid multipart header value " + value);
    }
    return value;
  }

  private static void writeAscii(ByteArrayOutputStream output, String value) {
    output.writeBytes(value.getBytes(StandardCharsets.US_ASCII));
  }

  private static HttpRequest.Builder request(String url, Map<String, String> headers)
      throws IOException {
    try {
      HttpRequest.Builder builder =
          HttpRequest.newBuilder(URI.create(url))
              .timeout(REQUEST_TIMEOUT)
              .header("Accept", "application/json");
      headers.forEach(builder::header);
      return builder;
    } catch (IllegalArgumentException e) {
      throw new IOException("Invalid B-Trust URL " + url, e);
    }
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
