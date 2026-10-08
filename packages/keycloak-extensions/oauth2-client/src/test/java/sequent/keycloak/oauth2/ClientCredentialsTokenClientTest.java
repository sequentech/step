// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.oauth2;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.URLDecoder;
import java.net.http.HttpRequest;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class ClientCredentialsTokenClientTest {

  @Test
  void formUrlEncodeSurvivesReservedFormCharacters() {
    // Regression test for meta#12526: a secret containing '%' broke
    // application/x-www-form-urlencoded decoding on the token request.
    Map<Object, Object> data = new HashMap<>();
    data.put("client_id", "service-account");
    data.put("client_secret", "ab%12&cd+ef=gh ij");
    data.put("grant_type", "client_credentials");

    String form = ClientCredentialsTokenClient.formUrlEncode(data);

    Map<String, String> decoded = new HashMap<>();
    for (String pair : form.split("&")) {
      String[] kv = pair.split("=", 2);
      decoded.put(
          URLDecoder.decode(kv[0], StandardCharsets.UTF_8),
          URLDecoder.decode(kv[1], StandardCharsets.UTF_8));
    }

    assertEquals("service-account", decoded.get("client_id"));
    assertEquals("ab%12&cd+ef=gh ij", decoded.get("client_secret"));
    assertEquals("client_credentials", decoded.get("grant_type"));
  }

  @Test
  void extractAccessTokenReturnsTokenOnSuccess() throws IOException {
    String responseBody = "{\"access_token\":\"the-token\",\"token_type\":\"Bearer\"}";

    String token = ClientCredentialsTokenClient.extractAccessToken(responseBody);

    assertEquals("the-token", token);
  }

  @Test
  void extractAccessTokenThrowsClearErrorWhenTokenMissing() {
    // This is the exact failure mode from the incident: Keycloak's token
    // endpoint rejects a malformed request and returns an error body with no
    // access_token. Previously this caused a NullPointerException.
    String responseBody = "{\"error\":\"invalid_client\",\"error_description\":\"bad secret\"}";

    IllegalStateException ex =
        assertThrows(
            IllegalStateException.class,
            () -> ClientCredentialsTokenClient.extractAccessToken(responseBody));
    assertTrue(ex.getMessage().contains("invalid_client"));
  }

  @Test
  void tokenRequestCarriesABoundedTimeout() {
    // Without a timeout the calling Keycloak request thread waits indefinitely
    // when the token endpoint accepts the connection but never responds.
    HttpRequest request =
        ClientCredentialsTokenClient.buildTokenRequest(
            "https://keycloak.example/realms/tenant/protocol/openid-connect/token",
            new HashMap<>());

    assertTrue(request.timeout().isPresent());
    assertTrue(request.timeout().get().toMillis() > 0);
  }

  @Test
  void extractAccessTokenThrowsOnMalformedJson() {
    assertThrows(
        IOException.class, () -> ClientCredentialsTokenClient.extractAccessToken("not json"));
  }
}
