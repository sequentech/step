// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_SELF;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.mockito.MockedStatic;

/**
 * A DECIDE lookup before the contact code and a FINISH lookup after it: only the voters the
 * decision lets continue reach the code, so a rejected enrollment never sends an SMS or email.
 */
class LookupStageTest {

  private final Map<String, String> notes = new HashMap<>();
  private final Map<String, String> config = new HashMap<>();
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final LookupAndUpdateUser authenticator = new LookupAndUpdateUser();
  private MockedStatic<sequent.keycloak.authenticator.Utils> messaging;

  @BeforeEach
  void setUp() {
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(call -> notes.get(call.getArgument(0)));
    doAnswer(call -> notes.put(call.getArgument(0), call.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), any());
    RootAuthenticationSessionModel root = mock(RootAuthenticationSessionModel.class);
    when(root.getId()).thenReturn("enrollment-session");
    when(authSession.getParentSession()).thenReturn(root);

    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    when(session.getContext()).thenReturn(keycloakContext);

    AuthenticatorConfigModel configModel = mock(AuthenticatorConfigModel.class);
    when(configModel.getConfig()).thenReturn(config);
    config.put("messageCourierAttribute", "EMAIL");

    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getAuthenticatorConfig()).thenReturn(configModel);
    when(context.form()).thenReturn(form);
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getEvent()).thenReturn(event);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);

    messaging = mockStatic(sequent.keycloak.authenticator.Utils.class);
  }

  @AfterEach
  void tearDown() {
    messaging.close();
  }

  private void decided(String status) {
    notes.put(LookupAndUpdateUser.VERIFICATION_STATUS, status);
    notes.put(LookupAndUpdateUser.VERIFICATION_COMPLETED, "true");
  }

  private void verifyNothingSent() {
    messaging.verifyNoInteractions();
  }

  @Test
  void readsTheConfiguredStage() {
    assertEquals(LookupStage.DECIDE, LookupStage.fromConfig("DECIDE"));
    assertEquals(LookupStage.FINISH, LookupStage.fromConfig("FINISH"));
    assertEquals(LookupStage.DECIDE_AND_FINISH, LookupStage.fromConfig("DECIDE_AND_FINISH"));
  }

  /** Realms configured before the setting existed keep deciding and finishing in one step. */
  @Test
  void decidesAndFinishesWithoutAKnownStage() {
    assertEquals(LookupStage.DECIDE_AND_FINISH, LookupStage.fromConfig(null));
    assertEquals(LookupStage.DECIDE_AND_FINISH, LookupStage.fromConfig(""));
    assertEquals(LookupStage.DECIDE_AND_FINISH, LookupStage.fromConfig("decide"));
  }

  @Test
  void onlyAcceptedAndPendingEnrollmentsContinue() {
    assertTrue(LookupStage.continuesAfterDecision("ACCEPTED"));
    assertTrue(LookupStage.continuesAfterDecision("PENDING"));
    assertFalse(LookupStage.continuesAfterDecision("REJECTED"));
    assertFalse(LookupStage.continuesAfterDecision(null));
  }

  @Test
  void onlyTheDecideStageKeepsQuietAboutUncheckedContacts() {
    assertFalse(LookupStage.DECIDE.messagesUncheckedContacts());
    assertTrue(LookupStage.DECIDE_AND_FINISH.messagesUncheckedContacts());
    assertTrue(LookupStage.FINISH.messagesUncheckedContacts());
  }

  @Test
  void isAListSettingOfTheLookup() {
    ProviderConfigProperty property =
        authenticator.getConfigProperties().stream()
            .filter(p -> p.getName().equals(LookupAndUpdateUser.LOOKUP_STAGE))
            .findFirst()
            .orElseThrow();
    assertEquals(ProviderConfigProperty.LIST_TYPE, property.getType());
    assertEquals(LookupStage.DECIDE_AND_FINISH.name(), property.getDefaultValue());
    assertEquals(List.of("DECIDE_AND_FINISH", "DECIDE", "FINISH"), property.getOptions());
  }

  /** Coming back to a DECIDE lookup that let the voter continue goes on to the contact code. */
  @Test
  void aDecidedEnrollmentThatContinuesGoesOn() {
    config.put(LookupAndUpdateUser.LOOKUP_STAGE, "DECIDE");
    for (String status : List.of("ACCEPTED", "PENDING")) {
      decided(status);
      authenticator.authenticate(context);
    }

    verify(context, org.mockito.Mockito.times(2)).success();
    verify(form, never()).createForm(anyString());
    verifyNothingSent();
  }

  /** A rejected enrollment stays on its result: it never reaches the contact code. */
  @Test
  void aRejectedEnrollmentStaysOnTheResult() {
    config.put(LookupAndUpdateUser.LOOKUP_STAGE, "DECIDE");
    decided("REJECTED");

    authenticator.authenticate(context);

    verify(form).createForm("registration-rejected-finish.ftl");
    verify(context, never()).success();
    verifyNothingSent();
  }

  /** A FINISH lookup with no decision before it is a misconfigured flow, not an enrollment. */
  @Test
  void finishingWithoutADecisionFails() {
    config.put(LookupAndUpdateUser.LOOKUP_STAGE, "FINISH");

    authenticator.authenticate(context);

    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    verify(context, never()).success();
    assertNull(notes.get(LookupAndUpdateUser.ENROLLMENT_FINISHED_NOTE));
    verifyNothingSent();
  }

  /** An accepted decision names the voter: without one there is nobody to let in. */
  @Test
  void finishingAnAcceptanceWithoutTheVoterFails() {
    config.put(LookupAndUpdateUser.LOOKUP_STAGE, "FINISH");
    decided("ACCEPTED");

    authenticator.authenticate(context);

    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    verifyNothingSent();
  }

  /** Keycloak runs a step again on a reload: a finished enrollment shows its result again. */
  @Test
  void aFinishedEnrollmentShowsItsResultAgain() {
    config.put(LookupAndUpdateUser.LOOKUP_STAGE, "FINISH");
    decided("REJECTED");
    notes.put(LookupAndUpdateUser.ENROLLMENT_FINISHED_NOTE, "true");

    authenticator.authenticate(context);

    verify(form).createForm("registration-rejected-finish.ftl");
    verifyNothingSent();
  }
}
