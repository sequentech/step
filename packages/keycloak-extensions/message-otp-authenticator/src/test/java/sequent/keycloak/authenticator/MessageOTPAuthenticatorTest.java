// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.credential.CredentialModel;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;

class MessageOTPAuthenticatorTest {

  private static final String FLOW_ID = "flow-id";
  private static final String CONFIG_ID = "config-id";
  private static final String PHONE_ATTRIBUTE = MessageOTPAuthenticator.MOBILE_NUMBER_FIELD;
  private static final String PHONE_NUMBER = "+34600000000";
  private static final String EMAIL = "voter@example.com";

  private final MessageOTPAuthenticator authenticator = new MessageOTPAuthenticator();
  private final KeycloakSession session = mock(KeycloakSession.class);

  private Map<String, String> baseConfig() {
    Map<String, String> config = new HashMap<>();
    config.put(Utils.TEL_USER_ATTRIBUTE, PHONE_ATTRIBUTE);
    return config;
  }

  /**
   * Builds a realm mock whose authentication flows contain a single message-otp-authenticator
   * execution with the given config, so that {@code Utils.getConfig(realm)} finds it.
   */
  private RealmModel mockRealm(Map<String, String> configMap) {
    RealmModel realm = mock(RealmModel.class);
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId(FLOW_ID);
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    execution.setAuthenticatorConfig(CONFIG_ID);
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setId(CONFIG_ID);
    config.setConfig(configMap);
    when(realm.getAuthenticationFlowsStream()).thenAnswer(invocation -> Stream.of(flow));
    when(realm.getAuthenticationExecutionsStream(FLOW_ID))
        .thenAnswer(invocation -> Stream.of(execution));
    when(realm.getAuthenticatorConfigById(CONFIG_ID)).thenReturn(config);
    return realm;
  }

  private UserModel mockUser(
      String mobileNumber,
      String email,
      boolean hasCredential,
      SubjectCredentialManager credentialManager) {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(PHONE_ATTRIBUTE)).thenReturn(mobileNumber);
    when(user.getEmail()).thenReturn(email);
    when(user.credentialManager()).thenReturn(credentialManager);
    when(credentialManager.getStoredCredentialsByTypeStream(MessageOTPCredentialModel.TYPE))
        .thenAnswer(
            invocation ->
                hasCredential ? Stream.of(new CredentialModel()) : Stream.<CredentialModel>empty());
    when(credentialManager.createStoredCredential(any(CredentialModel.class)))
        .thenAnswer(invocation -> invocation.getArgument(0));
    return user;
  }

  @Test
  void autoCreateDisabledByDefaultDoesNotCreateCredentialAndReturnsFalse() {
    RealmModel realm = mockRealm(baseConfig());
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(PHONE_NUMBER, EMAIL, /* hasCredential= */ false, credentialManager);

    assertFalse(authenticator.configuredFor(session, realm, user));
    verify(credentialManager, never()).createStoredCredential(any(CredentialModel.class));
  }

  @Test
  void autoCreateEnabledCreatesCredentialForUserWithMobile() {
    Map<String, String> config = baseConfig();
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(PHONE_NUMBER, null, /* hasCredential= */ false, credentialManager);

    assertTrue(authenticator.configuredFor(session, realm, user));

    ArgumentCaptor<CredentialModel> captor = ArgumentCaptor.forClass(CredentialModel.class);
    verify(credentialManager).createStoredCredential(captor.capture());
    assertEquals(MessageOTPCredentialModel.TYPE, captor.getValue().getType());
  }

  @Test
  void autoCreateEnabledCreatesCredentialForUserWithEmailOnly() {
    Map<String, String> config = baseConfig();
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(null, EMAIL, /* hasCredential= */ false, credentialManager);

    assertTrue(authenticator.configuredFor(session, realm, user));
    verify(credentialManager).createStoredCredential(any(CredentialModel.class));
  }

  @Test
  void autoCreateEnabledWithoutMobileOrEmailReturnsFalse() {
    Map<String, String> config = baseConfig();
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(null, null, /* hasCredential= */ false, credentialManager);

    assertFalse(authenticator.configuredFor(session, realm, user));
    verify(credentialManager, never()).createStoredCredential(any(CredentialModel.class));
  }

  @Test
  void existingCredentialReturnsTrueWithoutCreatingAnother() {
    Map<String, String> config = baseConfig();
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(PHONE_NUMBER, EMAIL, /* hasCredential= */ true, credentialManager);

    assertTrue(authenticator.configuredFor(session, realm, user));
    verify(credentialManager, never()).createStoredCredential(any(CredentialModel.class));
  }

  @Test
  void nullUserReturnsFalseWhenNotDeferred() {
    Map<String, String> config = baseConfig();
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);

    assertFalse(authenticator.configuredFor(session, realm, null));
  }

  /**
   * Deferred mode relies on auth notes and must never require nor create a stored credential,
   * regardless of the auto-create credential setting.
   */
  private void assertDeferredUserWorksWithAutoCreateSetting(String autoCreateValue) {
    Map<String, String> config = baseConfig();
    config.put(Utils.DEFERRED_USER_ATTRIBUTE, "true");
    config.put(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE, autoCreateValue);
    RealmModel realm = mockRealm(config);

    KeycloakContext context = mock(KeycloakContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(session.getContext()).thenReturn(context);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote(PHONE_ATTRIBUTE)).thenReturn(PHONE_NUMBER);

    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    UserModel user = mockUser(null, null, /* hasCredential= */ false, credentialManager);

    assertTrue(authenticator.configuredFor(session, realm, user));
    verify(credentialManager, never()).createStoredCredential(any(CredentialModel.class));
  }

  @Test
  void deferredUserWorksWithAutoCreateEnabledAndNeverCreatesCredential() {
    assertDeferredUserWorksWithAutoCreateSetting("true");
  }

  @Test
  void deferredUserWorksWithAutoCreateDisabledAndNeverCreatesCredential() {
    assertDeferredUserWorksWithAutoCreateSetting("false");
  }

  @Test
  void deferredUserWithoutAuthNotesReturnsFalse() {
    Map<String, String> config = baseConfig();
    config.put(Utils.DEFERRED_USER_ATTRIBUTE, "true");
    RealmModel realm = mockRealm(config);

    KeycloakContext context = mock(KeycloakContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(session.getContext()).thenReturn(context);
    when(context.getAuthenticationSession()).thenReturn(authSession);

    assertFalse(authenticator.configuredFor(session, realm, null));
  }

  @Test
  void autoCreateCredentialFactoryPropertyIsDisabledByDefault() {
    Optional<ProviderConfigProperty> property =
        new MessageOTPAuthenticatorFactory()
            .getConfigProperties().stream()
                .filter(prop -> Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE.equals(prop.getName()))
                .findFirst();

    assertTrue(property.isPresent());
    assertEquals(ProviderConfigProperty.BOOLEAN_TYPE, property.get().getType());
    assertEquals("false", property.get().getDefaultValue());
  }
}
