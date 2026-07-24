// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.lang.reflect.Method;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.models.UserModel;
import org.keycloak.userprofile.ValidationException;
import org.keycloak.validate.ValidationError;

class DeferredRegistrationUserCreationTest {

  private static final String CUSTOM_HIDDEN_ATTRIBUTE = "customHidden";

  @Test
  void segmentedCredentialIsEnabledOnlyForLoginModeAndRealmPolicy() {
    assertTrue(
        DeferredRegistrationUserCreation.isSegmentedCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                DeferredRegistrationUserCreation.SEGMENTED_NUMERIC_POLICY)));
    assertFalse(
        DeferredRegistrationUserCreation.isSegmentedCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.REGISTRATION.getValue(),
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                DeferredRegistrationUserCreation.SEGMENTED_NUMERIC_POLICY)));
    assertFalse(
        DeferredRegistrationUserCreation.isSegmentedCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                "standard")));
    assertFalse(
        DeferredRegistrationUserCreation.isSegmentedCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(), Map.of()));
    assertFalse(
        DeferredRegistrationUserCreation.isSegmentedCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(), null));
  }

  @Test
  void passwordCreationPolicyIsNotAppliedInLoginMode() {
    assertFalse(
        DeferredRegistrationUserCreation.shouldValidatePasswordCreationPolicy(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue()));
    assertTrue(
        DeferredRegistrationUserCreation.shouldValidatePasswordCreationPolicy(
            DeferredRegistrationUserCreation.FormMode.REGISTRATION.getValue()));
  }

  @Test
  void normalizeFormParametersRemovesHiddenAndSensitiveFields() throws Exception {
    MultivaluedMap<String, String> formParams = new MultivaluedHashMap<>();
    formParams.add(RegistrationPage.FIELD_PASSWORD, "password");
    formParams.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "password");
    formParams.add(UserModel.LOCALE, "en");
    formParams.add(CUSTOM_HIDDEN_ATTRIBUTE, "hidden");
    formParams.add(UserModel.EMAIL, "voter@example.com");

    Method method =
        DeferredRegistrationUserCreation.class.getDeclaredMethod(
            "normalizeFormParameters", MultivaluedMap.class, Set.class);
    method.setAccessible(true);

    @SuppressWarnings("unchecked")
    MultivaluedMap<String, String> normalized =
        (MultivaluedMap<String, String>)
            method.invoke(
                new DeferredRegistrationUserCreation(),
                formParams,
                Set.of(UserModel.LOCALE, CUSTOM_HIDDEN_ATTRIBUTE));

    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD));
    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD_CONFIRM));
    assertFalse(normalized.containsKey(UserModel.LOCALE));
    assertFalse(normalized.containsKey(CUSTOM_HIDDEN_ATTRIBUTE));
    assertTrue(normalized.containsKey(UserModel.EMAIL));
  }

  @Test
  void hiddenProfileAttributesDefaultToLocale() {
    assertEquals(
        Set.of(UserModel.LOCALE),
        DeferredRegistrationUserCreation.getHiddenProfileAttributes(Map.of()));
  }

  @Test
  void hiddenProfileAttributesCanBeConfigured() {
    assertEquals(
        Set.of(UserModel.LOCALE, CUSTOM_HIDDEN_ATTRIBUTE),
        DeferredRegistrationUserCreation.getHiddenProfileAttributes(
            Map.of(
                DeferredRegistrationUserCreation.HIDDEN_PROFILE_ATTRIBUTES,
                " locale, " + CUSTOM_HIDDEN_ATTRIBUTE + " ")));
  }

  @Test
  void isRequiredErrorForHiddenAttributeOnlyMatchesConfiguredHiddenAttributes() {
    ValidationException.Error localeRequiredError =
        new ValidationException.Error(
            new ValidationError(
                "required",
                UserModel.LOCALE,
                DeferredRegistrationUserCreation.MISSING_FIELDS_ERROR));
    ValidationException.Error localeInvalidError =
        new ValidationException.Error(
            new ValidationError(
                "validator", UserModel.LOCALE, ValidationError.MESSAGE_INVALID_VALUE));
    ValidationException.Error emailRequiredError =
        new ValidationException.Error(
            new ValidationError(
                "required",
                UserModel.EMAIL,
                DeferredRegistrationUserCreation.MISSING_FIELDS_ERROR));

    Set<String> hiddenProfileAttributes = Set.of(UserModel.LOCALE);

    assertTrue(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            localeRequiredError, hiddenProfileAttributes));
    assertFalse(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            localeInvalidError, hiddenProfileAttributes));
    assertFalse(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            emailRequiredError, hiddenProfileAttributes));
  }
}
