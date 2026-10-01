// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.PNG;

import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;

class ScanovateReturnResourceTest {
  private static final String SECRET = "callback-secret";

  private final LivenessSessions sessions =
      new LivenessSessions(new LivenessSessionsTest.MemoryStore(), millis -> {});
  private final CaptureUploads uploads = new CaptureUploads(new LivenessSessionsTest.MemoryStore());
  private final ScanovateReturnResource resource =
      new ScanovateReturnResource(
          mock(KeycloakSession.class), ignored -> sessions, ignored -> uploads);

  /** The voter's browser sees these requests: they must not name the identity provider. */
  @Test
  void exposesVendorNeutralNames() {
    assertEquals("identity-verification", new ScanovateReturnResourceFactory().getId());
    assertEquals("X-Capture-Token", ScanovateReturnResource.CAPTURE_TOKEN_HEADER);
  }

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

  // Keycloak's security headers turn a response without a media type into a 500, which Liveness
  // Plus reads as a rejected token or a failed callback.
  @Test
  void everyLivenessResponseIsJson() {
    String token = sessions.create("proc-1", SECRET);
    String body = LivenessSessionsTest.result("completed", true, JPEG);

    for (Response response :
        List.of(
            resource.verifyLivenessToken(token, "proc-1", SECRET),
            resource.verifyLivenessToken(token, "proc-1", "wrong"),
            resource.livenessCallback(token, SECRET, body),
            resource.livenessCallback(token, "wrong", body),
            resource.livenessCallback(token, SECRET, "not json"))) {
      assertTrue(
          MediaType.APPLICATION_JSON_TYPE.isCompatible(response.getMediaType()),
          response.getStatus() + " has media type " + response.getMediaType());
    }
  }

  private static InputStream body(byte[] content) {
    return new ByteArrayInputStream(content);
  }

  @Test
  void storesUploadedCaptureParts() {
    String token = uploads.create();

    Response response = resource.uploadCapture("front", token, body(JPEG));

    assertEquals(200, response.getStatus());
    assertArrayEquals(JPEG, uploads.parts(token).orElseThrow().get("front"));
  }

  @Test
  void rejectsInvalidUploads() {
    String token = uploads.create();
    byte[] oversized = new byte[CaptureUploads.MAX_UPLOAD_BYTES + 1];
    System.arraycopy(JPEG, 0, oversized, 0, JPEG.length);

    assertEquals(401, resource.uploadCapture("front", "forged", body(JPEG)).getStatus());
    assertEquals(401, resource.uploadCapture("front", null, body(JPEG)).getStatus());
    assertEquals(400, resource.uploadCapture("selfie", token, body(JPEG)).getStatus());
    assertEquals(400, resource.uploadCapture("front", token, body(new byte[0])).getStatus());
    assertEquals(400, resource.uploadCapture("front", token, null).getStatus());
    assertEquals(413, resource.uploadCapture("front", token, body(oversized)).getStatus());
    assertEquals(415, resource.uploadCapture("front", token, body(PNG)).getStatus());
    assertEquals(0, uploads.parts(token).orElseThrow().size());
  }

  @Test
  void everyUploadResponseIsJson() {
    String token = uploads.create();

    for (Response response :
        List.of(
            resource.uploadCapture("front", token, body(JPEG)),
            resource.uploadCapture("front", "forged", body(JPEG)),
            resource.uploadCapture("selfie", token, body(JPEG)),
            resource.uploadCapture("front", token, body(PNG)))) {
      assertTrue(
          MediaType.APPLICATION_JSON_TYPE.isCompatible(response.getMediaType()),
          response.getStatus() + " has media type " + response.getMediaType());
    }
  }
}
