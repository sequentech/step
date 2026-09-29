// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;

import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;

class ScanovateReturnResourceTest {
  private static final String SECRET = "callback-secret";

  private final LivenessSessions sessions =
      new LivenessSessions(new LivenessSessionsTest.MemoryStore(), millis -> {});
  private final ScanovateReturnResource resource =
      new ScanovateReturnResource(mock(KeycloakSession.class), ignored -> sessions);

  @Test
  void verifiesValidTokens() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(200, resource.verifyLivenessToken(token, "proc-1", SECRET).getStatus());
  }

  @Test
  void rejectsInvalidTokens() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(401, resource.verifyLivenessToken(token, "proc-1", "wrong").getStatus());
    assertEquals(401, resource.verifyLivenessToken("other", "proc-1", SECRET).getStatus());
    assertEquals(401, resource.verifyLivenessToken(null, null, null).getStatus());
  }

  @Test
  void recordsResultsFromTheTokenHeader() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(
        200,
        resource
            .livenessCallback(token, SECRET, LivenessSessionsTest.result("completed", true, JPEG))
            .getStatus());
    assertTrue(sessions.awaitResult(token, 0).orElseThrow().passed());
  }

  @Test
  void readsTheTokenFromTheBodyWithoutHeader() {
    String token = sessions.create("proc-1", SECRET);
    String body =
        LivenessSessionsTest.result("completed", true, JPEG)
            .replace(
                "\"case_id\": \"proc-1\"", "\"case_id\": \"proc-1\", \"token\": \"" + token + "\"");

    assertEquals(200, resource.livenessCallback(null, SECRET, body).getStatus());
    assertTrue(sessions.awaitResult(token, 0).orElseThrow().passed());
  }

  @Test
  void rejectsUnauthorizedAndMalformedCallbacks() {
    String token = sessions.create("proc-1", SECRET);
    String body = LivenessSessionsTest.result("completed", true, JPEG);

    assertEquals(401, resource.livenessCallback(token, "wrong", body).getStatus());
    assertEquals(401, resource.livenessCallback(null, SECRET, body).getStatus());
    assertEquals(400, resource.livenessCallback(token, SECRET, "not json").getStatus());
    assertEquals(400, resource.livenessCallback(token, SECRET, "[]").getStatus());
  }
}
