// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/*  Testing Notes
        1. Token Expriration, Using Thread.sleep() to check time passing, must be greater than Tokens expirationInSecs.
        2. getExpiration() from the type JsonWebToken is deprecatedJava.
*/

public class ManualVerificationTokenTest {

  private ManualVerificationToken token;

  @BeforeEach
  public void setup() {
    String userId = "sampleUserId";
    int expirationInSecs = 3600;
    String redirectUri = "https://example.com/callback";
    token = new ManualVerificationToken(userId, expirationInSecs, redirectUri);
  }

  @Test
  public void testTokenConstruction() {
    assertEquals("manual-verification-token", token.getType());
    assertEquals("sampleUserId", token.getUserId());
    assertEquals("https://example.com/callback", token.getRedirectUri());
  }
}
