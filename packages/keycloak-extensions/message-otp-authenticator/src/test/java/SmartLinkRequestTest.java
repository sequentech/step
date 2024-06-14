// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;

public class SmartLinkRequestTest {

    @Test
    public void testDeserializeFromJson() throws IOException {
        String json = "{\n" +
                "  \"username\": \"testuser\",\n" +
                "  \"email_or_username\": \"test@example.com\",\n" +
                "  \"client_id\": \"client1\",\n" +
                "  \"redirect_uri\": \"http://example.com/redirect\",\n" +
                "  \"expiration_seconds\": 3600,\n" +
                "  \"force_create\": true,\n" +
                "  \"update_profile\": true,\n" +
                "  \"update_password\": false,\n" +
                "  \"send_notification\": true,\n" +
                "  \"scopes\": \"openid profile\",\n" +
                "  \"nonce\": \"abcd1234\",\n" +
                "  \"state\": \"xyz\",\n" +
                "  \"remember_me\": true,\n" +
                "  \"reusable\": false,\n" +
                "  \"mark_email_verified\": false\n" +
                "}";
        ObjectMapper mapper = new ObjectMapper();
        SmartLinkRequest request = mapper.readValue(json, SmartLinkRequest.class);

        assertNotNull(request);
        assertEquals("testuser", request.getUsername());
        assertEquals("test@example.com", request.getEmailOrUsername());
        assertEquals("client1", request.getClientId());
        assertEquals("http://example.com/redirect", request.getRedirectUri());
        assertEquals(3600, request.getExpirationSeconds());
        assertTrue(request.isForceCreate());
        assertTrue(request.isUpdateProfile());
        assertFalse(request.isUpdatePassword());
        assertTrue(request.isSendNotification());
        assertEquals("openid profile", request.getScopes());
        assertEquals("abcd1234", request.getNonce());
        assertEquals("xyz", request.getState());
        assertTrue(request.getRememberMe());
        assertFalse(request.getActionTokenPersistent());
        assertFalse(request.getMarkEmailVerified());
    }

    @Test
    public void testDefaultValues() {
        SmartLinkRequest request = new SmartLinkRequest();

        assertEquals(60 * 60 * 24, request.getExpirationSeconds());
        assertFalse(request.isForceCreate());
        assertFalse(request.isUpdateProfile());
        assertFalse(request.isUpdatePassword());
        assertFalse(request.isSendNotification());
        assertNull(request.getScopes());
        assertNull(request.getNonce());
        assertNull(request.getState());
        assertFalse(request.getRememberMe());
        assertTrue(request.getActionTokenPersistent());
        assertTrue(request.getMarkEmailVerified());
    }
}


