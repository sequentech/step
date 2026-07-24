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

class RegisterTemplateTest {

  private static final Path REGISTER_TEMPLATE =
      Path.of("src/main/resources/theme/sequent.admin-portal/login/register.ftl");

  @Test
  void deferredLoginModeEnablesSocialProviders() throws IOException {
    String template = Files.readString(REGISTER_TEMPLATE);

    assertTrue(
        template.contains(
            "displaySocialProviders=(formMode?? && formMode = 'LOGIN' && (social.providers)?has_content)"));
    assertTrue(template.contains("<#elseif section = \"socialProviders\" >"));
    assertTrue(template.contains("id=\"kc-social-providers\""));
    assertTrue(template.contains("msg(\"identity-provider-login-label\")"));
    assertTrue(template.contains("href=\"${p.loginUrl}\""));
    assertTrue(template.contains("${msg(p.displayName)!}"));
  }

  @Test
  void segmentedCredentialIsRestrictedToDeferredLoginMode() throws IOException {
    String template = Files.readString(REGISTER_TEMPLATE);

    assertTrue(
        template.contains(
            "<#assign segmentedCredentialLogin = loginMode && passwordRequired && (realm.attributes['credential-input-policy']!'standard') == 'segmented-numeric'>"));
    assertTrue(template.contains("data-segmented-credential"));
    assertTrue(template.contains("realm.attributes['credential-segment-layout']!'4-4-4-4'"));
    assertFalse(template.contains("?html"));
    assertTrue(template.contains("msg(\"segmentedCredentialError\")"));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/segmented-credential.js\""));
    assertTrue(template.contains("<#if segmentedCredentialLogin>"));
  }

  @Test
  void ordinaryRegistrationKeepsPasswordCreationControls() throws IOException {
    String template = Files.readString(REGISTER_TEMPLATE);

    assertTrue(template.contains("autocomplete=\"new-password\""));
    assertTrue(template.contains("id=\"password-confirm\""));
    assertTrue(template.contains("id=\"password-progress\""));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/passwordVisibility.js\""));
  }
}
