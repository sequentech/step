// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.*;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.models.*;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.junit.jupiter.MockitoExtension;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;

import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class UtilsTest {

    @Mock
    private KeycloakSession keycloakSession;
    @Mock
    private RealmModel realmModel;
    @Mock
    private UserModel userModel;
    @Mock
    private AuthenticationSessionModel authenticationSessionModel;
    @Mock
    private SmsSenderProvider smsSenderProvider;
    @Mock
    private EmailTemplateProvider emailTemplateProvider;
    @Mock
    private EmailSenderProvider emailSenderProvider;

    private AuthenticatorConfigModel configModel;
    private Map<String, String> configMap;

    @BeforeEach
    public void setup() {
        MockitoAnnotations.openMocks(this);

        // Setup config
        configMap = new HashMap<>();
        configMap.put(Utils.CODE_LENGTH, "6");
        configMap.put(Utils.CODE_TTL, "60");
        configMap.put(Utils.TEL_USER_ATTRIBUTE, "mobile");
        configModel = new AuthenticatorConfigModel();
        configModel.setConfig(configMap);

        // Setup mock behavior
        when(keycloakSession.getProvider(SmsSenderProvider.class)).thenReturn(smsSenderProvider);
        when(keycloakSession.getProvider(EmailTemplateProvider.class)).thenReturn(emailTemplateProvider);
        when(keycloakSession.getProvider(EmailSenderProvider.class)).thenReturn(emailSenderProvider);
    }

    @Test
    public void testSendCode() throws IOException, EmailException {
        // Setup test data
        when(userModel.getEmail()).thenReturn("user@example.com");
        when(userModel.getFirstAttribute("mobile")).thenReturn("1234567890");
        when(authenticationSessionModel.getRealm()).thenReturn(realmModel);
        when(realmModel.getName()).thenReturn("test-realm");
        when(realmModel.getDisplayName()).thenReturn("Test Realm");

        // Execute method
        Utils.sendCode(configModel, keycloakSession, userModel, authenticationSessionModel, Utils.MessageCourier.BOTH, false);

        // Verify interactions
        verify(smsSenderProvider, times(1)).send(anyString(), eq(Utils.SEND_CODE_SMS_I18N_KEY), anyList(), eq(realmModel), eq(userModel), eq(keycloakSession));
        verify(emailTemplateProvider, times(1)).send(eq(Utils.SEND_CODE_EMAIL_SUBJECT), anyList(), eq(Utils.SEND_CODE_EMAIL_FTL), anyMap());
    }
}
