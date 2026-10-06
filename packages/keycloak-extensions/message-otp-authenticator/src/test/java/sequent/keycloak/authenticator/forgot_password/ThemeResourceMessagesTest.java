// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class ThemeResourceMessagesTest {

  private static final Path MESSAGES = Path.of("src/main/resources/theme-resources/messages");

  @Test
  void profileDisplayNameAliasesUseTheExistingFieldTranslations() throws IOException {
    for (String language : List.of("ca", "en", "es", "eu", "gl", "tl")) {
      Properties messages = new Properties();
      try (Reader reader =
          Files.newBufferedReader(MESSAGES.resolve("messages_" + language + ".properties"))) {
        messages.load(reader);
      }

      assertEquals(
          messages.getProperty("dateOfBirth"),
          messages.getProperty("profile.attributes.dateOfBirth"),
          language);
      assertEquals(
          messages.getProperty("nationalId"),
          messages.getProperty("profile.attributes.nationalId"),
          language);
    }
  }
}
