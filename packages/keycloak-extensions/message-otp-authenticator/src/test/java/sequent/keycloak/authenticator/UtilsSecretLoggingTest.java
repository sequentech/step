// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.theme.KeycloakSanitizerMethod;

class UtilsSecretLoggingTest {
  private static final String TOKEN = "eyJhbGciOiJIUzUxMiJ9.eyJ0eXAiOiJvdGwifQ.c3ludGhldGlj";

  private static String actionLink(String clientId) {
    return Utils.actionTokenBuilder(URI.create("https://auth.example/"), TOKEN, clientId)
        .build("test")
        .toString();
  }

  @Test
  void masksNumericCodeInCommunications() {
    String code = "482619";
    assertEquals(
        "Code ******, repeated ******",
        Utils.maskCode("Code " + code + ", repeated " + code, code));
    assertEquals("No code here", Utils.maskCode("No code here", code));
  }

  @Test
  void masksActionLinkInTextAndHtmlBodies() {
    String link = actionLink("portal");
    String body =
        "{\"textBody\":\""
            + link
            + "\",\"htmlBody\":\"<a href=\\\""
            + link.replace("&", "&amp;")
            + "\\\">Login</a>\"}";
    String masked = Utils.maskCode(body, link);
    assertFalse(masked.contains(TOKEN));
    assertTrue(masked.contains("Login"));
  }

  @Test
  void masksActionLinkInSanitizedHtml() throws Exception {
    for (String clientId : List.of("portal", "voter..portal")) {
      String link = actionLink(clientId);
      String html =
          (String)
              new KeycloakSanitizerMethod()
                  .exec(List.of("<a href=\"" + link + "\" target=\"_blank\">Continue</a>"));
      String masked = Utils.maskCode(html, link);
      assertFalse(masked.contains(TOKEN));
      assertTrue(masked.contains("Continue"));
    }
  }

  @Test
  void buildsActionLinkWithoutLoggingItsToken() {
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertEquals(
          "https://auth.example/realms/test/login-actions/action-token?key="
              + TOKEN
              + "&client_id=portal",
          actionLink("portal"));
      assertTrue(logs.text().contains("actionTokenBuilder()"));
      assertFalse(logs.text().contains(TOKEN));
    }
  }
}
