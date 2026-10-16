// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.representations.IDToken;

class AuthorizedElectionsUserAttributeMapperTest {

  @Test
  void putElectionEventIdClaim_addsHasuraElectionEventId() {
    IDToken token = new IDToken();

    AuthorizedElectionsUserAttributeMapper.putElectionEventIdClaim(token, "150017");

    Map<?, ?> hasuraClaims =
        (Map<?, ?>)
            token.getOtherClaims().get(AuthorizedElectionsUserAttributeMapper.HASURA_CLAIMS);
    assertEquals("150017", hasuraClaims.get("x-hasura-election-event-id"));
  }

  @Test
  void putElectionEventIdClaim_preservesExistingHasuraClaims() {
    IDToken token = new IDToken();
    Map<String, Object> existingClaims = new HashMap<>();
    existingClaims.put("authorized-election-ids", List.of("election-1"));
    token
        .getOtherClaims()
        .put(AuthorizedElectionsUserAttributeMapper.HASURA_CLAIMS, existingClaims);

    AuthorizedElectionsUserAttributeMapper.putElectionEventIdClaim(token, "150017");

    Map<?, ?> hasuraClaims =
        (Map<?, ?>)
            token.getOtherClaims().get(AuthorizedElectionsUserAttributeMapper.HASURA_CLAIMS);
    assertEquals(List.of("election-1"), hasuraClaims.get("authorized-election-ids"));
    assertEquals("150017", hasuraClaims.get("x-hasura-election-event-id"));
  }

  @Test
  void buildTokenRequestForm_encodesReservedCharactersInValues() {
    Map<String, String> data = new LinkedHashMap<>();
    data.put("client_id", "service-account");
    data.put("client_secret", "a&b=c+d e");

    String form = AuthorizedElectionsUserAttributeMapper.buildTokenRequestForm(data);

    assertEquals("client_id=service-account&client_secret=a%26b%3Dc%2Bd+e", form);
    Map<String, String> decoded = new HashMap<>();
    Arrays.stream(form.split("&"))
        .map(pair -> pair.split("=", 2))
        .forEach(
            kv ->
                decoded.put(
                    URLDecoder.decode(kv[0], StandardCharsets.UTF_8),
                    URLDecoder.decode(kv[1], StandardCharsets.UTF_8)));
    assertEquals(data, decoded);
  }

  @Test
  void parseAccessToken_returnsTokenFromValidResponse() {
    assertEquals(
        "abc.def.ghi",
        AuthorizedElectionsUserAttributeMapper.parseAccessToken(
                "{\"access_token\":\"abc.def.ghi\",\"token_type\":\"Bearer\"}")
            .orElseThrow());
  }

  @Test
  void parseAccessToken_isEmptyForErrorResponse() {
    assertTrue(
        AuthorizedElectionsUserAttributeMapper.parseAccessToken(
                "{\"error\":\"unauthorized_client\",\"error_description\":\"Invalid client\"}")
            .isEmpty());
  }

  @Test
  void parseAccessToken_isEmptyForInvalidJson() {
    assertTrue(
        AuthorizedElectionsUserAttributeMapper.parseAccessToken("<html>502</html>").isEmpty());
  }

  @Test
  void parseAccessToken_isEmptyForEmptyBody() {
    assertTrue(AuthorizedElectionsUserAttributeMapper.parseAccessToken("").isEmpty());
    assertTrue(AuthorizedElectionsUserAttributeMapper.parseAccessToken(null).isEmpty());
  }

  @Test
  void parseAccessToken_isEmptyForNonStringOrBlankToken() {
    assertTrue(
        AuthorizedElectionsUserAttributeMapper.parseAccessToken("{\"access_token\":null}")
            .isEmpty());
    assertTrue(
        AuthorizedElectionsUserAttributeMapper.parseAccessToken("{\"access_token\":\"\"}")
            .isEmpty());
    assertTrue(
        AuthorizedElectionsUserAttributeMapper.parseAccessToken("{\"access_token\":\"   \"}")
            .isEmpty());
    assertTrue(AuthorizedElectionsUserAttributeMapper.parseAccessToken("[]").isEmpty());
  }
}
