// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertTrue;

import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.Template;
import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class MessageOtpTemplateTest {
  private static final Path TEMPLATE =
      Path.of("src/main/resources/theme-resources/templates/message-otp.login.ftl");

  @Test
  void theCodePageParsesAndOffersTheChannelControls() throws IOException {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_32);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    try (Reader reader = Files.newBufferedReader(TEMPLATE)) {
      new Template("message-otp.login.ftl", reader, configuration);
    }
    String template = Files.readString(TEMPLATE);
    assertTrue(template.contains("name=\"channel\" value=\"${option}\""));
    assertTrue(template.contains("name=\"messengerStatus\" value=\"true\""));
    assertTrue(template.contains("msg(\"messageOtp.delivery.unknown\")"));
    assertTrue(template.contains("msg(\"messageOtp.otherWay.title\")"));
  }
}
