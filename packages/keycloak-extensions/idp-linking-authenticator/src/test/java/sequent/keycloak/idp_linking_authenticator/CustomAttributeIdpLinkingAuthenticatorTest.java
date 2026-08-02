// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_linking_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.authenticators.broker.util.SerializedBrokeredIdentityContext;
import org.keycloak.broker.provider.BrokeredIdentityContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

@ExtendWith(MockitoExtension.class)
public class CustomAttributeIdpLinkingAuthenticatorTest {

  private CustomAttributeIdpLinkingAuthenticator authenticator;

  @Mock private AuthenticationFlowContext context;
  @Mock private SerializedBrokeredIdentityContext serializedCtx;
  @Mock private BrokeredIdentityContext brokerContext;
  @Mock private AuthenticatorConfigModel authConfig;
  @Mock private KeycloakSession session;
  @Mock private RealmModel realm;
  @Mock private UserProvider userProvider;
  @Mock private UserModel user;
  @Mock private AuthenticationSessionModel authSession;

  @BeforeEach
  void setUp() {
    authenticator = new CustomAttributeIdpLinkingAuthenticator();
  }

  // ── Configuration handling ──────────────────────────────────────────────────

  @Test
  void testAuthenticateImpl_nullConfig_attempts() {
    when(context.getAuthenticatorConfig()).thenReturn(null);

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
    verify(context, never()).success();
    verify(context, never()).failure(any());
  }

