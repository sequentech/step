// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.sessions.AuthenticationSessionModel;

class ConditionalAuthNoteAuthenticatorTest {
  private static final String NOTE_KEY = "verified";
  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel session = mock(AuthenticationSessionModel.class);
  private final AuthenticatorConfigModel config = new AuthenticatorConfigModel();
  private final ConditionalAuthNoteAuthenticator authenticator =
      ConditionalAuthNoteAuthenticator.SINGLETON;

  private void configure(String expected, boolean regex, boolean negate, String actual) {
    Map<String, String> values = new HashMap<>();
    values.put(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_KEY, NOTE_KEY);
    if (expected != null) {
      values.put(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_VALUE, expected);
    }
    values.put(ConditionalAuthNoteAuthenticatorFactory.CONF_VALUE_IS_REGEX, String.valueOf(regex));
    values.put(ConditionalAuthNoteAuthenticatorFactory.CONF_NEGATE, String.valueOf(negate));
    config.setConfig(values);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(context.getAuthenticationSession()).thenReturn(session);
    when(session.getAuthNote(NOTE_KEY)).thenReturn(actual);
  }

  @Test
  void exactMatchingIsCaseSensitiveAndNegationInvertsOnlyAnEvaluatedMatch() {
    configure("yes", false, false, "yes");
    assertTrue(authenticator.matchCondition(context));
    verify(session).getAuthNote(NOTE_KEY);
    configure("yes", false, false, "YES");
    assertFalse(authenticator.matchCondition(context));
    configure("yes", false, true, "YES");
    assertTrue(authenticator.matchCondition(context));
    configure("yes", false, true, "yes");
    assertFalse(authenticator.matchCondition(context));
  }

  @Test
  void regexRequiresTheWholeValueAndLiteralModeDoesNotInterpretMetacharacters() {
    configure("proof-[0-9]+", true, false, "proof-17");
    assertTrue(authenticator.matchCondition(context));
    configure("proof-[0-9]+", true, false, "prefix-proof-17");
    assertFalse(authenticator.matchCondition(context));
    configure("proof-[0-9]+", false, false, "proof-17");
    assertFalse(authenticator.matchCondition(context));
    configure("proof-[0-9]+", false, false, "proof-[0-9]+");
    assertTrue(authenticator.matchCondition(context));
  }

  @Test
  void invalidRegexIsHandledAsANonMatchWithoutThrowing() {
    configure("[a]", true, false, "a");
    assertTrue(authenticator.matchCondition(context));
    configure("[", true, false, "a");
    assertFalse(authenticator.matchCondition(context));
  }

  @Test
  void absentExpectedValueMatchesBlankNotesButNotMissingNotes() {
    configure(null, false, false, "");
    assertTrue(authenticator.matchCondition(context));
    configure(null, false, false, " \t");
    assertTrue(authenticator.matchCondition(context));
    configure(null, false, false, "proof");
    assertFalse(authenticator.matchCondition(context));
    configure(null, false, false, null);
    assertFalse(authenticator.matchCondition(context));
  }

  @Test
  void missingSessionOrNoteDoesNotActivateANegatedFlow() {
    configure("yes", false, true, "no");
    assertTrue(authenticator.matchCondition(context));
    when(session.getAuthNote(NOTE_KEY)).thenReturn(null);
    assertFalse(authenticator.matchCondition(context));
    when(context.getAuthenticationSession()).thenReturn(null);
    assertFalse(authenticator.matchCondition(context));
  }

  @Test
  void missingConfigurationStopsBeforeReadingTheSession() {
    assertFalse(authenticator.matchCondition(context));
    when(context.getAuthenticatorConfig()).thenReturn(config);
    config.setConfig(null);
    assertFalse(authenticator.matchCondition(context));
    verify(context, never()).getAuthenticationSession();
  }
}
