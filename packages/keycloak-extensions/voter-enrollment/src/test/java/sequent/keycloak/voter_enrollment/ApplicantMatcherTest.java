// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.UserModel;

class ApplicantMatcherTest {

  private static final String ID_CARD_NUMBER = "sequent.read-only.id-card-number";
  private static final String DATE_OF_BIRTH = "dateOfBirth";
  private static final String CARD_TYPE_PHILSYS = "philSysID";
  private static final String CARD_TYPE_SEAMAN_BOOK = "seamanBook";

  private static final List<String> SEARCH_ATTRIBUTES =
      List.of(
          UserModel.FIRST_NAME,
          ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE,
          UserModel.LAST_NAME,
          DATE_OF_BIRTH,
          ID_CARD_NUMBER,
          ApplicantMatcher.EMBASSY_ATTRIBUTE);

  private static UserModel user(
      String firstName, String lastName, Map<String, List<String>> attributes) {
    UserModel user = mock(UserModel.class);
    when(user.getFirstName()).thenReturn(firstName);
    when(user.getLastName()).thenReturn(lastName);
    attributes.forEach(
        (name, values) -> when(user.getFirstAttribute(name)).thenReturn(values.get(0)));
    return user;
  }

  private static Map<String, List<String>> registeredVoterAttributes() {
    Map<String, List<String>> attributes = new HashMap<>();
    attributes.put(ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE, List.of("Santos"));
    attributes.put(DATE_OF_BIRTH, List.of("1980-01-31"));
    attributes.put(ID_CARD_NUMBER, List.of("1234-5678"));
    attributes.put(ApplicantMatcher.EMBASSY_ATTRIBUTE, List.of("Madrid"));
    return attributes;
  }

  private static UserModel registeredVoter() {
    return user("José", "Dela-Cruz", registeredVoterAttributes());
  }

  private static Map<String, String> applicant(String cardType) {
    Map<String, String> data = new HashMap<>();
    data.put(UserModel.FIRST_NAME, "JOSE");
    data.put(ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE, "santos");
    data.put(UserModel.LAST_NAME, "dela cruz");
    data.put(DATE_OF_BIRTH, "1980-01-31");
    data.put(ID_CARD_NUMBER, "1234-5678");
    data.put(ApplicantMatcher.EMBASSY_ATTRIBUTE, "Madrid");
    data.put(ApplicantMatcher.ID_CARD_TYPE_ATTRIBUTE, cardType);
    return data;
  }

