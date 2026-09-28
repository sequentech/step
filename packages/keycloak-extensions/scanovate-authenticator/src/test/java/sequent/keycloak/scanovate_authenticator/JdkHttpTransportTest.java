// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.HttpTransport.MultipartPart;

class JdkHttpTransportTest {
  // The JDK HTTP client rejects host names with underscores, like docker compose service names
  private static final String INVALID_URL = "http://mock_server:8500/auth/token";

  @Test
  void invalidUrlsAreReportedAsIoErrors() {
    JdkHttpTransport transport = new JdkHttpTransport();

    assertThrows(IOException.class, () -> transport.get(INVALID_URL, Map.of()));
    assertThrows(IOException.class, () -> transport.postJson(INVALID_URL, Map.of(), "{}"));
    assertThrows(
        IOException.class,
        () ->
            transport.postMultipart(
                INVALID_URL,
                Map.of(),
                List.of(new MultipartPart("a", "a.jpg", "image/jpeg", new byte[] {1}))));
  }

  @Test
  void multipartBodyIsEncodedPartByPart() {
    byte[] binary = {(byte) 0xFF, (byte) 0xD8, 0x0D, 0x0A, 0x00};
    List<MultipartPart> parts =
        List.of(
            new MultipartPart("front_image", "front_image.jpg", "image/jpeg", binary),
            new MultipartPart("scan_video", "scan_video.webm", "video/webm", new byte[] {7}));

    ByteArrayOutputStream expected = new ByteArrayOutputStream();
    expected.writeBytes(
        ("--b0undary\r\n"
                + "Content-Disposition: form-data; name=\"front_image\";"
                + " filename=\"front_image.jpg\"\r\n"
                + "Content-Type: image/jpeg\r\n"
                + "\r\n")
            .getBytes(StandardCharsets.US_ASCII));
    expected.writeBytes(binary);
    expected.writeBytes(
        ("\r\n--b0undary\r\n"
                + "Content-Disposition: form-data; name=\"scan_video\";"
                + " filename=\"scan_video.webm\"\r\n"
                + "Content-Type: video/webm\r\n"
                + "\r\n")
            .getBytes(StandardCharsets.US_ASCII));
    expected.write(7);
    expected.writeBytes("\r\n--b0undary--\r\n".getBytes(StandardCharsets.US_ASCII));

    assertArrayEquals(expected.toByteArray(), JdkHttpTransport.multipartBody(parts, "b0undary"));
  }

  @Test
  void emptyMultipartBodyOnlyHasTheClosingBoundary() {
    assertArrayEquals(
        "--b--\r\n".getBytes(StandardCharsets.US_ASCII),
        JdkHttpTransport.multipartBody(List.of(), "b"));
  }

  @Test
  void multipartHeadersCannotBeInjected() {
    for (MultipartPart part :
        List.of(
            new MultipartPart("a\"; x=\"", "a.jpg", "image/jpeg", new byte[0]),
            new MultipartPart("a", "a.jpg\r\nX-Injected: 1", "image/jpeg", new byte[0]),
            new MultipartPart("a", "a.jpg", "image/jpeg\nX-Injected: 1", new byte[0]))) {
      assertThrows(
          IllegalArgumentException.class, () -> JdkHttpTransport.multipartBody(List.of(part), "b"));
    }
  }

  @Test
  void boundariesAreRandomAndValid() {
    String first = JdkHttpTransport.newBoundary();
    String second = JdkHttpTransport.newBoundary();

    assertNotEquals(first, second);
    // RFC 2046: at most 70 characters from a restricted set
    assertTrue(first.matches("[0-9A-Za-z'()+_,./:=?-]{1,70}"), first);
  }
}
