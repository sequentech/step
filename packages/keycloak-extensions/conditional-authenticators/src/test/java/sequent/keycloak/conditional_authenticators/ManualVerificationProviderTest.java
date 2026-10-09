// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.BadRequestException;
import jakarta.ws.rs.ForbiddenException;
import jakarta.ws.rs.NotAuthorizedException;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MultivaluedHashMap;
import java.net.URI;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.ClientModel;
import org.keycloak.models.ClientProvider;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserProvider;
import org.keycloak.services.resources.admin.fgap.AdminPermissionEvaluator;
import org.keycloak.services.resources.admin.fgap.UserPermissionEvaluator;

class ManualVerificationProviderTest {
  private KeycloakSession session;
  private KeycloakContext context;
  private RealmModel realm;
  private UserProvider users;
  private ClientModel client;

  @BeforeEach
  void setUp() {
    session = mock(KeycloakSession.class);
    context = mock(KeycloakContext.class);
    realm = mock(RealmModel.class);
    users = mock(UserProvider.class);
    client = mock(ClientModel.class);
    ClientProvider clients = mock(ClientProvider.class);
    HttpHeaders headers = mock(HttpHeaders.class);
    KeycloakUriInfo uri = mock(KeycloakUriInfo.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("target-event");
    when(context.getRequestHeaders()).thenReturn(headers);
    when(headers.getRequestHeaders()).thenReturn(new MultivaluedHashMap<>());
    when(session.users()).thenReturn(users);
    when(session.clients()).thenReturn(clients);
    when(clients.getClientByClientId(realm, "voting-portal")).thenReturn(client);
    when(client.isEnabled()).thenReturn(true);
    when(client.getRedirectUris()).thenReturn(Set.of("https://vote.example/*"));
    when(context.getUri()).thenReturn(uri);
    when(uri.getBaseUri()).thenReturn(URI.create("https://id.example/"));
    when(uri.getBaseUriBuilder())
        .thenAnswer(unused -> jakarta.ws.rs.core.UriBuilder.fromUri("https://id.example/"));
  }

  @Test
  void anonymousRequestCannotLookUpVoters() {
    assertThrows(
        NotAuthorizedException.class,
        () ->
            new ManualVerificationProvider(session)
                .generateLink("voter", "https://vote.example/login"));
    verifyNoInteractions(users);
    verify(context).setRealm(realm);
  }

  @Test
  void voterWithoutManageUsersCannotLookUpVoters() {
    assertThrows(
        ForbiddenException.class,
        () -> authorized(false).generateLink("voter", "https://vote.example/login"));
    verifyNoInteractions(users);
  }

  @Test
  void authorizedAdminRejectsExternalRedirectBeforeLookingUpVoter() {
    assertThrows(
        BadRequestException.class,
        () -> authorized(true).generateLink("voter", "https://attacker.example/login"));
    verifyNoInteractions(users);
  }

  @Test
  void authorizedAdminRejectsJavascriptRedirectBeforeLookingUpVoter() {
    assertThrows(
        BadRequestException.class,
        () -> authorized(true).generateLink("voter", "javascript:alert(1)"));
    verifyNoInteractions(users);
  }

  @Test
  void authorizedAdminPreservesNotFoundForValidVotingPortalRedirect() {
    assertEquals(
        404,
        authorized(true)
            .generateLink("missing", "https://vote.example/tenant/t/event/e/login")
            .getStatus());
    verify(users).getUserById(realm, "missing");
    verify(context).setRealm(realm);
  }

  @Test
  void authorizedAdminPreservesOptionalRedirect() {
    assertEquals(404, authorized(true).generateLink("missing", null).getStatus());
    verify(users).getUserById(realm, "missing");
  }

  @Test
  void authorizedAdminIssuesClientBoundTokenForTargetRealm() {
    org.keycloak.models.UserModel voter = mock(org.keycloak.models.UserModel.class);
    org.keycloak.models.TokenManager tokens = mock(org.keycloak.models.TokenManager.class);
    when(users.getUserById(realm, "voter")).thenReturn(voter);
    when(voter.getId()).thenReturn("voter");
    when(realm.getName()).thenReturn("target-event");
    when(client.getClientId()).thenReturn("voting-portal");
    when(session.tokens()).thenReturn(tokens);
    when(tokens.encode(any())).thenReturn("signed-action-token");
    var response = authorized(true).generateLink("voter", "https://vote.example/login");
    assertEquals(200, response.getStatus());
    var captured = org.mockito.ArgumentCaptor.forClass(ManualVerificationToken.class);
    verify(tokens).encode(captured.capture());
    assertEquals("voting-portal", captured.getValue().getIssuedFor());
    assertEquals("https://vote.example/login", captured.getValue().getRedirectUri());
    assertEquals("https://id.example/realms/target-event", captured.getValue().getIssuer());
    assertTrue(response.getEntity().toString().contains("/realms/target-event/"));
  }

  private ManualVerificationProvider authorized(boolean canManage) {
    return new ManualVerificationProvider(session) {
      @Override
      public void setup() {
        permissions = mock(AdminPermissionEvaluator.class);
        UserPermissionEvaluator userPermissions = mock(UserPermissionEvaluator.class);
        when(permissions.users()).thenReturn(userPermissions);
        when(userPermissions.canManage()).thenReturn(canManage);
      }
    };
  }
}
