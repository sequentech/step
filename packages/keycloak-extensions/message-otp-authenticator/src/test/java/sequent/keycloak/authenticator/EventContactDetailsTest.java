// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.ArgumentCaptor;

/** R3-06: Keycloak events never carry a voter's mobile number or email address. */
class EventContactDetailsTest {
  private static final Pattern E164 = Pattern.compile("\\+[1-9][0-9]{6,14}");
  private static final Pattern EMAIL = Pattern.compile("[^\\s\"@]+@[^\\s\"@]+\\.[a-z]+");

  @Test
  void eventDetailsHoldNoNumberAndNoEmail() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    UserModel user = mock(UserModel.class);
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getSession()).thenReturn(session);
    when(context.getUser()).thenReturn(user);
    when(context.getEvent()).thenReturn(event);
    UserProfileProvider profiles = mock(UserProfileProvider.class);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profiles);
    UPConfig profile = new UPConfig();
    profile.setAttributes(
        List.of(
            new UPAttribute("firstName"),
            new UPAttribute("email"),
            new UPAttribute(Utils.PHONE_NUMBER_ATTRIBUTE),
            new UPAttribute("sequent.read-only.alternate-phone")));
    when(profiles.getConfiguration()).thenReturn(profile);
    Map<String, String> notes = new HashMap<>();
    notes.put("firstName", "Maria");
    notes.put("email", "maria.cruz@example.test");
    notes.put(Utils.PHONE_NUMBER_ATTRIBUTE, "+639171234567");
    notes.put("sequent.read-only.alternate-phone", "+34600111222");
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    when(user.getId()).thenReturn("voter-1");
    Map<String, List<String>> attributes = new HashMap<>();
    attributes.put("firstName", List.of("Maria"));
    attributes.put("email", List.of("maria.cruz@example.test"));
    attributes.put(Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+639171234567"));
    attributes.put("country", List.of("Spain"));
    when(user.getAttributes()).thenReturn(attributes);

    Utils.buildEventDetails(context, "MessageOTPAuthenticator");

    ArgumentCaptor<String> values = ArgumentCaptor.forClass(String.class);
    verify(event, atLeastOnce()).detail(anyString(), values.capture());
    for (String value : values.getAllValues()) {
      if (value == null) {
        continue;
      }
      assertFalse(E164.matcher(value).find(), "a number in the event: " + value);
      assertFalse(EMAIL.matcher(value).find(), "an email address in the event: " + value);
    }
    verify(event).detail("firstName", "Maria");
    assertTrue(
        values.getAllValues().stream().anyMatch(v -> v != null && v.contains("\"country\"")));
  }
}
