// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;
import static sequent.keycloak.authenticator.Utils.ACCESS_DENIED;
import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.USER_NOT_FOUND;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.x509.X509AuthenticatorConfigModel;
import org.keycloak.events.Errors;
import org.keycloak.events.Event;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.sessions.AuthenticationSessionModel;

class X509UserResolutionAuthenticatorTest {

  private static final String IDENTITY = "e2e-cert-voter";

  private final Map<String, String> authNotes = new HashMap<>();
  private final X509AuthenticatorConfigModel config =
      new X509AuthenticatorConfigModel()
          .setUserIdentityMapperType(
              X509AuthenticatorConfigModel.IdentityMapperType.USERNAME_EMAIL);
  private X509UserResolutionAuthenticator authenticator;
  private AuthenticationFlowContext context;
  private EventBuilder event;
  private UserProvider users;
  private RealmModel realm;
  private UserModel resolvedUser;

  @BeforeEach
  void setUp() {
    authenticator = spy(new X509UserResolutionAuthenticator());
    context = mock(AuthenticationFlowContext.class);

    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> authNotes.get(i.getArgument(0)));
    doAnswer(i -> authNotes.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), anyString());
    doAnswer(i -> authNotes.remove(i.getArgument(0))).when(authSession).removeAuthNote(anyString());

    // Since Keycloak 26.8, EventBuilder.error() sends a clone, so the builder's own event never
    // carries the error.
    event = mock(EventBuilder.class);
    when(event.getEvent()).thenReturn(new Event());
    when(context.getEvent()).thenReturn(event);

    KeycloakSession session = mock(KeycloakSession.class);
    users = mock(UserProvider.class);
    realm = mock(RealmModel.class);
    when(session.users()).thenReturn(users);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getUser()).thenAnswer(i -> resolvedUser);
  }

  /** Stands in for X509ClientCertificateAuthenticator once the certificate passed validation. */
  private void parentLooksUpUser() {
    doAnswer(
            i -> {
              UserModel user =
                  authenticator.getUserIdentityToModelMapper(config).find(context, IDENTITY);
              if (user == null) {
                context.getEvent().error(Errors.USER_NOT_FOUND);
              } else if (!user.isEnabled()) {
                context.getEvent().error(Errors.USER_DISABLED);
              } else {
                resolvedUser = user;
              }
              return null;
            })
        .when(authenticator)
        .authenticateCertificate(context);
  }

  /** Stands in for X509ClientCertificateAuthenticator failing before the user lookup. */
  private void parentFailsBeforeLookup() {
    doNothing().when(authenticator).authenticateCertificate(context);
  }

  @Test
  void unknownUserAfterValidationIsUserNotFound() {
    when(users.getUserByUsername(realm, IDENTITY)).thenReturn(null);
    parentLooksUpUser();

    authenticator.authenticate(context);

    assertEquals(USER_NOT_FOUND, authNotes.get(AUTH_NOTE_DENY_TYPE));
    verify(event).detail(AUTH_NOTE_DENY_TYPE, USER_NOT_FOUND);
  }

  @Test
  void disabledUserIsUserNotFound() {
    UserModel user = mock(UserModel.class);
    when(user.isEnabled()).thenReturn(false);
    when(users.getUserByUsername(realm, IDENTITY)).thenReturn(user);
    parentLooksUpUser();

    authenticator.authenticate(context);

    assertEquals(USER_NOT_FOUND, authNotes.get(AUTH_NOTE_DENY_TYPE));
  }

  @Test
  void failureBeforeUserLookupIsAccessDenied() {
    parentFailsBeforeLookup();

    authenticator.authenticate(context);

    assertEquals(ACCESS_DENIED, authNotes.get(AUTH_NOTE_DENY_TYPE));
    verify(event).detail(AUTH_NOTE_DENY_TYPE, ACCESS_DENIED);
  }

  @Test
  void denyTypeFromClassifierIsKept() {
    authNotes.put(AUTH_NOTE_DENY_TYPE, "cert-not-provided");
    parentFailsBeforeLookup();

    authenticator.authenticate(context);

    assertEquals("cert-not-provided", authNotes.get(AUTH_NOTE_DENY_TYPE));
  }

  @Test
  void resolvedUserGetsNoDenyType() {
    UserModel user = mock(UserModel.class);
    when(user.isEnabled()).thenReturn(true);
    when(users.getUserByUsername(realm, IDENTITY)).thenReturn(user);
    parentLooksUpUser();

    authenticator.authenticate(context);

    assertNull(authNotes.get(AUTH_NOTE_DENY_TYPE));
    assertSame(user, context.getUser());
  }

  @Test
  void lookupMarkerFromPreviousAttemptDoesNotLeak() {
    authNotes.put(X509UserResolutionAuthenticator.AUTH_NOTE_USER_LOOKUP_FAILED, "true");
    parentFailsBeforeLookup();

    authenticator.authenticate(context);

    assertEquals(ACCESS_DENIED, authNotes.get(AUTH_NOTE_DENY_TYPE));
    assertFalse(
        authNotes.containsKey(X509UserResolutionAuthenticator.AUTH_NOTE_USER_LOOKUP_FAILED));
  }
}
