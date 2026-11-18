// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.doReturn;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.ProtocolMapperModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.representations.IDToken;

class AuthorizedElectionsUserAttributeMapperTest {

  private static final String AUTHORIZED_ELECTIONS = "authorized-election-ids";
  private static final String TENANT_ID = "tenant-id";
  private static final String EVENT_ID = "event-id";
  private static final Map<String, String> ELECTIONS =
      Map.of("external-1", "election-1", "external-2", "election-2", "external-3", "election-3");

  private AuthorizedElectionsUserAttributeMapper mapper;
  private ProtocolMapperModel mappingModel;
  private UserSessionModel userSession;
  private final Map<String, List<String>> attributes = new HashMap<>();

  @BeforeEach
  void setUp() throws Exception {
    mapper = spy(new AuthorizedElectionsUserAttributeMapper());
    doReturn(ELECTIONS).when(mapper).getAllElectionsFromElectionEvent(EVENT_ID, TENANT_ID);
    doReturn(Map.of("area-1", List.of("election-1", "election-2")))
        .when(mapper)
        .getElectionsByArea(EVENT_ID, TENANT_ID);

    UserModel user = mock(UserModel.class);
    when(user.getAttributeStream(anyString()))
        .thenAnswer(
            invocation -> attributes.getOrDefault(invocation.getArgument(0), List.of()).stream());
    RealmModel realm = mock(RealmModel.class);
    when(realm.getName()).thenReturn("tenant-" + TENANT_ID + "-event-" + EVENT_ID);
    userSession = mock(UserSessionModel.class);
    when(userSession.getUser()).thenReturn(user);
    when(userSession.getRealm()).thenReturn(realm);

    mappingModel =
        AuthorizedElectionsUserAttributeMapper.createClaimMapper(
            "Authorized elections",
            AUTHORIZED_ELECTIONS,
            AUTHORIZED_ELECTIONS,
            "String",
            true,
            true,
            true,
            true);
    mappingModel.setId("authorized-elections-mapper");
    mappingModel.getConfig().put("use.array.attrs", "true");
  }

  @Test
  void setClaim_usesCanonicalAreaAttributeWhenExplicitAssignmentsAreAbsent() throws Exception {
    attributes.put("area-id", List.of("area-1"));
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertEquals(
        List.of("election-1", "election-2"), token.getOtherClaims().get(AUTHORIZED_ELECTIONS));
    assertEquals(
        EVENT_ID,
        ((Map<?, ?>)
                token.getOtherClaims().get(AuthorizedElectionsUserAttributeMapper.HASURA_CLAIMS))
            .get(AuthorizedElectionsUserAttributeMapper.HASURA_ELECTION_EVENT_ID));
  }

  @Test
  void setClaim_formatsAreaElectionsAsPostgresArrayWhenConfigured() throws Exception {
    attributes.put("area-id", List.of("area-1"));
    mappingModel.getConfig().put("use.array.attrs", "false");
    mappingModel.getConfig().put("multivalued", "false");
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertEquals(
        "{\"election-1\", \"election-2\"}", token.getOtherClaims().get(AUTHORIZED_ELECTIONS));
  }

  @Test
  void setClaim_explicitExternalIdsTakePrecedenceOverArea() throws Exception {
    attributes.put("area-id", List.of("area-1"));
    attributes.put(AUTHORIZED_ELECTIONS, List.of("external-3", "unknown-external-id"));
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertEquals(List.of("election-3"), token.getOtherClaims().get(AUTHORIZED_ELECTIONS));
    verify(mapper, never()).getElectionsByArea(anyString(), anyString());
  }

  @Test
  void setClaim_preservesAllElectionsFallbackWhenAreaIsAbsent() throws Exception {
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertAllElections(token);
    verify(mapper, never()).getElectionsByArea(anyString(), anyString());
  }

  @Test
  void setClaim_preservesAllElectionsFallbackWhenAreaIsUnknown() throws Exception {
    attributes.put("area-id", List.of("unknown-area"));
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertAllElections(token);
    verify(mapper).getElectionsByArea(EVENT_ID, TENANT_ID);
  }

  @Test
  void setClaim_preservesAllElectionsFallbackWhenAreaLookupFails() throws Exception {
    attributes.put("area-id", List.of("area-1"));
    doThrow(new IOException("Synthetic lookup failure"))
        .when(mapper)
        .getElectionsByArea(EVENT_ID, TENANT_ID);
    IDToken token = new IDToken();

    mapper.setClaim(token, mappingModel, userSession);

    assertAllElections(token);
    verify(mapper).getElectionsByArea(EVENT_ID, TENANT_ID);
  }

  private static void assertAllElections(IDToken token) {
    assertEquals(
        Set.of("election-1", "election-2", "election-3"),
        Set.copyOf((List<?>) token.getOtherClaims().get(AUTHORIZED_ELECTIONS)));
  }

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
}
