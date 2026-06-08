// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.ivr_config;

import static org.assertj.core.api.Assertions.assertThat;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.WebApplicationException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.representations.AccessToken;

class IvrConfigResourceProviderTest {
  private KeycloakSession session;
  private RealmModel realm;

  @BeforeEach
  void setUp() {
    session = mock(KeycloakSession.class);
    KeycloakContext ctx = mock(KeycloakContext.class);
    realm = mock(RealmModel.class);
    AuthenticationFlowModel flow = mock(AuthenticationFlowModel.class);

    when(session.getContext()).thenReturn(ctx);
    when(ctx.getRealm()).thenReturn(realm);
    when(flow.getId()).thenReturn("flow-id");
    when(realm.getDirectGrantFlow()).thenReturn(flow);
    // No ivr-voting client flow override by default, fall back to realm default flow.
    lenient()
        .when(realm.getClientByClientId(IvrConfigResourceProvider.IVR_VOTING_CLIENT_ID))
        .thenReturn(null);
  }

  // ---------- Happy paths ----------

  @Test
  void stockUsernameAndPasswordProducesExpectedSteps() {
    stubExecutions(
        exec(
            "direct-grant-validate-username",
            AuthenticationExecutionModel.Requirement.REQUIRED,
            null),
        exec(
            "direct-grant-validate-password",
            AuthenticationExecutionModel.Requirement.REQUIRED,
            null));
    IvrConfigResourceProvider provider = providerWithValidToken();

    @SuppressWarnings("unchecked")
    Map<String, Object> body = (Map<String, Object>) provider.getIvrConfig().getEntity();
    @SuppressWarnings("unchecked")
    List<AuthStep> steps = (List<AuthStep>) body.get(Constants.IVR_CONFIG_FIELD_STEPS);

    assertThat(steps)
        .containsExactly(
            new AuthStep("voter_id", 8, Constants.AUTH_STEP_KIND_IDENTIFIER, "username", null),
            new AuthStep("pin", 8, Constants.AUTH_STEP_KIND_SECRET, "password", null));
  }

  @Test
  void ivrVotingClientFlowOverridePrefersClientDirectGrantFlow() {
    // Setup: ivr-voting client exists with override direct grant flow
    ClientModel client = mock(org.keycloak.models.ClientModel.class);
    AuthenticationFlowModel overrideFlow = mock(AuthenticationFlowModel.class);

    when(overrideFlow.getId()).thenReturn("override-flow-id");
    when(client.getAuthenticationFlowBindingOverride(
            IvrConfigResourceProvider.IVR_VOTING_OVERRIDE_FLOW))
        .thenReturn("override-flow-id");
    when(realm.getClientByClientId(IvrConfigResourceProvider.IVR_VOTING_CLIENT_ID))
        .thenReturn(client);
    when(realm.getAuthenticationFlowById("override-flow-id")).thenReturn(overrideFlow);

    // Stub executions for the override flow (different from realm default)
    when(realm.getAuthenticationExecutionsStream(eq("override-flow-id")))
        .thenAnswer(
            inv ->
                Stream.of(
                    exec(
                        "direct-grant-validate-username",
                        AuthenticationExecutionModel.Requirement.REQUIRED,
                        null)));

    IvrConfigResourceProvider provider = providerWithValidToken();
    @SuppressWarnings("unchecked")
    List<AuthStep> steps =
        (List<AuthStep>)
            ((Map<?, ?>) provider.getIvrConfig().getEntity()).get(Constants.IVR_CONFIG_FIELD_STEPS);

    // Verify only one step (from override flow), not two (from realm default)
    assertThat(steps).hasSize(1);
    assertThat(steps.get(0).field()).isEqualTo("voter_id");
  }

  @Test
  void customAuthenticatorReadsConfigKeys() {
    AuthenticationExecutionModel custom =
        exec("ivr-dob-authenticator", AuthenticationExecutionModel.Requirement.REQUIRED, "cfg-1");
    stubExecutions(custom);
    AuthenticatorConfigModel cfg = mock(AuthenticatorConfigModel.class);
    when(cfg.getConfig())
        .thenReturn(
            Map.of(
                Constants.AUTH_STEP_PROP_FIELD, "dob",
                Constants.AUTH_STEP_PROP_MAX_DIGITS, "8",
                Constants.AUTH_STEP_PROP_KIND, Constants.AUTH_STEP_KIND_SECRET,
                Constants.AUTH_STEP_PROP_MAPS_TO, "dob",
                Constants.AUTH_STEP_PROP_PROMPT_KEY, "auth_enter_dob"));
    when(realm.getAuthenticatorConfigById("cfg-1")).thenReturn(cfg);

    IvrConfigResourceProvider provider = providerWithValidToken();
    @SuppressWarnings("unchecked")
    List<AuthStep> steps =
        (List<AuthStep>)
            ((Map<?, ?>) provider.getIvrConfig().getEntity()).get(Constants.IVR_CONFIG_FIELD_STEPS);

    assertThat(steps)
        .containsExactly(
            new AuthStep("dob", 8, Constants.AUTH_STEP_KIND_SECRET, "dob", "auth_enter_dob"));
  }

