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

  /** Reads {@code elections} as the mapper reads the elections Hasura returns. */
  private static AuthorizedElectionsUserAttributeMapper.Elections elections(String elections)
      throws Exception {
    return AuthorizedElectionsUserAttributeMapper.elections(new ObjectMapper().readTree(elections));
  }

  /** The election ID that each value names among {@code elections}. */
  private static Map<String, String> electionIdsByKey(String elections) throws Exception {
    return elections(elections).idsByKey();
  }

  /** Voter imports used to store IDs. An empty external ID counts as none. */
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

  /** A shared external ID cannot tell its elections apart. */
  @Test
  void electionIdsByKey_keysElectionsWhoseExternalIdIsRepeatedOnlyById() throws Exception {
    assertEquals(
        Map.of(
            "election-1", "election-1",
            "election-2", "election-2",
            "election-3", "election-3"),
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "GIAMBI30-3-31"},
             {"id": "election-2", "external_id": "GIAMBI30-3-31"},
             {"id": "election-3", "external_id": "GIAMBI30-3-31"}]
            """));
  }

  /** The value is looked up among external IDs first, where it names several elections. */
  @Test
  void electionIdsByKey_doesNotKeyAnElectionByAnIdThatOthersShareAsExternalId() throws Exception {
    assertEquals(
        Map.of(
            "election-1", "election-1",
            "election-2", "election-2"),
        electionIdsByKey(
            """
            [{"id": "election-1", "external_id": "election-3"},
             {"id": "election-2", "external_id": "election-3"},
             {"id": "election-3", "external_id": null}]
            """));
  }

  /** As the voters import and the tally census do. */
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

  /** Whatever the order Hasura returns the elections in. */
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

  /** Users without authorized elections may vote in every election, even one no value names. */
  @Test
  void elections_keepsEveryElectionId() throws Exception {
    assertEquals(
        List.of("election-1", "election-2"),
        elections(
                """
                [{"id": "election-1", "external_id": "election-2"},
                 {"id": "election-2", "external_id": null}]
                """)
            .ids());
  }

  /** Areas name their elections by ID, even one that is another election's external ID. */
  @Test
  void idsAmong_keepsTheElectionsOfTheEventWithoutRepeats() throws Exception {
    AuthorizedElectionsUserAttributeMapper.Elections elections =
        elections(
            """
            [{"id": "election-1", "external_id": "election-2"},
             {"id": "election-2", "external_id": null}]
            """);

    assertEquals(
        List.of("election-2", "election-1"),
        elections.idsAmong(List.of("election-2", "election-3", "election-2", "election-1")));
  }

  /** The user keeps the elections their other values name. */
  @Test
  void toElectionIds_dropsAnExternalIdThatSeveralElectionsShare() throws Exception {
    assertEquals(
        List.of("election-1"),
        AuthorizedElectionsUserAttributeMapper.toElectionIds(
            List.of("GIAMBI30-3-31", "election-1"),
            electionIdsByKey(
                """
                [{"id": "election-1", "external_id": "GIAMBI30-3-31"},
                 {"id": "election-2", "external_id": "GIAMBI30-3-31"}]
                """)));
  }

  /** Such as the values the voters export writes in double quotes. */
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
