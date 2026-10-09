// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.voter_enrollment;

import java.text.Normalizer;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import org.keycloak.models.UserModel;

/**
 * Checks that the user harvest returns for an accepted enrollment application matches the applicant
 * data on the configured search attributes.
 *
 * <p>The comparison follows harvest's automatic verification rules (at most one mismatch, and it
 * cannot be on another attribute while the embassy matches; first and middle name compared together
 * for some ID card types), with a normalization that is never stricter than harvest's: it ignores
 * case, accents and any character that is not a letter or a digit.
 */
final class ApplicantMatcher {

  static final String ID_CARD_TYPE_ATTRIBUTE = "sequent.read-only.id-card-type";
  static final String MIDDLE_NAME_ATTRIBUTE = "middleName";
  static final String EMBASSY_ATTRIBUTE = "embassy";

  private static final Set<String> COMBINED_NAME_CARD_TYPES =
      Set.of("seamanBook", "driversLicense");
  private static final int MAX_MISMATCHES = 1;

  private ApplicantMatcher() {}

  static boolean matchesApplicant(
      UserModel user, Map<String, String> applicantData, List<String> searchAttributes) {
    if (user == null || applicantData == null || searchAttributes.isEmpty()) {
      return false;
    }
    String cardType = applicantData.get(ID_CARD_TYPE_ATTRIBUTE);
    boolean combinedName = cardType != null && COMBINED_NAME_CARD_TYPES.contains(cardType);
    int mismatches = 0;
    boolean embassyMatches = false;
    for (String searchAttribute : searchAttributes) {
      String attribute = searchAttribute.trim();
      if (combinedName && MIDDLE_NAME_ATTRIBUTE.equals(attribute)) {
        continue;
      }
      boolean matches;
      if (combinedName && UserModel.FIRST_NAME.equals(attribute)) {
        matches = combinedNameMatches(user, applicantData);
      } else {
        String applicantValue = normalize(applicantData.get(attribute));
        matches =
            userValues(user, attribute)
                .anyMatch(userValue -> applicantValue.equals(normalize(userValue)));
      }
      if (EMBASSY_ATTRIBUTE.equals(attribute)) {
        embassyMatches = matches;
      }
      if (!matches) {
        mismatches++;
      }
    }
    return mismatches == 0 || (mismatches <= MAX_MISMATCHES && !embassyMatches);
  }

  private static boolean combinedNameMatches(UserModel user, Map<String, String> applicantData) {
    String applicantFirstName = applicantData.get(UserModel.FIRST_NAME);
    String applicantName =
        applicantFirstName == null
            ? ""
            : normalize(applicantFirstName) + normalize(applicantData.get(MIDDLE_NAME_ATTRIBUTE));
    String userFirstName = user.getFirstName();
    if (userFirstName == null) {
      return applicantName.isEmpty();
    }
    return userValues(user, MIDDLE_NAME_ATTRIBUTE)
        .anyMatch(
            userMiddleName ->
                applicantName.equals(normalize(userFirstName) + normalize(userMiddleName)));
  }

  private static Stream<String> userValues(UserModel user, String attribute) {
    switch (attribute) {
      case UserModel.FIRST_NAME:
        return Stream.of(user.getFirstName());
      case UserModel.LAST_NAME:
        return Stream.of(user.getLastName());
      case UserModel.USERNAME:
        return Stream.of(user.getUsername());
      case UserModel.EMAIL:
        return Stream.of(user.getEmail());
      default:
        List<String> values = user.getAttributeStream(attribute).toList();
        return values.isEmpty() ? Stream.of((String) null) : values.stream();
    }
  }

  private static String normalize(String value) {
    if (value == null) {
      return "";
    }
    StringBuilder normalized = new StringBuilder();
    value
        .toLowerCase(Locale.ROOT)
        .codePoints()
        .map(
            codePoint ->
                Normalizer.normalize(new String(Character.toChars(codePoint)), Normalizer.Form.NFD)
                    .codePointAt(0))
        .filter(Character::isLetterOrDigit)
        .forEach(normalized::appendCodePoint);
    return normalized.toString();
  }
}
