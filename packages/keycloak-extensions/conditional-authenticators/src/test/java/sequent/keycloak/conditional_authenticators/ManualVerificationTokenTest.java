// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// 
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

 
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.Instant;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

/*  Testing Notes
        1. Token Expriration, Using Thread.sleep() to check time passing, must be greater than Tokens expirationInSecs.
        2. getExpiration() from the type JsonWebToken is deprecatedJava.
*/ 

public class ManualVerificationTokenTest {

    private ManualVerificationToken token;

    @BeforeEach
    public void setup() {
        // Initialize a sample ManualVerificationToken
        String userId = "sampleUserId";
        int expirationInSecs = 3; // Adjust expiration time for testing (e.g., 3 seconds)
        String redirectUri = "https://example.com/callback";

        token = new ManualVerificationToken(userId, expirationInSecs, redirectUri);
    }

    @Test
    public void testTokenConstruction() {
        // Verify token type
        assertEquals("manual-verification-token", token.getType());
        // Verify user ID
        assertEquals("sampleUserId", token.getUserId());
        // Verify redirect URI
        assertEquals("https://example.com/callback", token.getRedirectUri());
    }
 

 

 
}