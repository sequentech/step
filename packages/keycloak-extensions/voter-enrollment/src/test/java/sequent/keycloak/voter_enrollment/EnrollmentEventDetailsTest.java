// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.ArgumentCaptor;

/** R3-06: enrollment events never carry a voter's mobile number or email address. */
class EnrollmentEventDetailsTest {
  private static final Pattern E164 = Pattern.compile("\\+[1-9][0-9]{6,14}");
  private static final Pattern EMAIL = Pattern.compile("[^\\s\"@]+@[^\\s\"@]+\\.[a-z]+");

  @Test
  void enrollmentEventDetailsHoldNoNumberAndNoEmail() {
    EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    RootAuthenticationSessionModel root = mock(RootAuthenticationSessionModel.class);
    when(root.getId()).thenReturn("synthetic-session");
    when(authSession.getParentSession()).thenReturn(root);
    KeycloakSession session = mock(KeycloakSession.class);
    UserProfileProvider profiles = mock(UserProfileProvider.class);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profiles);
    UPConfig profile = new UPConfig();
    profile.setAttributes(
        List.of(
            new UPAttribute("firstName"),
            new UPAttribute("email"),
            new UPAttribute("sequent.read-only.mobile-number"),
            new UPAttribute("embassy")));
    when(profiles.getConfiguration()).thenReturn(profile);
    Map<String, String> notes = new HashMap<>();
    notes.put("firstName", "Maria");
    notes.put("email", "maria.cruz@example.test");
    notes.put("sequent.read-only.mobile-number", "+639171234567");
    notes.put("embassy", "Synthetic Post");
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    UserModel user = mock(UserModel.class);
    when(user.getId()).thenReturn("voter-1");
    Map<String, List<String>> attributes = new HashMap<>();
    attributes.put("email", List.of("maria.cruz@example.test"));
    attributes.put("sequent.read-only.mobile-number", List.of("+639171234567"));
    attributes.put("embassy", List.of("Synthetic Post"));
    when(user.getAttributes()).thenReturn(attributes);

    Utils.buildEventDetails(event, authSession, user, session, "LookupAndUpdateUser");

    ArgumentCaptor<String> values = ArgumentCaptor.forClass(String.class);
    verify(event, atLeastOnce()).detail(anyString(), values.capture());
    for (String value : values.getAllValues()) {
      if (value != null) {
        assertFalse(E164.matcher(value).find(), "a number in the event: " + value);
        assertFalse(EMAIL.matcher(value).find(), "an email address in the event: " + value);
      }
    }
    verify(event).detail("embassy", "Synthetic Post");
  }
}
