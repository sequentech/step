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

class LoginTemplateTest {

  private static final Path LOGIN_TEMPLATE =
      Path.of("src/main/resources/theme/sequent.voting-portal/login/login.ftl");

  @Test
  void structuredCredentialIsOptInAndConfigurable() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(
        template.contains(
            "<#assign structuredCredential = (realm.attributes['credential-input-policy']!'standard') == 'structured'>"));
    assertTrue(template.contains("data-structured-credential"));
    assertTrue(
        template.contains("realm.attributes['credential-input-pattern']!'dddd-dddd-dddd-dddd'"));
    assertTrue(template.contains("realm.attributes['credential-input-placeholder']!'d'"));
    assertFalse(template.contains("?html"));
    assertTrue(template.contains("msg(\"structuredCredentialError\")"));
    assertTrue(template.contains("data-paste-error=\"${msg('structuredCredentialPasteError')}\""));
    assertTrue(
        template.contains("data-format-error=\"${msg('structuredCredentialFormatError')}\""));
    assertTrue(template.contains("<#if structuredCredential>inputmode=\"numeric\"</#if>"));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/structured-credential.js\""));
    assertFalse(template.contains("segmentedCredential"));
    assertFalse(template.contains("credential-segment-layout"));
  }

  @Test
  void standardPasswordFieldRemainsTheFallback() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(template.contains("name=\"password\" type=\"password\""));
    assertTrue(
        template.contains("autocomplete=\"<#if structuredCredential>username<#else>off</#if>\""));
    assertTrue(
        template.contains(
            "autocomplete=\"<#if structuredCredential>current-password<#else>off</#if>\""));
    assertTrue(
        template.contains(
            "<#if structuredCredentialHasError || credentialFieldError>aria-invalid=\"true\"</#if>"));
    assertFalse(template.contains("aria-invalid=\"<#if"));
    assertTrue(template.contains("src=\"${url.resourcesPath}/js/passwordVisibility.js\""));
    assertTrue(template.contains("<#if structuredCredential>"));
    assertTrue(template.contains("<#else>"));
  }

  @Test
  void structuredCredentialDoesNotMaskOperationalGlobalErrors() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertFalse(template.contains("credentialGlobalError"));
    assertTrue(
        template.contains(
            "<#assign structuredCredentialHasError = structuredCredential && credentialFieldError>"));
  }

  @Test
  void prefillsTheUsernameFromTheLoginHint() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(template.contains("name=\"username\" value=\"${(login.username!'')}\""));
  }

  @Test
  void locksThePrefilledUsernameWhenTheRealmPolicyIsReadOnly() throws IOException {
    String template = Files.readString(LOGIN_TEMPLATE);

    assertTrue(
        template.contains(
            "<#assign usernamePrefilled = (login.username!'')?has_content && !login.rememberMe??>"));
    assertTrue(
        template.contains(
            "(realm.attributes['loginHintUsernamePolicy']!'EDITABLE') == 'READ_ONLY'"));
    assertTrue(template.contains("<#if usernameReadOnly>readonly</#if>"));
  }
}
