// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * The enrollment of the COMELEC template: a code by email if the voter gave an email, a code by SMS
 * if they gave a mobile number, then a code to both.
 */
class CodeProgressTest {
  private static final String CONDITION = "test-note-condition";

  private final RealmModel realm = mock(RealmModel.class);
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, List<AuthenticationExecutionModel>> flows = new HashMap<>();

  private AuthenticationExecutionModel emailCode;
  private AuthenticationExecutionModel smsCode;
  private AuthenticationExecutionModel finalCode;

  /** Matches when the auth note its config names is set and not blank. */
  static class NoteCondition implements ConditionalAuthenticator {
    @Override
    public boolean matchCondition(AuthenticationFlowContext context) {
      String note =
          context
              .getAuthenticationSession()
              .getAuthNote(context.getAuthenticatorConfig().getConfig().get("note"));
      return note != null && !note.isBlank();
    }

    @Override
    public void action(AuthenticationFlowContext context) {}

    @Override
    public boolean requiresUser() {
      return false;
    }

    @Override
    public void setRequiredActions(
        org.keycloak.models.KeycloakSession session, RealmModel realm, UserModel user) {}

    @Override
    public void close() {}
  }

  @BeforeEach
  void setUp() {
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakSessionFactory sessionFactory = mock(KeycloakSessionFactory.class);
    AuthenticatorFactory conditionFactory = mock(AuthenticatorFactory.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote(anyString())).thenAnswer(call -> notes.get(call.getArgument(0)));
    when(session.getKeycloakSessionFactory()).thenReturn(sessionFactory);
    when(sessionFactory.getProviderFactory(Authenticator.class, CONDITION))
        .thenReturn(conditionFactory);
    when(conditionFactory.create(session)).thenReturn(new NoteCondition());
    AuthenticatorFactory codeFactory = mock(AuthenticatorFactory.class);
    when(sessionFactory.getProviderFactory(
            Authenticator.class, MessageOTPAuthenticatorFactory.PROVIDER_ID))
        .thenReturn(codeFactory);
    when(codeFactory.create(session)).thenReturn(mock(Authenticator.class));

    AuthenticationFlowModel top = flow("registration");
    when(context.getTopLevelFlow()).thenReturn(top);
    execution("registration", "form", "registration-page-form", Requirement.REQUIRED);
    subflow("registration", "email-subflow", Requirement.CONDITIONAL);
    condition("email-subflow", "email");
    emailCode =
        execution(
            "email-subflow",
            "email-code",
            MessageOTPAuthenticatorFactory.PROVIDER_ID,
            Requirement.REQUIRED);
    subflow("registration", "sms-subflow", Requirement.CONDITIONAL);
    condition("sms-subflow", "mobile");
    smsCode =
        execution(
            "sms-subflow",
            "sms-code",
            MessageOTPAuthenticatorFactory.PROVIDER_ID,
            Requirement.REQUIRED);
    finalCode =
        execution(
            "registration",
            "final-code",
            MessageOTPAuthenticatorFactory.PROVIDER_ID,
            Requirement.REQUIRED);
    execution(
        "registration",
        "disabled-code",
        MessageOTPAuthenticatorFactory.PROVIDER_ID,
        Requirement.DISABLED);
    for (Map.Entry<String, List<AuthenticationExecutionModel>> entry : flows.entrySet()) {
      when(realm.getAuthenticationExecutionsStream(entry.getKey()))
          .thenAnswer(call -> entry.getValue().stream());
    }
  }

  @Test
  void countsTheCodesOfAVoterWithAnEmail() {
    notes.put("email", "voter@example.com");

    assertEquals(Optional.of(new CodeProgress(1, 2)), CodeProgress.of(at(emailCode)));
    assertEquals(Optional.of(new CodeProgress(2, 2)), CodeProgress.of(at(finalCode)));
  }

  @Test
  void countsTheCodesOfAVoterWithBothChannels() {
    notes.put("email", "voter@example.com");
    notes.put("mobile", "+34600000000");

    assertEquals(Optional.of(new CodeProgress(2, 3)), CodeProgress.of(at(smsCode)));
    assertEquals(Optional.of(new CodeProgress(3, 3)), CodeProgress.of(at(finalCode)));
  }

  @Test
  void showsNothingForASingleCode() {
    assertEquals(Optional.empty(), CodeProgress.of(at(finalCode)));
  }

  @Test
  void showsNothingWithoutThePolicy() {
    notes.put("email", "voter@example.com");
    when(context.getAuthenticatorConfig()).thenReturn(config(Map.of()));

    assertEquals(Optional.empty(), CodeProgress.of(context));
  }

  /** A count it can't be sure of is worse than none. */
  @Test
  void showsNothingWhenAConditionCannotBeEvaluated() {
    notes.put("email", "voter@example.com");
    when(context
            .getSession()
            .getKeycloakSessionFactory()
            .getProviderFactory(Authenticator.class, CONDITION))
        .thenReturn(null);

    assertEquals(Optional.empty(), CodeProgress.of(at(finalCode)));
  }

  @Test
  void isAListSettingOfTheCodeStep() {
    org.keycloak.provider.ProviderConfigProperty property =
        new MessageOTPAuthenticatorFactory()
            .getConfigProperties().stream()
                .filter(p -> p.getName().equals(CodeProgress.POLICY))
                .findFirst()
                .orElseThrow();
    assertEquals(org.keycloak.provider.ProviderConfigProperty.LIST_TYPE, property.getType());
    assertEquals(CodeProgressPolicy.NONE.name(), property.getDefaultValue());
    assertEquals(List.of("NONE", "SHOW"), property.getOptions());
  }

  @Test
  void readsThePolicy() {
    assertEquals(CodeProgressPolicy.SHOW, CodeProgressPolicy.fromConfig("SHOW"));
    assertEquals(CodeProgressPolicy.NONE, CodeProgressPolicy.fromConfig(null));
    assertEquals(CodeProgressPolicy.NONE, CodeProgressPolicy.fromConfig("unknown"));
  }

  private AuthenticationFlowContext at(AuthenticationExecutionModel execution) {
    when(context.getExecution()).thenReturn(execution);
    when(context.getAuthenticatorConfig())
        .thenReturn(config(Map.of(CodeProgress.POLICY, CodeProgressPolicy.SHOW.name())));
    return context;
  }

  private AuthenticationFlowModel flow(String id) {
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId(id);
    flows.put(id, new ArrayList<>());
    return flow;
  }

  private AuthenticationExecutionModel execution(
      String flowId, String id, String provider, Requirement requirement) {
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setId(id);
    execution.setParentFlow(flowId);
    execution.setAuthenticator(provider);
    execution.setRequirement(requirement);
    execution.setPriority(flows.get(flowId).size());
    flows.get(flowId).add(execution);
    return execution;
  }

  private void subflow(String parent, String id, Requirement requirement) {
    AuthenticationExecutionModel execution =
        execution(parent, id + "-execution", null, requirement);
    execution.setAuthenticatorFlow(true);
    execution.setFlowId(id);
    flow(id);
  }

  private void condition(String flowId, String note) {
    AuthenticationExecutionModel execution =
        execution(flowId, flowId + "-condition", CONDITION, Requirement.REQUIRED);
    execution.setAuthenticatorConfig(flowId + "-config");
    AuthenticatorConfigModel config = config(Map.of("note", note));
    when(realm.getAuthenticatorConfigById(flowId + "-config")).thenReturn(config);
  }

  private static AuthenticatorConfigModel config(Map<String, String> values) {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(new HashMap<>(values));
    return config;
  }
}
