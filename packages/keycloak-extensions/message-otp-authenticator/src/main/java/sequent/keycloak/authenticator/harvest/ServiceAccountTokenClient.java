// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.harvest;

import java.io.IOException;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.stream.Collectors;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.util.JsonSerialization;

/**
 * Obtains a service-account access token with the client-credentials grant, so extensions can call
 * harvest and Hasura. Neither the client secret nor the tokens are ever logged.
 */
@JBossLog
public class ServiceAccountTokenClient {
  private static final String CLIENT_ID_ENV = "KEYCLOAK_CLIENT_ID";
  private static final String CLIENT_SECRET_ENV = "KEYCLOAK_CLIENT_SECRET";
  private static final String KEYCLOAK_URL_ENV = "KEYCLOAK_URL";

  private final HttpClient httpClient = HttpClient.newHttpClient();
  private final String keycloakUrl;
  private final String clientId;
  private final String clientSecret;

  public ServiceAccountTokenClient(String keycloakUrl, String clientId, String clientSecret) {
    this.keycloakUrl = keycloakUrl;
    this.clientId = clientId;
    this.clientSecret = clientSecret;
  }

  public static ServiceAccountTokenClient fromEnvironment() {
    return new ServiceAccountTokenClient(
        System.getenv(KEYCLOAK_URL_ENV),
        System.getenv(CLIENT_ID_ENV),
        System.getenv(CLIENT_SECRET_ENV));
  }

  public static String tenantRealmName(String tenantId) {
    return "tenant-" + tenantId;
  }

  /** Returns an access token issued by the given realm. */
  public String fetchAccessToken(String realmName) throws IOException {
    Map<String, String> form = new LinkedHashMap<>();
    form.put("client_id", clientId);
    form.put("client_secret", clientSecret);
    form.put("grant_type", "client_credentials");
    form.put("scope", "openid");
    String body =
        form.entrySet().stream()
            .map(entry -> encode(entry.getKey()) + "=" + encode(entry.getValue()))
            .collect(Collectors.joining("&"));
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(
                URI.create(keycloakUrl + "/realms/" + realmName + "/protocol/openid-connect/token"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .POST(HttpRequest.BodyPublishers.ofString(body))
            .build();

    HttpResponse<String> response;
    try {
      response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IOException("Interrupted while requesting a service-account token", e);
    }
    if (response.statusCode() != 200) {
      log.errorv(
          "Service-account token request to realm {0} failed with status {1}",
          realmName, response.statusCode());
      throw new IOException(
          "Service-account token request failed with status " + response.statusCode());
    }
    Object accessToken =
        JsonSerialization.readValue(response.body(), Map.class).get("access_token");
    if (!(accessToken instanceof String token) || token.isBlank()) {
      throw new IOException("Service-account token response has no access_token");
    }
    return token;
  }

  private static String encode(String value) {
    return URLEncoder.encode(value == null ? "" : value, StandardCharsets.UTF_8);
  }
}