  @Test
  void testAuthenticateImpl_nullConfigMap_attempts() {
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig()).thenReturn(null);

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
    verify(context, never()).success();
  }

  // ── Claim extraction ────────────────────────────────────────────────────────

  @Test
  void testAuthenticateImpl_emptyClaimValue_attempts() {
    setConfig(
        CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_IDP_CLAIM,
        CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_USER_ATTRIBUTE);
    when(brokerContext.getEmail()).thenReturn("");

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
  }

  @Test
  void testAuthenticateImpl_nullClaimValue_attempts() {
    setConfig(
        CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_IDP_CLAIM,
        CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_USER_ATTRIBUTE);
    when(brokerContext.getEmail()).thenReturn(null);

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
  }

  // ── User lookup results ─────────────────────────────────────────────────────

  @Test
  void testAuthenticateImpl_noMatchingUser_attempts() {
    setConfig("email", "linked_idp_identities");
    when(brokerContext.getEmail()).thenReturn("voter@example.com");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(userProvider);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("voter@example.com")))
        .thenReturn(Stream.empty());

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
    verify(context, never()).success();
  }

  @Test
  void testAuthenticateImpl_exactlyOneUser_setsUserAndSucceeds() {
    setConfig("email", "linked_idp_identities");
    when(brokerContext.getEmail()).thenReturn("voter@example.com");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(session.users()).thenReturn(userProvider);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("voter@example.com")))
        .thenReturn(Stream.of(user));
    when(user.getUsername()).thenReturn("voter1");

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).setUser(user);
    verify(context).success();
    verify(context, never()).attempted();
    verify(context, never()).failure(any());
  }

  @Test
  void testAuthenticateImpl_multipleMatchingUsers_fails() {
    setConfig("email", "linked_idp_identities");
    when(brokerContext.getEmail()).thenReturn("shared@example.com");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(userProvider);

    UserModel user2 = mock(UserModel.class);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("shared@example.com")))
        .thenReturn(Stream.of(user, user2));

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).failure(AuthenticationFlowError.IDENTITY_PROVIDER_ERROR);
    verify(context, never()).success();
    verify(context, never()).attempted();
  }

  // ── Configurable claim names ────────────────────────────────────────────────

  @Test
  void testAuthenticateImpl_customClaim_usesUserAttribute() {
    setConfig("SAFE_ID", "linked_idp_identities");
    when(brokerContext.getUserAttribute("SAFE_ID")).thenReturn("SAFE-12345");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(session.users()).thenReturn(userProvider);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("SAFE-12345")))
        .thenReturn(Stream.of(user));
    when(user.getUsername()).thenReturn("voter1");

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).setUser(user);
    verify(context).success();
  }

  @Test
  void testAuthenticateImpl_configuredUsernameClaim_usesUsername() {
    setConfig("username", "linked_idp_identities");
    when(brokerContext.getUsername()).thenReturn("external_voter");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(userProvider);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("external_voter")))
        .thenReturn(Stream.empty());

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
  }

  @Test
  void testAuthenticateImpl_configuredIdClaim_usesId() {
    setConfig("id", "linked_idp_identities");
    when(brokerContext.getId()).thenReturn("external-id-999");
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(userProvider);
    when(userProvider.searchForUserByUserAttributeStream(
            eq(realm), eq("linked_idp_identities"), eq("external-id-999")))
        .thenReturn(Stream.empty());

    authenticator.authenticateImpl(context, serializedCtx, brokerContext);

    verify(context).attempted();
  }

  // ── extractClaimValue helper ────────────────────────────────────────────────

  @Test
  void testExtractClaimValue_email() {
    when(brokerContext.getEmail()).thenReturn("test@example.com");
    assertEquals("test@example.com", authenticator.extractClaimValue(brokerContext, "email"));
  }

  @Test
  void testExtractClaimValue_username() {
    when(brokerContext.getUsername()).thenReturn("testuser");
    assertEquals("testuser", authenticator.extractClaimValue(brokerContext, "username"));
  }

  @Test
  void testExtractClaimValue_id() {
    when(brokerContext.getId()).thenReturn("subject-abc");
    assertEquals("subject-abc", authenticator.extractClaimValue(brokerContext, "id"));
  }

  @Test
  void testExtractClaimValue_sub_aliasForId() {
    when(brokerContext.getId()).thenReturn("subject-abc");
    assertEquals("subject-abc", authenticator.extractClaimValue(brokerContext, "sub"));
  }

  @Test
  void testExtractClaimValue_firstname() {
    when(brokerContext.getFirstName()).thenReturn("Alice");
    assertEquals("Alice", authenticator.extractClaimValue(brokerContext, "firstname"));
  }

  @Test
  void testExtractClaimValue_firstNameUnderscore() {
    when(brokerContext.getFirstName()).thenReturn("Alice");
    assertEquals("Alice", authenticator.extractClaimValue(brokerContext, "first_name"));
  }

  @Test
  void testExtractClaimValue_lastname() {
    when(brokerContext.getLastName()).thenReturn("Smith");
    assertEquals("Smith", authenticator.extractClaimValue(brokerContext, "lastname"));
  }

  @Test
  void testExtractClaimValue_lastNameUnderscore() {
    when(brokerContext.getLastName()).thenReturn("Smith");
    assertEquals("Smith", authenticator.extractClaimValue(brokerContext, "last_name"));
  }

  @Test
  void testExtractClaimValue_customAttribute() {
    when(brokerContext.getUserAttribute("SAFE_ID")).thenReturn("SAFE-999");
    assertEquals("SAFE-999", authenticator.extractClaimValue(brokerContext, "SAFE_ID"));
  }

  @Test
  void testExtractClaimValue_unknownAttribute_returnsNull() {
    when(brokerContext.getUserAttribute("unknown_claim")).thenReturn(null);
    assertNull(authenticator.extractClaimValue(brokerContext, "unknown_claim"));
  }

  // ── Factory metadata ────────────────────────────────────────────────────────

  @Test
  void testFactory_providerIdAndDisplayType() {
    CustomAttributeIdpLinkingAuthenticatorFactory factory =
        new CustomAttributeIdpLinkingAuthenticatorFactory();
    assertEquals(CustomAttributeIdpLinkingAuthenticatorFactory.PROVIDER_ID, factory.getId());
    assertEquals("Custom Attribute IdP Identity Linking", factory.getDisplayType());
  }

  @Test
  void testFactory_configPropertiesHaveTwoEntries() {
    CustomAttributeIdpLinkingAuthenticatorFactory factory =
        new CustomAttributeIdpLinkingAuthenticatorFactory();
    List<org.keycloak.provider.ProviderConfigProperty> props = factory.getConfigProperties();
    assertEquals(2, props.size());
    assertEquals(
        CustomAttributeIdpLinkingAuthenticatorFactory.CONF_IDP_CLAIM, props.get(0).getName());
    assertEquals(
        CustomAttributeIdpLinkingAuthenticatorFactory.CONF_USER_ATTRIBUTE, props.get(1).getName());
  }

  // ── Helpers ─────────────────────────────────────────────────────────────────

  private void setConfig(String idpClaim, String userAttribute) {
    Map<String, String> config = new HashMap<>();
    config.put(CustomAttributeIdpLinkingAuthenticatorFactory.CONF_IDP_CLAIM, idpClaim);
    config.put(CustomAttributeIdpLinkingAuthenticatorFactory.CONF_USER_ATTRIBUTE, userAttribute);
    when(authConfig.getConfig()).thenReturn(config);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
  }
}
