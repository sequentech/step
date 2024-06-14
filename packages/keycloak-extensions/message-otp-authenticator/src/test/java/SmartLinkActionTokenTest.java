// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.UUID;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.utils.KeycloakModelUtils;

public class SmartLinkActionTokenTest {

    private static final String USER_ID = "user-id";
    private static final int EXPIRATION_IN_SECS = 3600;
    private static final String NONCE = "e0b8c49e-b5a4-4d51-84db-73d8e1e0d9c7";
    private static final String CLIENT_ID = "client-id";
    private static final Boolean MARK_EMAIL_VERIFIED = true;
    private static final String REDIRECT_URI = "http://localhost";
    private static final String SCOPES = "openid profile";
    private static final String STATE = "state123";
    private static final Boolean REMEMBER_ME = false;
    private static final Boolean PERSISTENT = true;

    private SmartLinkActionToken token;

    @BeforeEach
    public void setup() {
        token = new SmartLinkActionToken(
            USER_ID,
            EXPIRATION_IN_SECS,
            NONCE,
            CLIENT_ID,
            MARK_EMAIL_VERIFIED,
            REDIRECT_URI,
            SCOPES,
            STATE,
            REMEMBER_ME,
            PERSISTENT
        );
    }

    @Test
    public void testConstructor() {
        assertEquals(USER_ID, token.getUserId());
        assertEquals(EXPIRATION_IN_SECS, token.getExpirationInSecs());
        assertEquals(CLIENT_ID, token.getIssuedFor());
        assertEquals(MARK_EMAIL_VERIFIED, token.getMarkEmailVerified());
        assertEquals(REDIRECT_URI, token.getRedirectUri());
        assertEquals(SCOPES, token.getScopes());
        assertEquals(STATE, token.getState());
        assertEquals(REMEMBER_ME, token.getRememberMe());
        assertEquals(PERSISTENT, token.getPersistent());
    }

    @Test
    public void testParseNonce() {
        UUID parsedNonce = SmartLinkActionToken.parseNonce(NONCE);
        assertNotNull(parsedNonce);
        assertEquals(NONCE, parsedNonce.toString());
    }

    @Test
    public void testParseNonceWithInvalidString() {
        UUID parsedNonce = SmartLinkActionToken.parseNonce("invalid-nonce");
        assertNull(parsedNonce);
    }

    @Test
    public void testSettersAndGetters() {
        token.setMarkEmailVerified(false);
        assertEquals(false, token.getMarkEmailVerified());

        token.setRedirectUri("http://example.com");
        assertEquals("http://example.com", token.getRedirectUri());

        token.setScopes("email");
        assertEquals("email", token.getScopes());

        token.setState("newState");
        assertEquals("newState", token.getState());

        token.setRememberMe(true);
        assertEquals(true, token.getRememberMe());

        token.setPersistent(false);
        assertEquals(false, token.getPersistent());
    }
}
