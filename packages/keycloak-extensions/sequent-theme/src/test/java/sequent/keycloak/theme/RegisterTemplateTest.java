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

  private static final Path SOCIAL_PROVIDERS_TEMPLATE =
      Path.of("src/main/resources/theme/sequent.admin-portal/login/social-providers.ftl");

  @Test
  void deferredLoginModeEnablesSocialProviders() throws IOException {
    String template = Files.readString(REGISTER_TEMPLATE);

    assertTrue(
        template.contains(
            "displaySocialProviders=(formMode?? && formMode = 'LOGIN' && (social.providers)?has_content)"));
    assertTrue(template.contains("<#elseif section = \"socialProviders\" >"));
    assertTrue(template.contains("<@socialProviders.render/>"));
  }

  @Test
  void socialProvidersMacroRendersProviderList() throws IOException {
    // Shared with sequent.voting-portal/login/login.ftl - see social-providers.ftl's own header
    // comment. Content assertions live here rather than duplicated per caller.
    String template = Files.readString(SOCIAL_PROVIDERS_TEMPLATE);

    assertTrue(template.contains("id=\"kc-social-providers\""));
    assertTrue(template.contains("msg(\"identity-provider-login-label\")"));
    assertTrue(template.contains("href=\"${p.loginUrl}\""));
    assertTrue(template.contains("${msg(p.displayName)!}"));
    assertTrue(
        template.contains(
            "p.alias != 'digital-certificates' || (realm.attributes['voter-certificate-policy']!'disabled') == 'enabled'"));
  }

  @Test
  void structuredCredentialIsRestrictedToDeferredLoginMode() throws IOException {
    String template = Files.readString(REGISTER_TEMPLATE);

    assertTrue(
        template.contains(
            "<#assign structuredCredentialLogin = loginMode && passwordRequired && ['structured', 'pattern']?seq_contains(realm.attributes['credential-input-policy']!'standard')>"));
    assertTrue(template.contains("data-structured-credential"));
    assertTrue(
        template.contains("realm.attributes['credential-input-pattern']!'dddd-dddd-dddd-dddd'"));
    assertTrue(template.contains("realm.attributes['credential-input-placeholder']!'d'"));
    assertFalse(template.contains("?html"));
    assertTrue(template.contains("msg(\"structuredCredentialError\")"));
    assertTrue(template.contains("data-paste-error=\"${msg('structuredCredentialPasteError')}\""));
    assertTrue(
        template.contains("data-format-error=\"${msg('structuredCredentialFormatError')}\""));
    assertTrue(template.contains("<#if structuredCredentialLogin>inputmode=\"numeric\"</#if>"));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/structured-credential.js\""));
    assertTrue(template.contains("<#if structuredCredentialLogin>"));
    assertTrue(
        template.contains("<#if structuredCredentialLogin>autocomplete=\"current-password\""));
    assertTrue(
        template.contains(
            "<#if structuredCredentialHasError || messagesPerField.existsError('password','password-confirm')>aria-invalid=\"true\"</#if>"));
    assertFalse(
        template.contains(
            "aria-invalid=\"<#if structuredCredentialHasError || messagesPerField.existsError('password','password-confirm')>true</#if>\""));
    assertTrue(
        template.contains(
            "displayMessage=messagesPerField.exists('global') displayRequiredFields=true"));
    assertFalse(template.contains("credentialGlobalError"));
    assertFalse(template.contains("segmentedCredential"));
    assertFalse(template.contains("credential-segment-layout"));
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
