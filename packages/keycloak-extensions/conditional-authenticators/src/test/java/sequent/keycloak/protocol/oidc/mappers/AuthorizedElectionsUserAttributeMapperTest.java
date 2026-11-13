// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;

import java.util.HashMap;
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

  private static final AuthorizedElectionsUserAttributeMapper.AreaElections TREE =
      new AuthorizedElectionsUserAttributeMapper.AreaElections(
          Map.of(
              "country", List.of("national"),
              "region", List.of("regional"),
              "city", List.of("municipal"),
              "other-city", List.of("other-municipal")),
          Map.of("region", "country", "city", "region", "other-city", "region"));

  @Test
  void areaIdAttribute_isThePlatformAttributeName() {
    assertEquals("area-id", AuthorizedElectionsUserAttributeMapper.AREA_ID_ATTRIBUTE);
  }

  @Test
  void electionsForArea_includesTheElectionsOfParentAreas() {
    assertEquals(
        List.of("municipal", "regional", "national"),
        AuthorizedElectionsUserAttributeMapper.electionsForArea("city", TREE));
  }

  @Test
  void electionsForArea_excludesSiblingAreas() {
    assertEquals(
        List.of("other-municipal", "regional", "national"),
        AuthorizedElectionsUserAttributeMapper.electionsForArea("other-city", TREE));
  }

  @Test
  void electionsForArea_isEmptyForAnUnknownArea() {
    assertEquals(
        List.of(), AuthorizedElectionsUserAttributeMapper.electionsForArea("unknown", TREE));
  }

  @Test
  void electionsForArea_stopsOnACycle() {
    var cyclic =
        new AuthorizedElectionsUserAttributeMapper.AreaElections(
            Map.of("a", List.of("e1"), "b", List.of("e2")), Map.of("a", "b", "b", "a"));
    assertEquals(
        List.of("e1", "e2"), AuthorizedElectionsUserAttributeMapper.electionsForArea("a", cyclic));
  }

  @Test
  void parseAreaElections_readsElectionsAndParents() throws IOException {
    var parsed =
        AuthorizedElectionsUserAttributeMapper.parseAreaElections(
            new ObjectMapper()
                .readTree(
                    """
                    {"data": {
                      "sequent_backend_area_contest": [
                        {"area_id": "city", "contest": {"election_id": "municipal"}}
                      ],
                      "sequent_backend_area": [
                        {"id": "city", "parent_id": "region"},
                        {"id": "region", "parent_id": null}
                      ]}}
                    """));
    assertEquals(Map.of("city", List.of("municipal")), parsed.electionsByArea());
    assertEquals(Map.of("city", "region"), parsed.parentByArea());
  }

  @Test
  void parseAreaElections_failsOnGraphqlErrors() {
    assertThrows(
        IOException.class,
        () ->
            AuthorizedElectionsUserAttributeMapper.parseAreaElections(
                new ObjectMapper().readTree("{\"errors\": [{\"message\": \"denied\"}]}")));
  }
}
