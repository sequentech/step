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
