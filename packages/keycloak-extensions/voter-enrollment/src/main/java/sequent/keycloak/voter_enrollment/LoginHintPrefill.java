// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.userprofile.Attributes;

final class LoginHintPrefill {

  static final String HINT_PREFIX = "login_hint__";
  static final int MAX_HINT_COUNT = 5;
  static final int MAX_HINT_NAME_LENGTH = 128;
  static final int MAX_HINT_VALUE_LENGTH = 255;

  private static final String CLIENT_NOTE_PREFIX =
      AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX + HINT_PREFIX;
  private static final Pattern HINT_NAME_PATTERN = Pattern.compile("[A-Za-z0-9._-]+");
  private static final Set<String> CREDENTIAL_FIELDS =
      Set.of(RegistrationPage.FIELD_PASSWORD, RegistrationPage.FIELD_PASSWORD_CONFIRM);

  private LoginHintPrefill() {}

  static Map<String, String> extractHints(Map<String, String> clientNotes) {
    Map<String, String> hints = new LinkedHashMap<>();

    for (Map.Entry<String, String> clientNote : clientNotes.entrySet()) {
      if (!clientNote.getKey().startsWith(CLIENT_NOTE_PREFIX)) {
        continue;
      }

      String attributeName = clientNote.getKey().substring(CLIENT_NOTE_PREFIX.length());
      String value = clientNote.getValue();
      if (hints.size() == MAX_HINT_COUNT
          || attributeName.isEmpty()
          || attributeName.length() > MAX_HINT_NAME_LENGTH
          || !HINT_NAME_PATTERN.matcher(attributeName).matches()
          || value == null
          || value.isBlank()
          || value.length() > MAX_HINT_VALUE_LENGTH) {
        throw new IllegalArgumentException("Invalid login hint parameters");
      }

      hints.put(attributeName, value);
    }

    return Map.copyOf(hints);
  }

  static MultivaluedMap<String, String> filterWritableHints(
      Map<String, String> hints, Attributes attributes, Set<String> excludedAttributes) {
    MultivaluedMap<String, String> filteredHints = new MultivaluedHashMap<>();
    Map<String, ?> writableAttributes = attributes.getWritable();
    Map<String, ?> unmanagedAttributes = attributes.getUnmanagedAttributes();

    hints.forEach(
        (attributeName, value) -> {
          if (!CREDENTIAL_FIELDS.contains(attributeName)
              && !excludedAttributes.contains(attributeName)
              && writableAttributes.containsKey(attributeName)
              && !unmanagedAttributes.containsKey(attributeName)
              && attributes.getMetadata(attributeName) != null) {
            filteredHints.putSingle(attributeName, value);
          }
        });

    return filteredHints;
  }
}
