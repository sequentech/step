// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.voter_enrollment;

import java.text.Normalizer;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import org.keycloak.models.UserModel;

/**
 * Checks that the user harvest returns for an accepted enrollment application matches the applicant
 * data on the configured search attributes.
 *
 * <p>The comparison follows harvest's automatic verification rules (the first value of each user
 * attribute; at most one mismatch, and it cannot be on another attribute while the embassy matches;
 * first and middle name compared together for some ID card types), with a normalization that is
 * never stricter than harvest's: it ignores case, accents and any character that is not a letter or
 * a digit.
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
        matches =
            normalize(applicantData.get(attribute)).equals(normalize(userValue(user, attribute)));
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
    String userName =
        userFirstName == null
            ? ""
            : normalize(userFirstName) + normalize(user.getFirstAttribute(MIDDLE_NAME_ATTRIBUTE));
    return applicantName.equals(userName);
  }

  private static String userValue(UserModel user, String attribute) {
    switch (attribute) {
      case UserModel.FIRST_NAME:
        return user.getFirstName();
      case UserModel.LAST_NAME:
        return user.getLastName();
      case UserModel.USERNAME:
        return user.getUsername();
      case UserModel.EMAIL:
        return user.getEmail();
      default:
        return user.getFirstAttribute(attribute);
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
