// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;

import org.junit.Before;
import org.junit.Test;
import org.keycloak.authentication.actiontoken.DefaultActionToken;
import com.fasterxml.jackson.databind.ObjectMapper;

public class ManualVerificationTokenTest {

    private static final String USER_ID = "test-user-id";
    private static final int EXPIRATION = 3600;
    private static final String REDIRECT_URI = "http://example.com";

    private ManualVerificationToken token;

    @Before
    public void setUp() {
        token = new ManualVerificationToken(USER_ID, EXPIRATION, REDIRECT_URI);
    }

    @Test
    public void testTokenConstruction() {
        assertNotNull(token);
        assertEquals(USER_ID, token.getUserId());
        assertEquals(REDIRECT_URI, token.getRedirectUri());
        assertEquals(ManualVerificationToken.TOKEN_TYPE, token.getType());
    }

    @Test
    public void testSetRedirectUri() {
        String newRedirectUri = "http://newexample.com";
        token.setRedirectUri(newRedirectUri);
        assertEquals(newRedirectUri, token.getRedirectUri());
    }

    @Test
    public void testSerializationAndDeserialization() throws Exception {
        ObjectMapper mapper = new ObjectMapper();

        // Serialize
        String json = mapper.writeValueAsString(token);
        assertNotNull(json);

        // Deserialize
        ManualVerificationToken deserializedToken = mapper.readValue(json, ManualVerificationToken.class);
        assertNotNull(deserializedToken);
        assertEquals(USER_ID, deserializedToken.getUserId());
        assertEquals(REDIRECT_URI, deserializedToken.getRedirectUri());
        assertEquals(ManualVerificationToken.TOKEN_TYPE, deserializedToken.getType());
    }

    // Test default constructor used in deserialization
    @Test
    public void testDefaultConstructor() throws Exception {
        // Create a JSON representation of the token
        String json = "{\"userId\":\"" + USER_ID + "\",\"redirectUri\":\"" + REDIRECT_URI + "\",\"exp\":" + EXPIRATION + "}";

        // Deserialize using the default constructor
        ObjectMapper mapper = new ObjectMapper();
        ManualVerificationToken defaultToken = mapper.readValue(json, ManualVerificationToken.class);

        assertNotNull(defaultToken);
        assertEquals(USER_ID, defaultToken.getUserId());
        assertEquals(REDIRECT_URI, defaultToken.getRedirectUri());
    }
}