  @Test
  void acceptsUserMatchingEverySearchAttributeIgnoringCaseAccentsAndPunctuation() {
    assertTrue(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(), applicant(CARD_TYPE_PHILSYS), SEARCH_ATTRIBUTES));
  }

  @Test
  void rejectsUserWhoseSearchAttributesDoNotMatchApplicant() {
    Map<String, String> otherApplicant = applicant(CARD_TYPE_PHILSYS);
    otherApplicant.put(UserModel.FIRST_NAME, "Maria");
    otherApplicant.put(DATE_OF_BIRTH, "1990-12-01");
    otherApplicant.put(ID_CARD_NUMBER, "9999-0000");

    assertFalse(
        ApplicantMatcher.matchesApplicant(registeredVoter(), otherApplicant, SEARCH_ATTRIBUTES));
  }

  @Test
  void acceptsSingleMismatchOnEmbassy() {
    Map<String, String> movedApplicant = applicant(CARD_TYPE_PHILSYS);
    movedApplicant.put(ApplicantMatcher.EMBASSY_ATTRIBUTE, "Paris");

    assertTrue(
        ApplicantMatcher.matchesApplicant(registeredVoter(), movedApplicant, SEARCH_ATTRIBUTES));
  }

  @Test
  void rejectsSingleMismatchOnAnotherAttributeWhenEmbassyMatches() {
    Map<String, String> otherCardApplicant = applicant(CARD_TYPE_PHILSYS);
    otherCardApplicant.put(ID_CARD_NUMBER, "9999-0000");

    assertFalse(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(), otherCardApplicant, SEARCH_ATTRIBUTES));
  }

  @Test
  void acceptsSingleMismatchWhenEmbassyIsNotSearched() {
    Map<String, String> otherCardApplicant = applicant(CARD_TYPE_PHILSYS);
    otherCardApplicant.put(ID_CARD_NUMBER, "9999-0000");

    assertTrue(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(),
            otherCardApplicant,
            List.of(UserModel.FIRST_NAME, UserModel.LAST_NAME, DATE_OF_BIRTH, ID_CARD_NUMBER)));
  }

  @Test
  void comparesFirstAndMiddleNameTogetherForCombinedNameCardTypes() {
    Map<String, String> seamanBookApplicant = applicant(CARD_TYPE_SEAMAN_BOOK);
    seamanBookApplicant.put(UserModel.FIRST_NAME, "Jose Santos");
    seamanBookApplicant.remove(ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE);
    seamanBookApplicant.put(ApplicantMatcher.EMBASSY_ATTRIBUTE, "Paris");

    assertTrue(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(), seamanBookApplicant, SEARCH_ATTRIBUTES));

    Map<String, String> philSysApplicant = new HashMap<>(seamanBookApplicant);
    philSysApplicant.put(ApplicantMatcher.ID_CARD_TYPE_ATTRIBUTE, CARD_TYPE_PHILSYS);

    assertFalse(
        ApplicantMatcher.matchesApplicant(registeredVoter(), philSysApplicant, SEARCH_ATTRIBUTES));
  }

  @Test
  void comparesFirstNameAloneWhenIdCardTypeIsMissing() {
    Map<String, String> applicantWithoutCardType = applicant(CARD_TYPE_PHILSYS);
    applicantWithoutCardType.remove(ApplicantMatcher.ID_CARD_TYPE_ATTRIBUTE);

    assertTrue(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(), applicantWithoutCardType, SEARCH_ATTRIBUTES));
  }

  /** Harvest compares the first value of a user attribute, so a later value does not count. */
  @Test
  void comparesOnlyTheFirstValueOfMultiValuedAttributes() {
    Map<String, List<String>> attributes = registeredVoterAttributes();
    attributes.put(ID_CARD_NUMBER, List.of("9999-0000", "1234-5678"));

    assertFalse(
        ApplicantMatcher.matchesApplicant(
            user("José", "Dela-Cruz", attributes),
            applicant(CARD_TYPE_PHILSYS),
            SEARCH_ATTRIBUTES));
  }

  /** The combined first and middle name also uses only the first middle name of the user. */
  @Test
  void comparesOnlyTheFirstMiddleNameForCombinedNameCardTypes() {
    Map<String, List<String>> attributes = registeredVoterAttributes();
    attributes.put(ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE, List.of("Reyes", "Santos"));
    Map<String, String> seamanBookApplicant = applicant(CARD_TYPE_SEAMAN_BOOK);
    seamanBookApplicant.put(UserModel.FIRST_NAME, "Jose Santos");
    seamanBookApplicant.remove(ApplicantMatcher.MIDDLE_NAME_ATTRIBUTE);

    assertFalse(
        ApplicantMatcher.matchesApplicant(
            user("José", "Dela-Cruz", attributes), seamanBookApplicant, SEARCH_ATTRIBUTES));
  }

  @Test
  void rejectsMissingUser() {
    assertFalse(
        ApplicantMatcher.matchesApplicant(null, applicant(CARD_TYPE_PHILSYS), SEARCH_ATTRIBUTES));
  }

  @Test
  void rejectsWhenNoSearchAttributesAreConfigured() {
    assertFalse(
        ApplicantMatcher.matchesApplicant(
            registeredVoter(), applicant(CARD_TYPE_PHILSYS), List.of()));
  }
}
