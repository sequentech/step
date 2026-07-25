// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.text.MessageFormat;
import java.util.Locale;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class MessageBundleTest {

  private static final Path MESSAGES =
      Path.of("src/main/resources/theme/sequent.admin-portal/login/messages");

  @Test
  void catalanApostrophesSurviveKeycloakMessageFormatting() throws IOException {
    Properties messages = load("ca");

    assertEquals(
        "Introduïu el PIN de la vostra carta d'informació electoral.",
        format(messages, "structuredCredentialHint", Locale.forLanguageTag("ca")));
    assertEquals(
        "El nom d'usuari o el PIN són incorrectes.",
        format(messages, "structuredCredentialError", Locale.forLanguageTag("ca")));
  }

  @Test
  void frenchApostrophesSurviveKeycloakMessageFormatting() throws IOException {
    Properties messages = load("fr");
    Locale locale = Locale.forLanguageTag("fr");

    assertEquals(
        "Connectez-vous au Portail d'Administration",
        format(messages, "loginAccountTitle", locale));
    assertEquals("RETOUR À L'APPLICATION", format(messages, "backToApplication", locale));
    assertEquals(
        "Saisissez le code PIN figurant sur votre lettre d'information électorale.",
        format(messages, "structuredCredentialHint", locale));
    assertEquals(
        "Le nom d'utilisateur ou le code PIN est incorrect.",
        format(messages, "structuredCredentialError", locale));
  }

  private static Properties load(String language) throws IOException {
    Properties messages = new Properties();
    try (Reader reader =
        Files.newBufferedReader(MESSAGES.resolve("messages_" + language + ".properties"))) {
      messages.load(reader);
    }
    return messages;
  }

  private static String format(Properties messages, String key, Locale locale) {
    return new MessageFormat(messages.getProperty(key), locale).format(new Object[0]);
  }
}
