// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static sequent.keycloak.scanovate_authenticator.OnPremSessionsTest.JPEG;

import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;

class IdentityVerificationResourceTest {
  private static final String SECRET = "callback-secret";

  private final OnPremSessionsTest.MemoryStore store = new OnPremSessionsTest.MemoryStore();
  private final OnPremSessions sessions =
      new OnPremSessions(OnPremService.LIVENESS, store, millis -> {});
  private final OnPremSessions ocr = new OnPremSessions(OnPremService.OCR, store, millis -> {});
  private final HoldingPhotos photos = new HoldingPhotos(store);
  private final IdentityVerificationResource resource =
      new IdentityVerificationResource(
          mock(KeycloakSession.class),
          (service, ignored) -> service == OnPremService.OCR ? ocr : sessions,
          ignored -> photos);

  private static java.io.InputStream body(byte[] content) {
    return new java.io.ByteArrayInputStream(content);
  }

  @Test
  void storesTheHoldingPhotoOfAnIssuedToken() {
    String token = photos.create();

    Response response = resource.uploadHoldingPhoto(token, body(JPEG));

    assertEquals(200, response.getStatus());
    assertEquals(MediaType.APPLICATION_JSON_TYPE, response.getMediaType());
    assertArrayEquals(JPEG, photos.photo(token).orElseThrow());
  }

  @Test
  void refusesHoldingPhotosItCannotUse() {
    String token = photos.create();

    assertEquals(401, resource.uploadHoldingPhoto(null, body(JPEG)).getStatus());
    assertEquals(401, resource.uploadHoldingPhoto("forged", body(JPEG)).getStatus());
    assertEquals(400, resource.uploadHoldingPhoto(token, null).getStatus());
    assertEquals(400, resource.uploadHoldingPhoto(token, body(new byte[0])).getStatus());
    assertEquals(415, resource.uploadHoldingPhoto(token, body("GIF89a".getBytes())).getStatus());
    assertEquals(
        413,
        resource
            .uploadHoldingPhoto(
                token, body(java.util.Arrays.copyOf(JPEG, HoldingPhotos.MAX_UPLOAD_BYTES + 1)))
            .getStatus());
    assertTrue(photos.photo(token).isEmpty());
  }

  @Test
  void anUnreadableUploadIsABadRequest() {
    java.io.InputStream broken =
        new java.io.InputStream() {
          @Override
          public int read() throws java.io.IOException {
            throw new java.io.IOException("reset");
          }
        };

    assertEquals(400, resource.uploadHoldingPhoto(photos.create(), broken).getStatus());
  }

  /** The voter's browser sees these requests: they must not name the identity provider. */
  @Test
  void exposesVendorNeutralNames() {
    assertEquals("identity-verification", new IdentityVerificationResourceFactory().getId());
  }

  @Test
  void verifiesValidTokens() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(200, resource.verifyToken("liveness", token, "proc-1", SECRET).getStatus());
  }

  @Test
  void rejectsInvalidTokens() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(401, resource.verifyToken("liveness", token, "proc-1", "wrong").getStatus());
    assertEquals(401, resource.verifyToken("liveness", "other", "proc-1", SECRET).getStatus());
    assertEquals(401, resource.verifyToken("liveness", null, null, null).getStatus());
  }

  @Test
  void recordsResultsFromTheTokenHeader() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(
        200,
        resource
            .callback("liveness", token, SECRET, OnPremSessionsTest.result("completed", true, JPEG))
            .getStatus());
    assertTrue(sessions.awaitResult(token, 0).orElseThrow().passed());
  }

  @Test
  void readsTheTokenFromTheBodyWithoutHeader() {
    String token = sessions.create("proc-1", SECRET);
    String body =
        OnPremSessionsTest.result("completed", true, JPEG)
            .replace(
                "\"case_id\": \"proc-1\"", "\"case_id\": \"proc-1\", \"token\": \"" + token + "\"");

    assertEquals(200, resource.callback("liveness", null, SECRET, body).getStatus());
    assertTrue(sessions.awaitResult(token, 0).orElseThrow().passed());
  }

  @Test
  void rejectsUnauthorizedAndMalformedCallbacks() {
    String token = sessions.create("proc-1", SECRET);
    String body = OnPremSessionsTest.result("completed", true, JPEG);

    assertEquals(401, resource.callback("liveness", token, "wrong", body).getStatus());
    assertEquals(401, resource.callback("liveness", null, SECRET, body).getStatus());
    assertEquals(400, resource.callback("liveness", token, SECRET, "not json").getStatus());
    assertEquals(400, resource.callback("liveness", token, SECRET, "[]").getStatus());
    assertEquals(400, resource.callback("liveness", token, SECRET, null).getStatus());
    assertEquals(400, resource.callback("liveness", token, SECRET, "null").getStatus());
    assertEquals(
        401,
        resource
            .callback("liveness", token, SECRET, body.replace("proc-1", "other-case"))
            .getStatus());
    assertTrue(sessions.awaitResult(token, 0).isEmpty());
    // Rejected callbacks cannot poison the authorized result or weaken the secret check.
    assertEquals(200, resource.callback("liveness", token, SECRET, body).getStatus());
    assertTrue(sessions.awaitResult(token, 0).orElseThrow().passed());
  }

  // Keycloak's security headers turn a response without a media type into a 500, which the
  // services read as a rejected token or a failed callback.
  @Test
  void everyResponseIsJson() {
    String token = sessions.create("proc-1", SECRET);
    String body = OnPremSessionsTest.result("completed", true, JPEG);

    for (Response response :
        List.of(
            resource.verifyToken("liveness", token, "proc-1", SECRET),
            resource.verifyToken("liveness", token, "proc-1", "wrong"),
            resource.callback("liveness", token, SECRET, body),
            resource.callback("liveness", token, "wrong", body),
            resource.callback("liveness", token, SECRET, "not json"))) {
      assertTrue(
          MediaType.APPLICATION_JSON_TYPE.isCompatible(response.getMediaType()),
          response.getStatus() + " has media type " + response.getMediaType());
    }
  }

  @Test
  void recordsOcrResultsInTheOcrSessions() {
    String token = ocr.create("proc-1", SECRET);
    String body = OnPremSessionsTest.ocrResult("completed", "success", JPEG);

    assertEquals(200, resource.verifyToken("ocr", token, "proc-1", SECRET).getStatus());
    assertEquals(401, resource.verifyToken("liveness", token, "proc-1", SECRET).getStatus());
    assertEquals(200, resource.callback("ocr", token, SECRET, body).getStatus());
    assertTrue(ocr.awaitResult(token, 0).orElseThrow().passed());
  }

  @Test
  void unknownServicesAreNotFound() {
    String token = sessions.create("proc-1", SECRET);

    assertEquals(404, resource.verifyToken("face", token, "proc-1", SECRET).getStatus());
    assertEquals(404, resource.callback("face", token, SECRET, "{}").getStatus());
  }
}
