// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import freemarker.cache.FileTemplateLoader;
import freemarker.cache.MultiTemplateLoader;
import freemarker.cache.StringTemplateLoader;
import freemarker.cache.TemplateLoader;
import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.Template;
import freemarker.template.TemplateException;
import freemarker.template.TemplateMethodModelEx;
import java.io.IOException;
import java.io.Reader;
import java.io.StringWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class TemplateSyntaxTest {

  private static final Path THEME_ROOT = Path.of("src/main/resources/theme");

  @Test
  void structuredCredentialTemplatesParseWithHtmlAutoEscaping() throws IOException {
    assertParses(THEME_ROOT.resolve("sequent.voting-portal/login/login.ftl"));
    assertParses(THEME_ROOT.resolve("sequent.admin-portal/login/register.ftl"));
  }

  @Test
  void loginTemplateEscapesHostileRealmPatternAndLocalizationOverrides()
      throws IOException, TemplateException {
    Path child = THEME_ROOT.resolve("sequent.voting-portal/login");
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    Configuration configuration = configuration();
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {
              new FileTemplateLoader(child.toFile()), new FileTemplateLoader(parent.toFile())
            }));

    Map<String, Object> model = baseModel();

    StringWriter rendered = new StringWriter();
    configuration.getTemplate("login.ftl").process(model, rendered);
    String html = rendered.toString();

    assertTrue(html.contains("data-credential-pattern=\"dddd&quot; onfocus=&quot;alert(1)\""));
    assertTrue(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assertTrue(html.contains("data-paste-error=\"paste&quot; onfocus=&quot;alert(2)\""));
    assertTrue(html.contains("data-format-error=\"format&quot; onfocus=&quot;alert(3)\""));
    assertFalse(html.contains("<img src=x onerror=alert(1)>"));
    assertFalse(html.contains("data-credential-pattern=\"dddd\" onfocus="));
  }

  @Test
  void deferredLoginRegistrationTemplateEscapesTheSameHostileConfiguration()
      throws IOException, TemplateException {
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    StringTemplateLoader baseThemeStubs = new StringTemplateLoader();
    baseThemeStubs.putTemplate("intl-tel-input.ftl", "");
    baseThemeStubs.putTemplate("register-commons.ftl", "<#macro termsAcceptance></#macro>");
    Configuration configuration = configuration();
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {new FileTemplateLoader(parent.toFile()), baseThemeStubs}));
    Map<String, Object> model = baseModel();
    model.put("formMode", "LOGIN");
    model.put("passwordRequired", true);
    model.put("hiddenProfileAttributes", List.of());
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                Map.ofEntries(
                    Map.entry("name", "username"),
                    Map.entry("values", List.of()),
                    Map.entry("value", "voter"),
                    Map.entry("annotations", Map.of("showPasswordAfterThis", "true")),
                    Map.entry("group", ""),
                    Map.entry("required", true),
                    Map.entry("multivalued", false),
                    Map.entry("readOnly", false),
                    Map.entry("displayName", "Username"),
                    Map.entry("html5DataAnnotations", Map.of()))),
            "html5DataAnnotations",
            Map.of()));

    StringWriter rendered = new StringWriter();
    configuration.getTemplate("register.ftl").process(model, rendered);
    String html = rendered.toString();

    assertTrue(html.contains("data-credential-pattern=\"dddd&quot; onfocus=&quot;alert(1)\""));
    assertTrue(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assertTrue(html.contains("data-paste-error=\"paste&quot; onfocus=&quot;alert(2)\""));
    assertTrue(html.contains("data-format-error=\"format&quot; onfocus=&quot;alert(3)\""));
    assertTrue(html.contains("inputmode=\"numeric\""));
    assertFalse(html.contains("data-credential-pattern=\"dddd\" onfocus="));
  }

  private static void assertParses(Path path) throws IOException {
    Configuration configuration = configuration();
    try (Reader reader = Files.newBufferedReader(path)) {
      new Template(path.getFileName().toString(), reader, configuration);
    }
  }

  private static Configuration configuration() {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_34);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    configuration.setAutoEscapingPolicy(Configuration.ENABLE_IF_SUPPORTED_AUTO_ESCAPING_POLICY);
    return configuration;
  }

  private static Map<String, Object> baseModel() {
    TemplateMethodModelEx falseMethod = arguments -> false;
    TemplateMethodModelEx message =
        arguments -> {
          String key = arguments.get(0).toString();
          if ("structuredCredentialHint".equals(key)) {
            return "<img src=x onerror=alert(1)>";
          }
          if ("structuredCredentialPasteError".equals(key)) {
            return "paste\" onfocus=\"alert(2)";
          }
          if ("structuredCredentialFormatError".equals(key)) {
            return "format\" onfocus=\"alert(3)";
          }
          return key;
        };
    TemplateMethodModelEx sanitize = arguments -> arguments.get(0).toString();
    TemplateMethodModelEx noFieldError = arguments -> false;
    TemplateMethodModelEx noFieldMessage = arguments -> "";
    TemplateMethodModelEx advancedMessage = arguments -> arguments.get(0).toString();
    return new HashMap<>(
        Map.ofEntries(
            Map.entry(
                "realm",
                Map.ofEntries(
                    Map.entry(
                        "attributes",
                        Map.of(
                            "credential-input-policy",
                            "structured",
                            "credential-input-pattern",
                            "dddd\" onfocus=\"alert(1)")),
                    Map.entry("password", true),
                    Map.entry("registrationAllowed", false),
                    Map.entry("loginWithEmailAllowed", false),
                    Map.entry("registrationEmailAsUsername", false),
                    Map.entry("rememberMe", false),
                    Map.entry("resetPasswordAllowed", false),
                    Map.entry("internationalizationEnabled", false),
                    Map.entry("displayName", "Realm"),
                    Map.entry("defaultLocale", "en"))),
            Map.entry("properties", Map.of("systemVersion", "test", "systemHash", "test-hash")),
            Map.entry(
                "url",
                Map.ofEntries(
                    Map.entry("resourcesPath", "/resources"),
                    Map.entry("resourcesCommonPath", "/common"),
                    Map.entry("ssoLoginInOtherTabsUrl", "/sso"),
                    Map.entry("loginAction", "/login"),
                    Map.entry("registrationAction", "/register"),
                    Map.entry("loginUrl", "/login"))),
            Map.entry("social", Map.of("providers", List.of())),
            Map.entry("locale", Map.of("supported", List.of(), "currentLanguageTag", "en")),
            Map.entry(
                "auth",
                Map.of(
                    "selectedCredential", "",
                    "showUsername", falseMethod,
                    "showResetCredentials", falseMethod,
                    "showTryAnotherWayLink", falseMethod)),
            Map.entry(
                "messagesPerField",
                Map.of(
                    "exists", noFieldError,
                    "existsError", noFieldError,
                    "get", noFieldMessage,
                    "getFirstError", noFieldMessage)),
            Map.entry("msg", message),
            Map.entry("advancedMsg", advancedMessage),
            Map.entry("kcSanitize", sanitize),
            Map.entry("login", Map.of()),
            Map.entry("scripts", List.of())));
  }
}
