// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Optional;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.models.UserModel;

class SmartLinkRequiredAttributesTest {

  @Test
  void parse_trimsEmptyValuesAndDuplicates() {
    assertEquals(
        List.of("email", "tlf", "department"),
        SmartLinkRequiredAttributes.parse(" email, tlf,,department,email "));
  }

  @Test
  void parse_acceptsBlankConfig() {
    assertEquals(List.of(), SmartLinkRequiredAttributes.parse(null));
    assertEquals(List.of(), SmartLinkRequiredAttributes.parse(" "));
  }

  @Test
  void firstRejectedAttribute_acceptsMatchingTextAttribute() {
    UserModel user = mock(UserModel.class);
    when(user.getAttributeStream("department")).thenReturn(Stream.of("math", "history"));

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();
    query.add("department", "history");

    assertEquals(
        Optional.empty(),
        SmartLinkRequiredAttributes.firstRejectedAttribute(user, query, List.of("department")));
  }

  @Test
  void firstRejectedAttribute_rejectsMissingTextAttribute() {
    UserModel user = mock(UserModel.class);

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();

    assertEquals(
        Optional.of("department"),
        SmartLinkRequiredAttributes.firstRejectedAttribute(user, query, List.of("department")));
  }

  @Test
  void firstRejectedAttribute_rejectsMismatchedTextAttribute() {
    UserModel user = mock(UserModel.class);
    when(user.getAttributeStream("department")).thenReturn(Stream.of("math"));

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();
    query.add("department", "Math");

    assertEquals(
        Optional.of("department"),
        SmartLinkRequiredAttributes.firstRejectedAttribute(user, query, List.of("department")));
  }

  @Test
  void firstRejectedAttribute_acceptsEmailCaseInsensitively() {
    UserModel user = mock(UserModel.class);
    when(user.getEmail()).thenReturn("Example@Sequentech.IO");

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();
    query.add("email", " example @sequentech.io ");

    assertEquals(
        Optional.empty(),
        SmartLinkRequiredAttributes.firstRejectedAttribute(user, query, List.of("email")));
  }

  @Test
  void firstRejectedAttribute_acceptsTlfFromMobileNumberAttribute() {
    UserModel user = mock(UserModel.class);
    when(user.getAttributeStream(SmartLinkRequiredAttributes.TLF_USER_ATTRIBUTE))
        .thenReturn(Stream.of("+34600111222"));

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();
    query.add("tlf", "+34600111222");

    assertEquals(
        Optional.empty(),
        SmartLinkRequiredAttributes.firstRejectedAttribute(user, query, List.of("tlf")));
  }

  @Test
  void firstRejectedAttribute_reportsFirstMismatch() {
    UserModel user = mock(UserModel.class);
    when(user.getEmail()).thenReturn("voter@example.org");
    when(user.getAttributeStream("department")).thenReturn(Stream.of("math"));

    MultivaluedMap<String, String> query = new MultivaluedHashMap<>();
    query.add("email", "voter@example.org");
    query.add("department", "history");

    Optional<String> rejected =
        SmartLinkRequiredAttributes.firstRejectedAttribute(
            user, query, List.of("email", "department", "tlf"));

    assertEquals(Optional.of("department"), rejected);
    assertFalse(rejected.equals(Optional.of("tlf")));
  }
}
