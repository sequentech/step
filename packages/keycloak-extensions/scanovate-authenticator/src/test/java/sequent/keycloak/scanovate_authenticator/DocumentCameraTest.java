// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class DocumentCameraTest {
  @Test
  void theDocumentScanStartsWithTheRearCameraUnlessConfiguredOtherwise() throws ScanovateException {
    assertEquals(DocumentCamera.REAR, DocumentCamera.fromConfig(Map.of()));
    Map<String, String> blank = new HashMap<>();
    blank.put(ScanovateAuthenticatorFactory.DOCUMENT_CAMERA, " ");
    assertEquals(DocumentCamera.REAR, DocumentCamera.fromConfig(blank));
  }

  @Test
  void theConfiguredCameraIsRead() throws ScanovateException {
    assertEquals(
        DocumentCamera.FRONT,
        DocumentCamera.fromConfig(Map.of(ScanovateAuthenticatorFactory.DOCUMENT_CAMERA, "FRONT")));
    assertEquals(
        DocumentCamera.REAR,
        DocumentCamera.fromConfig(Map.of(ScanovateAuthenticatorFactory.DOCUMENT_CAMERA, " rear ")));
  }

  @Test
  void anUnknownCameraIsAConfigurationError() {
    assertThrows(
        ScanovateException.class,
        () ->
            DocumentCamera.fromConfig(
                Map.of(ScanovateAuthenticatorFactory.DOCUMENT_CAMERA, "SIDEWAYS")));
  }

  @Test
  void thePageReadsItInLowerCase() {
    assertEquals("rear", DocumentCamera.REAR.pageValue());
    assertEquals("front", DocumentCamera.FRONT.pageValue());
  }
}
