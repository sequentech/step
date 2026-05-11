// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.lang.reflect.Method;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.models.UserModel;
import org.keycloak.userprofile.ValidationException;
import org.keycloak.validate.ValidationError;

class DeferredRegistrationUserCreationTest {

  @Test
  void normalizeFormParametersRemovesLocaleAndSensitiveFields() throws Exception {
    MultivaluedMap<String, String> formParams = new MultivaluedHashMap<>();
    formParams.add(RegistrationPage.FIELD_PASSWORD, "password");
    formParams.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "password");
    formParams.add(UserModel.LOCALE, "en");
    formParams.add(UserModel.EMAIL, "voter@example.com");

    Method method =
        DeferredRegistrationUserCreation.class.getDeclaredMethod(
            "normalizeFormParameters", MultivaluedMap.class);
    method.setAccessible(true);

    @SuppressWarnings("unchecked")
    MultivaluedMap<String, String> normalized =
        (MultivaluedMap<String, String>)
            method.invoke(new DeferredRegistrationUserCreation(), formParams);

    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD));
    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD_CONFIRM));
    assertFalse(normalized.containsKey(UserModel.LOCALE));
    assertTrue(normalized.containsKey(UserModel.EMAIL));
  }

  @Test
  void hiddenProfileAttributesIncludesLocale() {
    assertTrue(
        DeferredRegistrationUserCreation.HIDDEN_PROFILE_ATTRIBUTES.contains(UserModel.LOCALE));
  }

  @Test
  void isLocaleRequiredErrorOnlyMatchesRequiredLocale() {
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

    assertTrue(DeferredRegistrationUserCreation.isLocaleRequiredError(localeRequiredError));
    assertFalse(DeferredRegistrationUserCreation.isLocaleRequiredError(localeInvalidError));
    assertFalse(DeferredRegistrationUserCreation.isLocaleRequiredError(emailRequiredError));
  }
}
