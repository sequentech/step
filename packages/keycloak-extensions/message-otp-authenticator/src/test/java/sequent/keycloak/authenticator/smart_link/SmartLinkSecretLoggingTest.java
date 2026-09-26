// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.services.resources.admin.fgap.AdminPermissionEvaluator;
import org.mockito.MockedStatic;
import sequent.keycloak.authenticator.CapturedLogs;
import sequent.keycloak.login_bridge.LoginBridge;
import sequent.keycloak.login_bridge.LoginBridgeActionToken;

class SmartLinkSecretLoggingTest {
  private static final String LINK =
      "https://auth.example/realms/test/login-actions/action-token?key=synthetic-smart-link-token&client_id=portal";
  private final KeycloakSession session = mock(KeycloakSession.class, RETURNS_DEEP_STUBS);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserModel user = mock(UserModel.class);

  @Test
  void serializedLoginLinkIsReturnedWithoutLoggingItsToken() {
    when(session.getContext().getUri().getBaseUri())
        .thenReturn(URI.create("https://auth.example/"));
    when(realm.getName()).thenReturn("test");
    LoginBridgeActionToken token = mock(LoginBridgeActionToken.class);
    when(token.serialize(eq(session), eq(realm), any())).thenReturn("synthetic-smart-link-token");
    when(token.getIssuedFor()).thenReturn("portal");
    RealmModel originalRealm = session.getContext().getRealm();
    try (CapturedLogs logs = new CapturedLogs(LoginBridge.class)) {
      assertEquals(LINK, LoginBridge.linkFromActionToken(session, realm, token));
      assertFalse(logs.text().contains("synthetic-smart-link-token"));
    }
    verify(session.getContext()).setRealm(originalRealm);
  }

  @Test
  void adminEndpointReturnsAndDeliversTheLinkWithoutLoggingIt() {
    when(session.getContext().getRealm()).thenReturn(realm);
    ClientModel client = mock(ClientModel.class);
    when(session.clients().getClientByClientId(realm, "portal")).thenReturn(client);
    when(user.getId()).thenReturn("synthetic-user");
    SmartLinkResource resource = new SmartLinkResource(session);
    resource.permissions = mock(AdminPermissionEvaluator.class, RETURNS_DEEP_STUBS);
    when(resource.permissions.users().canManage()).thenReturn(true);
    resource.event = mock(EventBuilder.class, RETURNS_SELF);
    SmartLinkRequest request = new SmartLinkRequest();
    request.setClientId("portal");
    request.setRedirectUri("https://portal.example/callback");
    request.setEmailOrUsername("voter@example.com");
    request.setSendNotification(true);
    try (CapturedLogs logs = new CapturedLogs(SmartLinkResource.class);
        MockedStatic<LoginBridge> bridge = mockStatic(LoginBridge.class);
        MockedStatic<SmartLink> smartLink = mockStatic(SmartLink.class)) {
      bridge
          .when(() -> LoginBridge.validateRedirectUri(session, request.getRedirectUri(), client))
          .thenReturn(true);
      bridge
          .when(() -> LoginBridge.linkFromActionToken(eq(session), eq(realm), any()))
          .thenReturn(LINK);
      smartLink
          .when(
              () ->
                  SmartLink.getOrCreate(
                      eq(session),
                      eq(realm),
                      eq("voter@example.com"),
                      eq(false),
                      eq(false),
                      eq(false),
                      any()))
          .thenReturn(user);
      smartLink
          .when(() -> SmartLink.sendSmartLinkNotification(session, user, LINK))
          .thenReturn(true);
      SmartLinkResponse response = resource.createSmartLink(request);
      assertEquals(LINK, response.getLink());
      assertTrue(response.isSent());
      smartLink.verify(() -> SmartLink.sendSmartLinkNotification(session, user, LINK));
      assertFalse(logs.text().contains("synthetic-smart-link-token"));
    }
  }

  private AuthenticationFlowContext context() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class, RETURNS_DEEP_STUBS);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticatorConfig()).thenReturn(null);
    when(session.getContext().getClient().getClientId()).thenReturn("portal");
    MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();
    parameters.putSingle("username", "voter@example.com");
    when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(parameters);
    LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
    when(context.form()).thenReturn(form);
    return context;
  }

  @Test
  void browserFlowDeliversLinkWithoutLoggingIt() {
    AuthenticationFlowContext context = context();
    when(user.getEmail()).thenReturn("voter@example.com");
    when(user.isEnabled()).thenReturn(true);
    try (CapturedLogs logs = new CapturedLogs(SmartLinkAuthenticator.class);
        MockedStatic<LoginBridge> bridge = mockStatic(LoginBridge.class);
        MockedStatic<SmartLink> smartLink = mockStatic(SmartLink.class)) {
      smartLink
          .when(
              () ->
                  SmartLink.getOrCreate(
                      eq(session),
                      eq(realm),
                      eq("voter@example.com"),
                      eq(false),
                      eq(false),
                      eq(false),
                      any()))
          .thenReturn(user);
      bridge
          .when(() -> LoginBridge.linkFromActionToken(eq(session), eq(realm), any()))
          .thenReturn(LINK);
      smartLink
          .when(() -> SmartLink.sendSmartLinkNotification(session, user, LINK))
          .thenReturn(true);
      new SmartLinkAuthenticator().action(context);
      smartLink.verify(() -> SmartLink.sendSmartLinkNotification(session, user, LINK));
      verify(context.form()).createForm("view-email.ftl");
      assertFalse(logs.text().contains("synthetic-smart-link-token"));
    }
  }

  @Test
  void unknownUserIsRejectedWithoutDereferencingIt() {
    AuthenticationFlowContext context = context();
    try (MockedStatic<SmartLink> smartLink = mockStatic(SmartLink.class)) {
      new SmartLinkAuthenticator().action(context);
      verify(context)
          .failureChallenge(eq(AuthenticationFlowError.INVALID_USER), nullable(Response.class));
      verify(context, never()).success();
    }
  }
}
