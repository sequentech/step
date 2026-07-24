// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.Template;
import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class TemplateSyntaxTest {

  private static final Path THEME_ROOT = Path.of("src/main/resources/theme");

  @Test
  void segmentedCredentialTemplatesParse() throws IOException {
    assertParses(THEME_ROOT.resolve("sequent.voting-portal/login/login.ftl"));
    assertParses(THEME_ROOT.resolve("sequent.admin-portal/login/register.ftl"));
  }

  private static void assertParses(Path path) throws IOException {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_34);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    try (Reader reader = Files.newBufferedReader(path)) {
      new Template(path.getFileName().toString(), reader, configuration);
    }
  }
}
