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

class StructuredCredentialAssetTest {

  private static final Path SCRIPT =
      Path.of(
          "src/main/resources/theme/sequent.admin-portal/login/resources/js/structured-credential.js");

  private static final Path STYLES =
      Path.of("src/main/resources/theme/sequent.admin-portal/login/resources/css/custom.css");

  @Test
  void oneControlledInputSubmitsThroughTheOrdinaryPasswordField() throws IOException {
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("document.createElement(\"input\")"));
    assertTrue(script.contains("displayInput.inputMode = \"numeric\""));
    assertTrue(script.contains("displayInput.type = \"text\""));
    assertTrue(script.contains("displayInput.autocomplete = \"current-password\""));
    assertTrue(script.contains("usernameInput.autocomplete = \"username\""));
    assertTrue(script.contains("displayInput.setAttribute(\"aria-required\", \"true\")"));
    assertTrue(script.contains("realInput.value = digits.map"));
    assertTrue(script.contains("realInput.type = \"hidden\""));
    assertTrue(script.contains("prefilledValue.length <= pattern.totalSize"));
    assertFalse(script.contains("realInput.value.replace(/[^0-9]/g"));
    assertFalse(script.contains("displayInput.name"));
    assertFalse(script.contains("segmentInputs"));
  }

  @Test
  void patternPolicyIsAnAliasForTheExistingStructuredPasswordInput() throws IOException {
    String login =
        Files.readString(Path.of("src/main/resources/theme/sequent.voting-portal/login/login.ftl"));
    String register =
        Files.readString(
            Path.of("src/main/resources/theme/sequent.admin-portal/login/register.ftl"));
    assertTrue(login.contains("['structured', 'pattern']?seq_contains"));
    assertTrue(register.contains("['structured', 'pattern']?seq_contains"));
    assertTrue(login.contains("name=\"password\" type=\"password\""));
    assertTrue(login.contains("data-credential-pattern="));
    assertFalse(login.contains("name=\"secret\""));
  }

  @Test
  void autofocusMovesFromTheHiddenRealInputToTheVisibleOne() throws IOException {
    // credential-field-position=FIRST puts autofocus on the real input, which this widget hides.
    // Without the transfer the page opens with nothing focused.
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("realInput.hasAttribute(\"autofocus\")"));
    assertTrue(script.contains("realInput.removeAttribute(\"autofocus\")"));
    assertTrue(script.contains("displayInput.focus()"));
  }

  @Test
  void patternIsBoundedAndMalformedValuesKeepTheStockField() throws IOException {
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("const DIGIT_TOKEN = \"d\""));
    assertTrue(script.contains("const MAX_GROUPS = 8"));
    assertTrue(script.contains("const MAX_GROUP_SIZE = 12"));
    assertTrue(script.contains("const MAX_TOTAL_SIZE = 64"));
    assertTrue(script.contains("if (!pattern)"));
    assertTrue(script.contains("setupNativeVisibilityToggle(realInput, toggle)"));
  }

  @Test
  void keyboardMaskingAndPasteBehaviorsAreWired() throws IOException {
    String script = Files.readString(SCRIPT);

    assertTrue(script.contains("setSelectionRange"));
    assertTrue(script.contains("ArrowLeft"));
    assertTrue(script.contains("ArrowRight"));
    assertTrue(script.contains("beforeinput"));
    assertTrue(script.contains("paste"));
    assertTrue(script.contains("digits.fill(null, start, clearEnd)"));
    assertTrue(script.contains("REVEAL_DURATION_MS"));
    assertTrue(script.contains("visibilitychange"));
    assertTrue(script.contains("window.addEventListener(\"pagehide\", hideCredential)"));
    assertTrue(script.contains("toggle.tabIndex = originalTabIndex"));
    assertTrue(script.contains("event.target === usernameInput"));
    assertTrue(script.contains("usernameInput.removeAttribute(\"aria-invalid\")"));
  }

  @Test
  void cssUsesOneBorderedComponentAndContainsNoMultiBoxRules() throws IOException {
    String styles = Files.readString(STYLES);

    assertTrue(
        styles.contains(
            "[data-structured-credential][data-structured-credential-enhanced=\"true\"]"));
    assertTrue(styles.contains(".structured-credential__input"));
    assertTrue(styles.contains("[data-structured-credential-toggle]"));
    assertTrue(styles.contains("border: 1px solid var(--pf-global--palette--black-500)"));
    assertTrue(styles.contains("line-height: 24px"));
    assertTrue(styles.contains("padding-block: 10px"));
    assertTrue(styles.contains("[data-structured-credential-toggle]:focus-visible"));
    assertTrue(styles.contains("@media (forced-colors: active)"));
    assertTrue(styles.contains("[data-structured-credential-toggle]::after"));
    assertFalse(styles.contains("line-height: 44px"));
    assertFalse(styles.contains(".segmented-credential__segment"));
    assertFalse(styles.contains("--segmented-credential-gap"));
  }
}
