// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.oauth2;

import java.io.IOException;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import java.util.stream.Collectors;
import org.keycloak.util.JsonSerialization;

/**
 * Shared client for the OAuth2 client_credentials grant against a Keycloak realm's token
 * endpoint. Extracted so the request-encoding and response-parsing logic exists in exactly one
 * place; see meta#1252 for the incident (an unencoded client secret containing '%' broke
 * application/x-www-form-urlencoded decoding) that motivated pulling this out of two separately
 * copy-pasted implementations.
 */
public final class ClientCredentialsTokenClient {

  private ClientCredentialsTokenClient() {}

  public static String requestAccessToken(
      HttpClient httpClient, String tokenUrl, String clientId, String clientSecret)
      throws IOException {
    Map<Object, Object> data = new HashMap<>();
    data.put("client_id", clientId);
    data.put("scope", "openid");
    data.put("client_secret", clientSecret);
    data.put("grant_type", "client_credentials");

    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(tokenUrl))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .POST(HttpRequest.BodyPublishers.ofString(formUrlEncode(data)))
            .build();

    String responseBody =
        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString()).join().body();
    return extractAccessToken(responseBody);
  }

  // Percent-encodes each key/value pair so that secrets containing reserved
  // form-encoding characters (%, &, +, =, space) survive as a valid
  // application/x-www-form-urlencoded body.
  static String formUrlEncode(Map<Object, Object> data) {
    return data.entrySet().stream()
        .map(entry -> encode(entry.getKey()) + "=" + encode(entry.getValue()))
        .collect(Collectors.joining("&"));
  }

  private static String encode(Object value) {
    return URLEncoder.encode(String.valueOf(value), StandardCharsets.UTF_8);
  }

  // Fails loudly instead of NPE-ing on ".toString()" or silently returning the
  // raw error body as if it were a token.
  static String extractAccessToken(String responseBody) throws IOException {
    Object accessToken = JsonSerialization.readValue(responseBody, Map.class).get("access_token");
    if (accessToken == null) {
      throw new IllegalStateException(
          "Keycloak client_credentials response did not include an access_token: "
              + responseBody);
    }
    return accessToken.toString();
  }
}
