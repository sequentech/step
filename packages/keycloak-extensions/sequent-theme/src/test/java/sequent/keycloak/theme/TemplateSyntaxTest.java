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
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

class TemplateSyntaxTest {

  private static final Path THEME_ROOT = Path.of("src/main/resources/theme");

  @Test
  void structuredCredentialTemplatesParseWithHtmlAutoEscaping() throws IOException {
    assertParses(THEME_ROOT.resolve("sequent.voting-portal/login/login.ftl"));
    assertParses(THEME_ROOT.resolve("sequent.admin-portal/login/register.ftl"));
  }

  @Test
  void patternAliasRendersExactlyTheExistingStructuredPasswordForm()
      throws IOException, TemplateException {
    assertEquals(
        renderVotingPortalLogin(baseModel("structured")),
        renderVotingPortalLogin(baseModel("pattern")));
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
  void structuredMultiAttributeFailureUsesTheAuthenticatorError()
      throws IOException, TemplateException {
    TemplateMethodModelEx hasError = arguments -> true;
    TemplateMethodModelEx noMessage = arguments -> "";
    TemplateMethodModelEx genericError = arguments -> "invalidCredentialsMessage";
    Map<String, Object> model = baseModel("structured");
    model.put(
        "messagesPerField",
        Map.of(
            "exists", hasError,
            "existsError", hasError,
            "get", noMessage,
            "getFirstError", genericError));
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));

    String html = renderVotingPortalLogin(model);

