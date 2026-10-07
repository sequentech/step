// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.ObjectMapper;
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

  private static Map<String, String> electionIdsByKey(String elections) throws Exception {
    return AuthorizedElectionsUserAttributeMapper.electionIdsByKey(
        new ObjectMapper().readTree(elections));
  }

  @Test
  void electionIdsByKey_keysElectionsByExternalIdOrIdWithoutOne_andById() throws Exception {
    assertEquals(
        Map.of(
            "GIAMBI30-3-31", "election-1",
            "election-1", "election-1",
            "election-2", "election-2",
            "election-3", "election-3"),
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "GIAMBI30-3-31"},
             {"id": "election-2", "external_id": null},
             {"id": "election-3", "external_id": ""}]
            """));
  }

  @Test
  void electionIdsByKey_keepsTheIdOfAnElectionWhoseExternalIdIsRepeated() throws Exception {
    Map<String, String> electionIds =
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "GIAMBI30-3-31"},
             {"id": "election-2", "external_id": "GIAMBI30-3-31"}]
            """);

    assertEquals("election-1", electionIds.get("election-1"));
    assertEquals("election-2", electionIds.get("election-2"));
  }

  @Test
  void electionIdsByKey_givesAnExternalIdPrecedenceOverAnEqualId() throws Exception {
    assertEquals(
        Map.of(
            "election-2", "election-1",
            "election-1", "election-1",
            "GIAMBI30-3-31", "election-2"),
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "election-2"},
             {"id": "election-2", "external_id": "GIAMBI30-3-31"}]
            """));
  }

  @Test
  void electionIdsByKey_givesAnExternalIdPrecedenceOverTheIdOfAnElectionWithoutOne()
      throws Exception {
    Map<String, String> expected =
        Map.of(
            "election-2", "election-1",
            "election-1", "election-1");

    assertEquals(
        expected,
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "election-2"},
             {"id": "election-2", "external_id": null}]
            """));
    assertEquals(
        expected,
        electionIdsByKey(
            """
            [{"id": "election-2", "external_id": null},
             {"id": "election-1", "external_id": "election-2"}]
            """));
  }

  @Test
  void toElectionIds_dropsValuesThatNameNoElectionAndRepeats() {
    assertEquals(
        List.of("election-1", "election-2"),
        AuthorizedElectionsUserAttributeMapper.toElectionIds(
            List.of("unknown", "GIAMBI30-3-31", "election-1", "", "election-2"),
            Map.of(
                "GIAMBI30-3-31", "election-1",
                "election-1", "election-1",
                "election-2", "election-2")));
  }
}
