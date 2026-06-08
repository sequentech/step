// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import jakarta.ws.rs.core.MultivaluedMap;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.stream.Stream;
import org.keycloak.models.UserModel;

/** First-generation-style Smart Link extra-field matching for Keycloak user attributes. */
final class SmartLinkRequiredAttributes {

  private SmartLinkRequiredAttributes() {}

  static final String EMAIL_ATTRIBUTE = "email";
  static final String TLF_ATTRIBUTE = "tlf";
  static final String TLF_USER_ATTRIBUTE = "sequent.read-only.mobile-number";

  static List<String> parse(String rawAttributes) {
    if (rawAttributes == null || rawAttributes.isBlank()) {
      return List.of();
    }

    return Arrays.stream(rawAttributes.split(","))
        .map(String::trim)
        .filter(attribute -> !attribute.isEmpty())
        .distinct()
        .toList();
  }

  static Optional<String> firstRejectedAttribute(
      UserModel user, MultivaluedMap<String, String> queryParameters, List<String> attributes) {
    for (String attribute : attributes) {
      if (!matches(user, queryParameters, attribute)) {
        return Optional.of(attribute);
      }
    }
    return Optional.empty();
  }

  private static boolean matches(
      UserModel user, MultivaluedMap<String, String> queryParameters, String attribute) {
    String requestedValue = queryParameters.getFirst(attribute);
    if (requestedValue == null) {
      return false;
    }

    if (EMAIL_ATTRIBUTE.equals(attribute)) {
      return matchesEmail(requestedValue, user.getEmail());
    }
    if (TLF_ATTRIBUTE.equals(attribute)) {
      return hasAttributeValue(user, TLF_USER_ATTRIBUTE, requestedValue);
    }
    return hasAttributeValue(user, attribute, requestedValue);
  }

  private static boolean hasAttributeValue(
      UserModel user, String attribute, String requestedValue) {
    return attributeValues(user, attribute).anyMatch(requestedValue::equals);
  }

  private static boolean matchesEmail(String requestedValue, String userEmail) {
    String requestedEmail = normalizeEmail(requestedValue);
    String storedEmail = normalizeEmail(userEmail);
    return !requestedEmail.isEmpty() && requestedEmail.equals(storedEmail);
  }

  private static Stream<String> attributeValues(UserModel user, String attribute) {
    Stream<String> values = user.getAttributeStream(attribute);
    return values == null ? Stream.empty() : values;
  }

  private static String normalizeEmail(String email) {
    if (email == null) {
      return "";
    }
    return email.strip().replace(" ", "").toLowerCase(Locale.ROOT);
  }
}