    assertTrue(html.contains("invalidCredentialsMessage"));
    assertFalse(html.contains(">structuredCredentialError<"));
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
  void multiAttributeLoginKeepsFocusAndAutocompleteOff() throws IOException, TemplateException {
    // autofocus and autocomplete="off" came from login.ftl's own input tag before the
    // matchAttributes loop moved onto the shared macros. autocomplete="off" keeps a shared device
    // from suggesting the previous voter's values.
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
      assertTrue(normalized.contains("autocomplete=\"off\""));
    }
  }

  @Test
  void multiAttributeLoginEmitsNoTabindexAndRendersInMatchOrder()
      throws IOException, TemplateException {
    // These controls are natively focusable and rendered in the order they should be tabbed, so
    // source order alone gives the correct sequence. A positive tabindex would lift them out of
    // the document's natural order - ahead of the header's locale link, and ahead of the
    // credential field - which is the failure mode WCAG 2.4.3 describes.
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> model = baseModel("standard");
      model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
      model.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date")),
              mockAttribute("nationalId", "${nationalId}", Map.of())));
      String html = renderLogin(portal, model);

      assertFalse(html.contains("tabindex"));
      assertTrue(html.indexOf("id=\"dateOfBirth\"") < html.indexOf("id=\"nationalId\""));
      assertTrue(html.indexOf("id=\"nationalId\"") < html.indexOf("id=\"password\""));
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
    // The bundled build makes number normalization available synchronously. DOMContentLoaded still
    // defers the query until after login.ftl has emitted the tel input.
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
    assertTrue(html.contains("intlTelInputWithUtils.min.js"));
    assertFalse(html.contains("utilsScript"));
    assertFalse(html.contains("input.id = id + \"-input\""));
    assertTrue(html.contains("input.name = id + \"-input\""));
  }

  @Test
  void telPatternIsCheckedAgainstTheNormalizedNumber() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("mobile"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "mobile",
                "${mobile}",
                Map.of("inputType", "html5-tel", "inputTypePattern", "^\\+\\d+$"))));

    String html = renderVotingPortalLogin(model);

    assertTrue(inputTagFor(html, "mobile").contains("pattern=\"^\\+\\d+$\""));
    assertTrue(html.contains("input.removeAttribute(\"pattern\")"));
    assertTrue(html.contains("patternProbe.value = phoneInput.getNumber()"));
    assertTrue(html.contains("input.setCustomValidity(patternProbe.validationMessage)"));
  }

  @Test
  void multiAttributeLoginSkipsUnusedPhoneAssets() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("nationalId"));
    model.put(
        "profile", profileWithAttributes(mockAttribute("nationalId", "${nationalId}", Map.of())));

    String html = renderVotingPortalLogin(model);

    assertFalse(html.contains("intlTelInput"));
    assertFalse(html.contains("intlTelInput.css"));
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
    assertFalse(html.contains("autocomplete=\"off\""));
  }

  @Test
  void registrationKeepsUserProfileRequiredValidation() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", false);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute(
                    "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true)),
            "html5DataAnnotations",
            Map.of()));

    String html = renderRegister(model);

    assertTrue(inputTagFor(html, "dateOfBirth").contains("required"));
  }

  @Test
  void controlsReferenceTheirHelperAndErrorText() throws IOException, TemplateException {
    TemplateMethodModelEx dateError =
        arguments -> arguments.size() == 1 && "dateOfBirth".equals(arguments.get(0).toString());
    TemplateMethodModelEx errorMessage = arguments -> "Invalid date";
    Map<String, Object> model = baseModel("standard");
    model.put(
        "messagesPerField",
        Map.of(
            "exists", dateError,
            "existsError", dateError,
            "get", errorMessage,
            "getFirstError", errorMessage));
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth",
                "${dateOfBirth}",
                Map.of(
                    "inputType",
                    "html5-date",
                    "inputHelperTextBefore",
                    "Before",
                    "inputHelperTextAfter",
                    "After"))));

    String html = renderVotingPortalLogin(model);
    String input = inputTagFor(html, "dateOfBirth");

    assertTrue(
        input.contains(
            "aria-describedby=\"form-help-text-before-dateOfBirth "
                + "input-error-dateOfBirth form-help-text-after-dateOfBirth\""));
    assertTrue(html.contains("id=\"form-help-text-before-dateOfBirth\""));
    assertTrue(html.contains("id=\"input-error-dateOfBirth\""));
    assertTrue(html.contains("id=\"form-help-text-after-dateOfBirth\""));
  }

  @Test
  void requiredCheckboxGroupAcceptsAnyCheckedOption() throws IOException, TemplateException {
    Map<String, Object> channels =
        mockAttributeWithOptions(
            "channels", Map.of("inputType", "multiselect-checkboxes"), List.of("sms", "email"));
    channels.put("required", true);
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", false);
    model.put(
        "profile",
        Map.of(
            "attributes", List.of(channels),
            "html5DataAnnotations", Map.of()));

    String html = renderRegister(model);

    assertTrue(
        inputTagFor(html, "channels-sms").contains("data-required-checkbox-group=\"channels\""));
    assertTrue(inputTagFor(html, "channels-sms").contains("required"));
    assertTrue(
        inputTagFor(html, "channels-email").contains("data-required-checkbox-group=\"channels\""));
    assertTrue(html.contains("checkboxes[0].required = !checkboxes.some"));
  }

  @Test
  void multiAttributeLoginMarksUndeclaredAttributeRequiredWhenEnabled()
      throws IOException, TemplateException {
    // The authenticator keeps an attribute with no User Profile declaration mandatory, so the
    // fallback field must not present itself as optional.
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("nationalId", "dateOfBirth"));
    model.put("honorUserProfileRequired", true);
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute(
                "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), false)));
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    // Anchored to the input itself: a bare "required" would also match the requiredFields notice.
    assertTrue(normalized.contains("autocomplete=\"off\" required"));
    assertTrue(normalized.contains("</label> *"));
    assertTrue(html.contains("requiredFields"));
  }

  @Test
  void registrationQuotesHtmlAttributeAnnotationValues() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", false);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute(
                    "dateOfBirth",
                    "${dateOfBirth}",
                    Map.of(
                        "inputType",
                        "html5-date",
                        "html-attribute:autocomplete",
                        "off autofocus onfocus=alert(1)"))),
            "html5DataAnnotations",
            Map.of()));
    String tag = inputTagFor(renderRegister(model), "dateOfBirth");

    assertTrue(tag.contains("autocomplete=\"off autofocus onfocus=alert(1)\""));
  }

  /** The single rendered {@code <input>} tag carrying {@code id="<name>"}, whitespace collapsed. */
  private static String inputTagFor(String html, String name) {
    Matcher matcher =
        Pattern.compile("<input[^>]*id=\"" + Pattern.quote(name) + "\"[^>]*>").matcher(html);
    assertTrue(matcher.find(), "no input rendered for " + name);
    return matcher.group().replaceAll("\\s+", " ");
  }

  @Test
  void credentialFieldRendersFirstOnLoginWhenConfigured() throws IOException, TemplateException {
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> model = credentialFirstModel();
      model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
      model.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date")),
              mockAttribute("nationalId", "${nationalId}", Map.of())));
      String html = renderLogin(portal, model);

      assertTrue(html.indexOf("id=\"password\"") < html.indexOf("id=\"dateOfBirth\""));
      assertTrue(html.indexOf("id=\"dateOfBirth\"") < html.indexOf("id=\"nationalId\""));
      // exactly one credential field, and focus moved onto it
      assertEquals(1, html.split("id=\"password\"", -1).length - 1);
      assertEquals(1, html.split("autofocus", -1).length - 1);
      assertTrue(inputTagFor(html, "password").contains("autofocus"));
    }
  }

  @Test
  void credentialFieldStaysLastOnLoginByDefault() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
    String html = renderVotingPortalLogin(model);

    assertTrue(html.indexOf("id=\"dateOfBirth\"") < html.indexOf("id=\"password\""));
    assertTrue(inputTagFor(html, "dateOfBirth").contains("autofocus"));
  }

  @Test
  void credentialFieldRendersFirstOnRegistrationWhenConfigured()
      throws IOException, TemplateException {
    Map<String, Object> model = credentialFirstModel();
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("username", "${username}", Map.of()),
                mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))),
            "html5DataAnnotations",
            Map.of()));
    String html = renderRegister(model);

    assertTrue(html.indexOf("id=\"password\"") < html.indexOf("id=\"username\""));
    assertEquals(1, html.split("id=\"password\"", -1).length - 1);
    assertTrue(inputTagFor(html, "password").contains("autofocus"));
    assertEquals(1, html.split("autofocus", -1).length - 1);
  }

  @Test
  void credentialFirstKeepsPasswordAnnotationsOwnedByUsername()
      throws IOException, TemplateException {
    Map<String, Object> model = credentialFirstModel();
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date")),
                mockAttribute(
                    "username",
                    "${username}",
                    Map.of(
                        "passwordHelperTextBefore",
                        "Before password",
                        "passwordHelperTextAfter",
                        "After password",
                        "passwordStrengthBar",
                        "true"))),
            "html5DataAnnotations",
            Map.of()));

    String html = renderRegister(model);

    assertTrue(html.indexOf("id=\"password\"") < html.indexOf("id=\"dateOfBirth\""));
    assertTrue(html.contains("id=\"form-help-text-before-username\""));
    assertTrue(html.contains("Before password"));
    assertTrue(html.contains("id=\"form-help-text-after-username\""));
    assertTrue(html.contains("After password"));
    assertTrue(html.contains("id=\"password-progress\""));
    assertFalse(html.contains("form-help-text-before-dateOfBirth"));
  }

  @Test
  void showPasswordAfterThisOverridesCredentialFirst() throws IOException, TemplateException {
    // The annotation always wins, so realms configured before the setting existed are unaffected.
    Map<String, Object> model = credentialFirstModel();
    model.put("formMode", "LOGIN");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("username", "${username}", Map.of()),
                mockAttribute(
                    "dateOfBirth",
                    "${dateOfBirth}",
                    Map.of(
                        "inputType",
                        "html5-date",
                        "showPasswordAfterThis",
                        "true",
                        "passwordHelperTextBefore",
                        "Date-owned password helper"))),
            "html5DataAnnotations",
            Map.of()));
    String html = renderRegister(model);

    // The annotation places it after dateOfBirth, not first, even though the realm asks for FIRST.
    assertTrue(html.indexOf("id=\"dateOfBirth\"") < html.indexOf("id=\"password\""));
    assertTrue(html.indexOf("id=\"username\"") < html.indexOf("id=\"password\""));
    assertEquals(1, html.split("id=\"password\"", -1).length - 1);
    assertTrue(html.contains("id=\"form-help-text-before-dateOfBirth\""));
    assertTrue(html.contains("Date-owned password helper"));
    assertFalse(html.contains("form-help-text-before-username"));
  }

  @Test
  void allHiddenExplicitAnchorsStillRenderCredentialFirst() throws IOException, TemplateException {
    Map<String, Object> model = credentialFirstModel();
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", true);
    model.put("hiddenProfileAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute(
                    "dateOfBirth",
                    "${dateOfBirth}",
                    Map.of(
                        "inputType",
                        "html5-date",
                        "showPasswordAfterThis",
                        "true",
                        "passwordHelperTextBefore",
                        "Hidden helper"))),
            "html5DataAnnotations",
            Map.of()));

    String html = renderRegister(model);

    assertEquals(1, html.split("id=\"password\"", -1).length - 1);
    assertTrue(inputTagFor(html, "password").contains("autofocus"));
    assertFalse(html.contains("id=\"username\""));
    assertFalse(html.contains("id=\"dateOfBirth\""));
    assertFalse(html.contains("Hidden helper"));
  }

  @Test
  void showPasswordAfterThisCannotOverrideDisabledPasswordSetting()
      throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "LOGIN");
    model.put("passwordRequired", false);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute(
                    "dateOfBirth",
                    "${dateOfBirth}",
                    Map.of("inputType", "html5-date", "showPasswordAfterThis", "true"))),
            "html5DataAnnotations",
            Map.of()));

    assertFalse(renderRegister(model).contains("id=\"password\""));
  }

  @Test
  void falseShowPasswordAfterThisAlsoOverridesCredentialFirst()
      throws IOException, TemplateException {
    Map<String, Object> model = credentialFirstModel();
    model.put("formMode", "LOGIN");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("username", "${username}", Map.of("showPasswordAfterThis", "false")),
                mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))),
            "html5DataAnnotations",
            Map.of()));

    String html = renderRegister(model);

    assertFalse(html.contains("id=\"password\""));
  }

  /** baseModel with the realm opted in to a credential-first layout. */
  private static Map<String, Object> credentialFirstModel() {
    Map<String, Object> model = baseModel("standard");
    Map<String, Object> realm = new HashMap<>((Map<String, Object>) model.get("realm"));
    Map<String, Object> attributes = new HashMap<>((Map<String, Object>) realm.get("attributes"));
    attributes.put("credential-field-position", "FIRST");
    realm.put("attributes", attributes);
    model.put("realm", realm);
    return model;
  }

  @Test
  void localeSelectorIconsAreMarkedDecorative() throws IOException {
    // Two data-URI icons sit inside a button that already carries its own visible label. The
    // aria-hidden inside the encoded SVG applies to the image's own document, not to the img
    // element in the page, so it has to be declared on the element too.
    String template =
        Files.readString(THEME_ROOT.resolve("sequent.admin-portal/login/template.ftl"));
    Matcher images = Pattern.compile("<img\\b[^>]*>").matcher(template);
    int count = 0;
    while (images.find()) {
      String tag = images.group();
      count++;
      assertTrue(tag.contains("alt=\"\""), "img without empty alt: " + tag.substring(0, 60));
      assertTrue(tag.contains("aria-hidden=\"true\""), "img without aria-hidden");
    }
    assertTrue(count > 0, "expected the locale selector icons");
  }

  @Test
  void credentialIsMarkedRequiredWheneverTheRequiredNoticeIsShown()
      throws IOException, TemplateException {
    // The notice promises that required fields carry an asterisk, and the credential is always
    // mandatory - marking the match fields but not it left the page contradicting its own legend.
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> model = baseModel("standard");
      model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
      model.put("honorUserProfileRequired", true);
      model.put(
          "profile",
          profileWithAttributes(
              mockAttribute(
                  "dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true),
              mockAttribute("nationalId", "${nationalId}", Map.of(), false)));
      String normalized = renderLogin(portal, model).replaceAll("\\s+", " ");

      assertTrue(normalized.contains("requiredFields"));
      // one asterisk for the match field, one for the credential
      assertEquals(2, normalized.split("</label> \\*", -1).length - 1);
    }
  }

  @Test
  void credentialCarriesNoRequiredMarkerWithoutTheNotice() throws IOException, TemplateException {
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth"));
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
    String normalized = renderVotingPortalLogin(model).replaceAll("\\s+", " ");

    assertFalse(normalized.contains("requiredFields"));
    assertFalse(normalized.contains("</label> *"));
  }

  @Test
  void loginFormDefersValidationToTheAuthenticatorWhenConfigured()
      throws IOException, TemplateException {
    for (String portal : List.of("sequent.admin-portal", "sequent.voting-portal")) {
      Map<String, Object> browser = baseModel("standard");
      browser.put("matchAttributes", List.of("dateOfBirth"));
      browser.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
      assertFalse(renderLogin(portal, browser).contains("novalidate"));

      Map<String, Object> serverOnly = realmWith("login-validation-policy", "SERVER_ONLY");
      serverOnly.put("matchAttributes", List.of("dateOfBirth"));
      serverOnly.put(
          "profile",
          profileWithAttributes(
              mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))));
      String html = renderLogin(portal, serverOnly);

      assertTrue(html.contains("novalidate"));
      // the constraint attributes stay: novalidate suppresses the interactive pass, nothing else
      assertTrue(html.contains("max=\"9999-12-31\""));
    }
  }

  @Test
  void registrationKeepsBrowserValidation() throws IOException, TemplateException {
    Map<String, Object> model = realmWith("login-validation-policy", "SERVER_ONLY");
    model.put("formMode", "LOGIN");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"))),
            "html5DataAnnotations",
            Map.of()));

    assertFalse(renderRegister(model).contains("novalidate"));
  }

  /** baseModel with one extra realm attribute set. */
  private static Map<String, Object> realmWith(String key, String value) {
    Map<String, Object> model = baseModel("standard");
    Map<String, Object> realm = new HashMap<>((Map<String, Object>) model.get("realm"));
    Map<String, Object> attributes = new HashMap<>((Map<String, Object>) realm.get("attributes"));
    attributes.put(key, value);
    realm.put("attributes", attributes);
    model.put("realm", realm);
    return model;
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
    model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
    model.put("honorUserProfileRequired", true);
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true),
            mockAttribute("nationalId", "${nationalId}", Map.of(), false)));
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
  void noRequiredMarkersWhenEveryFieldIsMandatory() throws IOException, TemplateException {
    // With nothing optional, an asterisk on every field tells the voter nothing.
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("dateOfBirth", "nationalId"));
    model.put("honorUserProfileRequired", true);
    model.put(
        "profile",
        profileWithAttributes(
            mockAttribute("dateOfBirth", "${dateOfBirth}", Map.of("inputType", "html5-date"), true),
            mockAttribute("nationalId", "${nationalId}", Map.of(), true)));
    String html = renderVotingPortalLogin(model);
    String normalized = html.replaceAll("\\s+", " ");

    assertFalse(html.contains("requiredFields"));
    assertFalse(normalized.contains("</label> *"));
    // Still enforced, just not annotated. Anchored per input: a bare "required" would also match
    // the credential field, which is always required.
    assertTrue(inputTagFor(html, "dateOfBirth").contains("required"));
    assertTrue(inputTagFor(html, "nationalId").contains("required"));
  }

  @Test
  void requiredRadioMatchAttributeUsesBrowserGroupValidation()
      throws IOException, TemplateException {
    Map<String, Object> documentType =
        mockAttributeWithOptions(
            "documentType",
            Map.of("inputType", "select-radiobuttons"),
            List.of("national", "other"));
    documentType.put("required", true);
    Map<String, Object> model = baseModel("standard");
    model.put("matchAttributes", List.of("documentType"));
    model.put("honorUserProfileRequired", true);
    model.put("profile", profileWithAttributes(documentType));

    String html = renderVotingPortalLogin(model);

    assertTrue(inputTagFor(html, "documentType-national").contains("required"));
    assertTrue(inputTagFor(html, "documentType-other").contains("required"));
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

    // The attribute is neither enforced nor marked. The notice does appear, because the credential
    // is mandatory while this field is not - exactly the distinction the notice exists to draw.
    assertFalse(normalized.contains("aria-invalid=\"\" required"));
    assertTrue(html.contains("requiredFields"));
    assertEquals(1, normalized.split("</label> \\*", -1).length - 1);
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

  @Test
  void socialProvidersRenderAsOneList() throws IOException, TemplateException {
    // The grid layout applies to the list, so a list per provider would leave every grid holding a
    // single item.
    List<Map<String, Object>> providers =
        List.of("google", "github", "gitlab", "openid").stream()
            .map(
                alias ->
                    (Map<String, Object>)
                        Map.<String, Object>of(
                            "alias",
                            alias,
                            "loginUrl",
                            "/broker/" + alias,
                            "iconClasses",
                            "",
                            "displayName",
                            alias))
            .toList();
    Map<String, Object> model = baseModel("standard");
    model.put("social", Map.of("providers", providers));

    String html = renderVotingPortalLogin(model);

    assertEquals(4, html.split("<li>", -1).length - 1);
    assertEquals(1, html.split("<ul", -1).length - 1);
  }

  @Test
  void secondPasswordAnchorDoesNotDuplicateTheCredential() throws IOException, TemplateException {
    // Two attributes claiming the anchor is a misconfiguration; rendering both would put duplicate
    // password ids and names on the page.
    Map<String, Object> model = baseModel("standard");
    model.put("formMode", "REGISTRATION");
    model.put("passwordRequired", true);
    model.put(
        "profile",
        Map.of(
            "attributes",
            List.of(
                mockAttribute("username", "${username}", Map.of("showPasswordAfterThis", "false")),
                mockAttribute(
                    "dateOfBirth",
                    "${dateOfBirth}",
                    Map.of("inputType", "html5-date", "showPasswordAfterThis", "true")),
                mockAttribute(
                    "nationalId", "${nationalId}", Map.of("showPasswordAfterThis", "true"))),
            "html5DataAnnotations",
            Map.of()));

    String html = renderRegister(model);

    assertEquals(1, html.split("id=\"password\"", -1).length - 1);
    assertEquals(1, html.split("id=\"password-confirm\"", -1).length - 1);
    // The first anchor in profile order wins.
    assertTrue(html.indexOf("id=\"dateOfBirth\"") < html.indexOf("id=\"password\""));
    assertTrue(html.indexOf("id=\"password\"") < html.indexOf("id=\"nationalId\""));
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
    attribute.put("validators", Map.of());
    return attribute;
  }

  /** Mimics an attribute whose options come from an {@code options} validator. */
  private static Map<String, Object> mockAttributeWithOptions(
      String name, Map<String, Object> annotations, List<String> options) {
    Map<String, Object> attribute = mockAttribute(name, "${" + name + "}", annotations);
    attribute.put("validators", Map.of("options", Map.of("options", options)));
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
