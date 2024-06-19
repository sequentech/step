// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only


/* 
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import sequent.keycloak.inetum_authenticator.DeferredRegistrationUserCreation;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.events.Errors;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.policy.PolicyError;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.validation.Validation;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.keycloak.userprofile.ValidationException;

import java.util.ArrayList;
import java.util.List;

public class DeferredRegistrationUserCreationTest {

    private DeferredRegistrationUserCreation deferredRegistrationUserCreation;
    private ValidationContext validationContext;
    private KeycloakSession session;
    private RealmModel realm;
    private UserProfileProvider userProfileProvider;
    private UserProfile userProfile;

    @BeforeEach
    public void setUp() {
        deferredRegistrationUserCreation = new DeferredRegistrationUserCreation();
        validationContext = mock(ValidationContext.class);
        session = mock(KeycloakSession.class);
        realm = mock(RealmModel.class);
        userProfileProvider = mock(UserProfileProvider.class);
        userProfile = mock(UserProfile.class);

        when(validationContext.getSession()).thenReturn(session);
        when(session.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
        when(userProfileProvider.create(any(UserProfileContext.class), any(MultivaluedMap.class))).thenReturn(userProfile);
        when(validationContext.getRealm()).thenReturn(realm);
    }

    @Test
    public void testValidateSuccess() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(RegistrationPage.FIELD_EMAIL, "test@example.com");
        formData.add(RegistrationPage.FIELD_PASSWORD, "password");
        formData.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "password");
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(realm.isRegistrationEmailAsUsername()).thenReturn(false);

        deferredRegistrationUserCreation.validate(validationContext);

        verify(validationContext).success();
    }

    @Test
    public void testValidateInvalidEmail() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(RegistrationPage.FIELD_EMAIL, "invalid-email");
        formData.add(RegistrationPage.FIELD_PASSWORD, "password");
        formData.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "password");
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(realm.isRegistrationEmailAsUsername()).thenReturn(false);

        doThrow(new ValidationException(new ValidationException.Error(Messages.INVALID_EMAIL))).when(userProfile).validate();

        deferredRegistrationUserCreation.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_EMAIL);
    }

    @Test
    public void testValidatePasswordMismatch() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(RegistrationPage.FIELD_EMAIL, "test@example.com");
        formData.add(RegistrationPage.FIELD_PASSWORD, "password");
        formData.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "different-password");
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(realm.isRegistrationEmailAsUsername()).thenReturn(false);

        deferredRegistrationUserCreation.validate(validationContext);

        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD_CONFIRM, Messages.INVALID_PASSWORD_CONFIRM));
        verify(validationContext).validationError(formData, errors);
    }

    @Test
    public void testValidatePasswordPolicyError() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(RegistrationPage.FIELD_EMAIL, "test@example.com");
        formData.add(RegistrationPage.FIELD_PASSWORD, "weakpassword");
        formData.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "weakpassword");
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        PasswordPolicyManagerProvider policyProvider = mock(PasswordPolicyManagerProvider.class);
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(policyProvider);
        when(policyProvider.validate(anyString(), anyString())).thenReturn(new PolicyError("password policy error"));

        when(realm.isRegistrationEmailAsUsername()).thenReturn(false);

        deferredRegistrationUserCreation.validate(validationContext);

        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, "password policy error"));
        verify(validationContext).validationError(formData, errors);
    }

    // Add more tests as needed...
}


*/