  @Test
  void alternativeDisabledAndSubflowExecutionsAreSkipped() {
    AuthenticationExecutionModel subflow = mock(AuthenticationExecutionModel.class);
    when(subflow.isAuthenticatorFlow()).thenReturn(true);
    stubExecutions(
        subflow,
        exec(
            "direct-grant-validate-username",
            AuthenticationExecutionModel.Requirement.ALTERNATIVE,
            null),
        exec(
            "direct-grant-validate-password",
            AuthenticationExecutionModel.Requirement.DISABLED,
            null),
        exec(
            "direct-grant-validate-username",
            AuthenticationExecutionModel.Requirement.REQUIRED,
            null));

    IvrConfigResourceProvider provider = providerWithValidToken();
    @SuppressWarnings("unchecked")
    List<AuthStep> steps =
        (List<AuthStep>)
            ((Map<?, ?>) provider.getIvrConfig().getEntity()).get(Constants.IVR_CONFIG_FIELD_STEPS);

    assertThat(steps).hasSize(1);
    assertThat(steps.get(0).field()).isEqualTo("voter_id");
  }

  // ---------- Failure paths ----------

  @Test
  void unknownAuthenticatorWithoutConfigYields500() {
    stubExecutions(
        exec("unknown-authenticator", AuthenticationExecutionModel.Requirement.REQUIRED, null));

    IvrConfigResourceProvider provider = providerWithValidToken();
    WebApplicationException e = assertThrows(WebApplicationException.class, provider::getIvrConfig);
    assertEquals(500, e.getResponse().getStatus());
  }

  @Test
  void customAuthenticatorMissingRequiredKeysYields500() {
    stubExecutions(exec("custom-x", AuthenticationExecutionModel.Requirement.REQUIRED, "cfg-x"));
    AuthenticatorConfigModel cfg = mock(AuthenticatorConfigModel.class);
    when(cfg.getConfig())
        .thenReturn(new HashMap<>(Map.of(Constants.AUTH_STEP_PROP_MAX_DIGITS, "4")));
    when(realm.getAuthenticatorConfigById("cfg-x")).thenReturn(cfg);

    IvrConfigResourceProvider provider = providerWithValidToken();
    WebApplicationException e = assertThrows(WebApplicationException.class, provider::getIvrConfig);
    assertEquals(500, e.getResponse().getStatus());
  }

  // ---------- Auth ----------

  @Test
  void missingTokenYields401() {
    stubExecutions();
    IvrConfigResourceProvider provider =
        new IvrConfigResourceProvider(session) {
          @Override
          AccessToken extractToken() {
            return null;
          }
        };
    WebApplicationException e = assertThrows(WebApplicationException.class, provider::getIvrConfig);
    assertEquals(401, e.getResponse().getStatus());
  }

  @Test
  void serviceTokenWithoutRoleYields403() {
    AccessToken token = tokenWith("ivr-service", false);
    IvrConfigResourceProvider provider = providerWithToken(token);
    WebApplicationException e = assertThrows(WebApplicationException.class, provider::getIvrConfig);
    assertEquals(403, e.getResponse().getStatus());
  }

  // ---------- Test helpers ----------

  /**
   * Subclass override seam — the real implementation uses {@code Tokens.getAccessToken(session)},
   * which depends on a fully wired KeycloakContext. Tests inject the token directly.
   */
  private IvrConfigResourceProvider providerWithToken(AccessToken token) {
    return new IvrConfigResourceProvider(session) {
      @Override
      AccessToken extractToken() {
        return token;
      }
    };
  }

  private IvrConfigResourceProvider providerWithValidToken() {
    return providerWithToken(tokenWith("ivr-service", true));
  }

  private AccessToken tokenWith(String azp, boolean hasRole) {
    AccessToken token = new AccessToken();
    token.issuedFor(azp);
    if (hasRole) {
      AccessToken.Access access = new AccessToken.Access();
      access.addRole(IvrConfigResourceProvider.REQUIRED_ROLE);
      token.setRealmAccess(access);
    }
    return token;
  }

  private static AuthenticationExecutionModel exec(
      String authenticator, AuthenticationExecutionModel.Requirement requirement, String configId) {
    AuthenticationExecutionModel e = mock(AuthenticationExecutionModel.class);
    when(e.isAuthenticatorFlow()).thenReturn(false);
    when(e.getAuthenticator()).thenReturn(authenticator);
    when(e.getRequirement()).thenReturn(requirement);
    when(e.getAuthenticatorConfig()).thenReturn(configId);
    return e;
  }

  private void stubExecutions(AuthenticationExecutionModel... execs) {
    when(realm.getAuthenticationExecutionsStream(eq("flow-id")))
        .thenAnswer(inv -> Stream.of(execs));
  }
}
