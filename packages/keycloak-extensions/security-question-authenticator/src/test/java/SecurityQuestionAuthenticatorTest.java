
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.security_question_authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.Collections;

public class SecurityQuestionAuthenticatorTest {

    private SecurityQuestionAuthenticator authenticator;
    private AuthenticationFlowContext context;
    private LoginFormsProvider mockFormsProvider;
    private AuthenticationSessionModel mockAuthSession;
    private UserModel mockUser;
    private RealmModel mockRealm;

    @BeforeEach
    void setUp() {
        authenticator = new SecurityQuestionAuthenticator();
        context = mock(AuthenticationFlowContext.class);
        mockFormsProvider = mock(LoginFormsProvider.class);
        mockAuthSession = mock(AuthenticationSessionModel.class);
        mockUser = mock(UserModel.class);
        mockRealm = mock(RealmModel.class);

        when(context.form()).thenReturn(mockFormsProvider);
        when(context.getRealm()).thenReturn(mockRealm);
        when(context.getAuthenticationSession()).thenReturn(mockAuthSession);
        when(context.getUser()).thenReturn(mockUser);
    }

    @Test
    void testAuthenticate() {
        Response mockResponse = mock(Response.class);
        when(mockFormsProvider.createForm(any())).thenReturn(mockResponse);

        authenticator.authenticate(context);

        verify(context).challenge(mockResponse);
    }

    @Test
    void testAction_ValidAnswer() {
        // Mock form data
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        // Mock config
        AuthenticatorConfigModel mockConfig = mock(AuthenticatorConfigModel.class);
        when(Utils.getConfig(mockRealm)).thenReturn(java.util.Optional.of(mockConfig));
        when(mockConfig.getConfig()).thenReturn(Collections.singletonMap(Utils.NUM_LAST_CHARS, "4"));

        // Mock user attribute
        when(mockUser.getFirstAttribute(anyString())).thenReturn("secretValue");

        // Mock validation to return true
        when(mockAuthSession.getExecution()).thenReturn(mock(AuthenticationExecutionModel.class));
        when(mockAuthSession.getExecution().isRequired()).thenReturn(true);

        authenticator.action(context);

        verify(context).failureChallenge(
            eq(AuthenticationFlowError.INVALID_CREDENTIALS),
            any()
        );
    }

    @Test
    void testAction_InvalidAnswer() {
        // Mock form data
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("invalidAnswer");

        // Mock config
        AuthenticatorConfigModel mockConfig = mock(AuthenticatorConfigModel.class);
        when(Utils.getConfig(mockRealm)).thenReturn(java.util.Optional.of(mockConfig));
        when(mockConfig.getConfig()).thenReturn(Collections.singletonMap(Utils.NUM_LAST_CHARS, "4"));

        // Mock user attribute
        when(mockUser.getFirstAttribute(anyString())).thenReturn("secretValue");

        authenticator.action(context);

        verify(context).attempted();
    }
}
