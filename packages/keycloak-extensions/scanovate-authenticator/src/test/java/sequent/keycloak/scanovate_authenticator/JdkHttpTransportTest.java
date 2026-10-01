// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.util.Map;
import org.junit.jupiter.api.Test;

class JdkHttpTransportTest {
  // The JDK HTTP client rejects host names with underscores, like docker compose service names
  private static final String INVALID_URL = "http://scanovate_ocr:5040/single_image_ocr";

  @Test
  void invalidUrlsAreReportedAsIoErrors() {
    JdkHttpTransport transport = new JdkHttpTransport();

    assertThrows(IOException.class, () -> transport.postJson(INVALID_URL, Map.of(), "{}"));
  }
}
