// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class SegmentedCredentialAssetTest {

  private static final Path SCRIPT =
      Path.of(
          "src/main/resources/theme/sequent.admin-portal/login/resources/js/segmented-credential.js");

  @Test
  void segmentsArePresentationOnlyAndSubmitThroughThePasswordField() throws IOException {
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("realInput.value = segmentInputs.map"));
    assertTrue(script.contains("segmentInput.inputMode = \"numeric\""));
    assertTrue(script.contains("segmentInput.type = \"password\""));
    assertFalse(script.contains("segmentInput.name"));
  }

  @Test
  void configuredLayoutIsBoundedAndInvalidValuesKeepTheStockField() throws IOException {
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("const MAX_GROUPS = 8"));
    assertTrue(script.contains("const MAX_GROUP_SIZE = 12"));
    assertTrue(script.contains("const MAX_TOTAL_SIZE = 64"));
    assertTrue(script.contains("if (!layout)"));
    assertTrue(script.contains("setupVisibilityToggle([realInput]"));
  }
}
