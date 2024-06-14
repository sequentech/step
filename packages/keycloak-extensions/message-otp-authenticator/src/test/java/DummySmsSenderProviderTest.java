// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

import java.io.IOException;
import java.util.Collections;

import static org.mockito.Mockito.*;

public class DummySmsSenderProviderTest {

    private DummySmsSenderProvider smsSender;

    @Mock
    private KeycloakSession session;

    @Mock
    private RealmModel realm;

    @Mock
    private UserModel user;

    @BeforeEach
    public void setUp() {
        MockitoAnnotations.openMocks(this);
        smsSender = new DummySmsSenderProvider();
    }

    @AfterEach
    public void tearDown() {
        smsSender.close();
    }

    @Test
    public void testSendWithoutAttributes() throws IOException {
        String phoneNumber = "+1234567890";
        String message = "Test SMS message";

        smsSender.send(phoneNumber, message);

        // Verify that the log statements indicate the correct behavior
        // Here you might want to use a logging framework like Logback or Log4j
        // to capture logs and assert against them, or use mocking frameworks
        // like Mockito to mock the behavior further if needed.
    }

    @Test
    public void testSendWithAttributes() throws IOException {
        String phoneNumber = "+1234567890";
        String messageKey = "message.key";
        smsSender.send(phoneNumber, messageKey, Collections.emptyList(), realm, user, session);

        // Similarly, verify the behavior with attributes if needed
    }
}
