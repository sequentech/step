// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertEquals;
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
    assertTrue(
        html.contains("data-credential-input-placeholder=\"#&quot; onfocus=&quot;alert(4)\""));
    assertTrue(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assertTrue(html.contains("data-paste-error=\"paste&quot; onfocus=&quot;alert(2)\""));
    assertTrue(html.contains("data-format-error=\"format&quot; onfocus=&quot;alert(3)\""));
    assertFalse(html.contains("<img src=x onerror=alert(1)>"));
    assertFalse(html.contains("data-credential-pattern=\"dddd\" onfocus="));
    assertFalse(html.contains("data-credential-input-placeholder=\"#\" onfocus="));
  }

  @Test
  void socialProvidersFilterDigitalCertificatesUnlessRealmOptsIn()
      throws IOException, TemplateException {
    List<Map<String, Object>> providers =
        List.of(
            Map.of(
                "alias", "google",
                "loginUrl", "/broker/google",
                "iconClasses", "",
                "displayName", "Google"),
            Map.of(
                "alias", "digital-certificates",
                "loginUrl", "/broker/digital-certificates",
                "iconClasses", "",
                "displayName", "Digital Certificate"));

    Map<String, Object> disabledModel = baseModel("standard");
    disabledModel.put("social", Map.of("providers", providers));
    String disabledHtml = renderVotingPortalLogin(disabledModel);

    assertTrue(disabledHtml.contains("id=\"social-google\""));
    assertFalse(disabledHtml.contains("id=\"social-digital-certificates\""));

    Map<String, Object> enabledModel = baseModel("standard");
    enabledModel.put("social", Map.of("providers", providers));
    enabledModel.put(
        "realm",
        Map.ofEntries(
            Map.entry(
                "attributes",
                Map.of(
                    "credential-input-policy", "standard",
                    "credential-input-pattern", "dddd-dddd-dddd-dddd",
                    "voter-certificate-policy", "enabled")),
            Map.entry("password", true),
            Map.entry("registrationAllowed", false),
            Map.entry("loginWithEmailAllowed", false),
            Map.entry("registrationEmailAsUsername", false),
            Map.entry("rememberMe", false),
            Map.entry("resetPasswordAllowed", false),
            Map.entry("internationalizationEnabled", false),
            Map.entry("displayName", "Realm"),
            Map.entry("defaultLocale", "en")));
    String enabledHtml = renderVotingPortalLogin(enabledModel);

    assertTrue(enabledHtml.contains("id=\"social-google\""));
    assertTrue(enabledHtml.contains("id=\"social-digital-certificates\""));
  }

  @Test
  void multiAttributeLoginSupportsStructuredCredentialToggle()
      throws IOException, TemplateException {
    Map<String, Object> structuredModel = baseModel("structured");
    structuredModel.put("matchAttributes", List.of("dateOfBirth"));
    structuredModel.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
    String structuredHtml = renderVotingPortalLogin(structuredModel);

    assertTrue(structuredHtml.contains("type=\"date\" id=\"dateOfBirth\" name=\"dateOfBirth\""));
    assertTrue(structuredHtml.contains("data-structured-credential"));
    assertTrue(structuredHtml.contains("src=\"/resources/js/structured-credential.js\""));
    assertFalse(structuredHtml.contains("src=\"/resources/js/passwordVisibility.js\""));

    Map<String, Object> standardModel = baseModel("standard");
    standardModel.put("matchAttributes", List.of("dateOfBirth"));
    standardModel.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
    String standardHtml = renderVotingPortalLogin(standardModel);

    assertTrue(standardHtml.contains("type=\"date\" id=\"dateOfBirth\" name=\"dateOfBirth\""));
    assertFalse(standardHtml.contains("data-structured-credential"));
    assertTrue(standardHtml.contains("src=\"/resources/js/passwordVisibility.js\""));
    assertFalse(standardHtml.contains("src=\"/resources/js/structured-credential.js\""));
  }

  @Test
  void multiAttributeDateInputsHonorConfiguredMaxInBothPortals()
      throws IOException, TemplateException {
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> defaultModel = baseModel("standard");
      defaultModel.put("matchAttributes", List.of("dateOfBirth"));
      defaultModel.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
      String defaultHtml = renderLogin(portal, defaultModel);

      assertTrue(defaultHtml.contains("max=\"9999-12-31\""));

      Map<String, Object> configuredModel = baseModel("standard");
      configuredModel.put("matchAttributes", List.of("dateOfBirth"));
      configuredModel.put(
          "profile",
          profileWithAttributes(
              mockAttribute(
                  "dateOfBirth",
                  "${dateOfBirth}",
                  Map.of("inputType", "html5-date", "inputTypeMax", "2020-01-01"))));
      String configuredHtml = renderLogin(portal, configuredModel);

      assertTrue(configuredHtml.contains("max=\"2020-01-01\""));
      assertFalse(configuredHtml.contains("max=\"9999-12-31\""));
    }
  }

  @Test
  void multiAttributeLoginKeepsFocusTabOrderAndAutocompleteOff()
      throws IOException, TemplateException {
    // Regression: these three came from login.ftl's own input tag before the matchAttributes loop
    // was moved onto user-profile-commons.ftl's shared macros. autocomplete="off" in particular
    // keeps a shared device from suggesting the previous voter's values.
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> model = baseModel("standard");
      model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
      model.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date")),
              mockAttribute("nationalId", "${nationalId}", Map.of())));
      String html = renderLogin(portal, model);
      String normalized = html.replaceAll("\\s+", " ");

      assertTrue(normalized.contains("id=\"dateOfBirth\""));
      assertTrue(normalized.contains("autofocus"));
      assertTrue(normalized.contains("tabindex=\"1\""));
      assertTrue(normalized.contains("tabindex=\"2\""));
      assertTrue(normalized.contains("autocomplete=\"off\""));
    }
  }

  @Test
  void multiAttributeLoginOnlyAutofocusesTheFirstField() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date")),
            mockAttribute("nationalId", "${nationalId}", Map.of())));
    String html = renderVotingPortalLogin(model);

    assertEquals(1, html.split("autofocus", -1).length - 1);
  }

  @Test
  void telInputWidgetInitialisesAfterTheFieldsItUpgrades() throws IOException, TemplateException {
    // login.ftl emits the widget assets above its matchAttributes loop, so a synchronous
    // querySelectorAll would run before the tel inputs exist and silently upgrade nothing -
    // leaving the raw local number to be submitted instead of the normalised international one.
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("mobile"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("mobile", "${mobile}", Map.of("inputType", "html5-tel"))));
    String html = renderVotingPortalLogin(model);

    assertTrue(html.contains("type=\"tel\""));
    assertTrue(
        html.indexOf("querySelectorAll(\"input[type='tel']\")") < html.indexOf("type=\"tel\""));
    assertTrue(html.contains("addEventListener('DOMContentLoaded'"));
  }

  @Test
  void registerTemplateDoesNotGainFocusOrTabOrderFromTheSharedMacros()
      throws IOException, TemplateException {
    // The autofocus/tabindex/autocomplete macro parameters are opt-in; register.ftl must keep
    // rendering exactly as it did before login.ftl started sharing these macros.
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))),
            "html5DataAnnotations",
            Map.of()));
    String html = renderRegister(model);

    assertFalse(html.contains("autofocus"));
    assertFalse(html.contains("tabindex"));
  }

  @Test
  void multiAttributeLoginMarksUndeclaredAttributeRequiredWhenEnabled()
      throws IOException, TemplateException {
    // The authenticator keeps an attribute with no User Profile declaration mandatory, so the
    // fallback field must not present itself as optional.
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("nationalId"));
    model.put("honorUserProfileRequired", true);
    model.put("profile", profileWithAttributes());
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    assertTrue(normalized.contains("</label> *"));
    assertTrue(normalized.contains("required"));
    assertTrue(html.contains("requiredFields"));
  }

  @Test
  void multiAttributeLoginRendersConfiguredHelperText() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth",
                "${dateOfBirth}",
                Map.of(
                    "inputType", "html5-date",
                    "inputHelperTextBefore", "Enter as printed on your ID",
                    "inputHelperTextAfter", "Format: DD/MM/YYYY"))));
    String html = renderVotingPortalLogin(model);

    assertTrue(
        html.contains(
            "id=\"form-help-text-before-dateOfBirth\" aria-live=\"polite\">Enter as printed on your"
                + " ID</div>"));
    assertTrue(
        html.contains(
            "id=\"form-help-text-after-dateOfBirth\" aria-live=\"polite\">Format:"
                + " DD/MM/YYYY</div>"));
  }

  @Test
  void multiAttributeLoginOmitsHelperTextWhenNotConfigured() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
    String html = renderVotingPortalLogin(model);

    assertFalse(html.contains("form-help-text-before-dateOfBirth"));
    assertFalse(html.contains("form-help-text-after-dateOfBirth"));
  }

  @Test
  void multiAttributeLoginFallsBackToPlainTextFieldWhenNotInUserProfile()
      throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("nationalId"));
    model.put("profile", profileWithAttributes());
    String html = renderVotingPortalLogin(model);

    assertTrue(html.contains("id=\"nationalId\""));
    assertTrue(html.contains("name=\"nationalId\""));
    assertTrue(html.contains("type=\"text\""));
  }

  @Test
  void multiAttributeLoginMarksRequiredAttributeWhenEnabled()
      throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put("honorUserProfileRequired", true);
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true)));
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    assertTrue(normalized.contains("aria-invalid=\"\" required"));
    assertTrue(html.contains("requiredFields"));
    assertTrue(normalized.contains("</label> *"));
  }

  @Test
  void multiAttributeLoginOmitsRequiredMarkingWhenNotEnabled()
      throws IOException, TemplateException {
    // dateOfBirth is required=true in the realm's User Profile, but honorUserProfileRequired is
    // not set - neither the HTML5 required attribute nor the "*" marker should appear, since
    // matching-mandatoriness here comes entirely from matchAttributes, not User Profile.
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true)));
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    assertFalse(normalized.contains("aria-invalid=\"\" required"));
    assertFalse(html.contains("requiredFields"));
    assertFalse(normalized.contains("</label> *"));
  }

  @Test
  void multiAttributeLoginDoesNotMarkNonRequiredAttributeEvenWhenEnabled()
      throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put("honorUserProfileRequired", true);
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), false)));
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    assertFalse(normalized.contains("aria-invalid=\"\" required"));
    assertFalse(html.contains("requiredFields"));
    assertFalse(normalized.contains("</label> *"));
  }

  @Test
  void deferredLoginRegistrationTemplateEscapesTheSameHostileConfiguration()
      throws IOException, TemplateException {
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    StringTemplateLoader baseThemeStubs = new StringTemplateLoader();
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
    assertTrue(
        html.contains("data-credential-input-placeholder=\"#&quot; onfocus=&quot;alert(4)\""));
    assertTrue(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assertTrue(html.contains("data-paste-error=\"paste&quot; onfocus=&quot;alert(2)\""));
    assertTrue(html.contains("data-format-error=\"format&quot; onfocus=&quot;alert(3)\""));
    assertTrue(html.contains("inputmode=\"numeric\""));
    assertFalse(html.contains("data-credential-pattern=\"dddd\" onfocus="));
    assertFalse(html.contains("data-credential-input-placeholder=\"#\" onfocus="));
  }

  private static void assertParses(Path path) throws IOException {
    Configuration configuration = configuration();
    try (Reader reader = Files.newBufferedReader(path)) {
      new Template(path.getFileName().toString(), reader, configuration);
    }
  }

  private static String renderVotingPortalLogin(Map<String, Object> model)
      throws IOException, TemplateException {
    Path child = THEME_ROOT.resolve("sequent.voting-portal/login");
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    Configuration configuration = configuration();
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {
              new FileTemplateLoader(child.toFile()), new FileTemplateLoader(parent.toFile())
            }));
    StringWriter rendered = new StringWriter();
    configuration.getTemplate("login.ftl").process(model, rendered);
    return rendered.toString();
  }

  private static String renderRegister(Map<String, Object> model)
      throws IOException, TemplateException {
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    StringTemplateLoader baseThemeStubs = new StringTemplateLoader();
    baseThemeStubs.putTemplate("register-commons.ftl", "<#macro termsAcceptance></#macro>");
    Configuration configuration = configuration();
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {new FileTemplateLoader(parent.toFile()), baseThemeStubs}));
    StringWriter rendered = new StringWriter();
    configuration.getTemplate("register.ftl").process(model, rendered);
    return rendered.toString();
  }

  private static String renderLogin(String portal, Map<String, Object> model)
      throws IOException, TemplateException {
    Path child = THEME_ROOT.resolve(portal + "/login");
    Path parent = THEME_ROOT.resolve("sequent.admin-portal/login");
    Configuration configuration = configuration();
    if (child.equals(parent)) {
      configuration.setTemplateLoader(new FileTemplateLoader(parent.toFile()));
    } else {
      configuration.setTemplateLoader(
          new MultiTemplateLoader(
              new TemplateLoader[] {
                new FileTemplateLoader(child.toFile()), new FileTemplateLoader(parent.toFile())
              }));
    }
    StringWriter rendered = new StringWriter();
    configuration.getTemplate("login.ftl").process(model, rendered);
    return rendered.toString();
  }

  /** Mimics {@code AbstractUserProfileBean}'s {@code attributesByName} shape - see LoginBean. */
  private static Map<String, Object> profileWithAttributes(Map<String, Object>... attributes) {
    Map<String, Object> attributesByName = new HashMap<>();
    for (Map<String, Object> attribute : attributes) {
      attributesByName.put((String) attribute.get("name"), attribute);
    }
    return Map.of("attributesByName", attributesByName);
  }

  /**
   * Mimics one {@code AbstractUserProfileBean.Attribute} - the fields user-profile-commons.ftl
   * reads.
   */
  private static Map<String, Object> mockAttribute(
      String name, String displayName, Map<String, Object> annotations) {
    return mockAttribute(name, displayName, annotations, false);
  }

  private static Map<String, Object> mockAttribute(
      String name, String displayName, Map<String, Object> annotations, boolean required) {
    Map<String, Object> attribute = new HashMap<>();
    attribute.put("name", name);
    attribute.put("displayName", displayName);
    attribute.put("required", required);
    attribute.put("readOnly", false);
    attribute.put("multivalued", false);
    attribute.put("value", "");
    attribute.put("values", List.of());
    attribute.put("annotations", annotations);
    attribute.put("html5DataAnnotations", Map.of());
    return attribute;
  }

  private static Configuration configuration() {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_34);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    configuration.setAutoEscapingPolicy(Configuration.ENABLE_IF_SUPPORTED_AUTO_ESCAPING_POLICY);
    return configuration;
  }

  private static Map<String, Object> baseModel() {
    return baseModel("structured");
  }

  private static Map<String, Object> baseModel(String credentialInputPolicy) {
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
                            credentialInputPolicy,
                            "credential-input-pattern",
                            "dddd\" onfocus=\"alert(1)",
                            "credential-input-placeholder",
                            "#\" onfocus=\"alert(4)")),
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
