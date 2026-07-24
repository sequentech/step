// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class LoginTemplateTest {

  private static final Path LOGIN_TEMPLATE =
      Path.of("src/main/resources/theme/sequent.voting-portal/login/login.ftl");

  @Test
  void segmentedCredentialIsOptInAndConfigurable() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(
        template.contains(
            "<#assign segmentedCredential = (realm.attributes['credential-input-policy']!'standard') == 'segmented-numeric'>"));
    assertTrue(template.contains("data-segmented-credential"));
    assertTrue(template.contains("realm.attributes['credential-segment-layout']!'4-4-4-4'"));
    assertTrue(template.contains("msg(\"segmentedCredentialError\")"));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/segmented-credential.js\""));
  }

  @Test
  void standardPasswordFieldRemainsTheFallback() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(template.contains("name=\"password\" type=\"password\""));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/passwordVisibility.js\""));
    assertTrue(template.contains("<#if segmentedCredential>"));
    assertTrue(template.contains("<#else>"));
  }
}
