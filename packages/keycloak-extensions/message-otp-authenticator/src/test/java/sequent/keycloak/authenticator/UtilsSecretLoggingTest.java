// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.net.URI;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;

class UtilsSecretLoggingTest {
  @Test
  void masksEveryLiteralOtpAndOtlRepresentationInCommunications() {
    String code = "482619";
    assertEquals(
        "Code ******, repeated ******",
        Utils.maskCode("Code " + code + ", repeated " + code, code));
    String link =
        "https://auth.example/realms/test/login-actions/action-token?key=ey.synthetic-token.sig&client_id=portal";
    String htmlLink = link.replace("&", "&amp;");
    String body =
        "{\"textBody\":\""
            + link
            + "\",\"htmlBody\":\"<a href=\\\""
            + htmlLink
            + "\\\">Login</a>\"}";
    String masked = Utils.maskCode(body, link);
    assertFalse(masked.contains("ey.synthetic-token.sig"));
    assertTrue(masked.contains("Login"));
    assertEquals("No code here", Utils.maskCode("No code here", code));
  }

  @Test
  void generatedActionLinkContainsItsTokenButLogsDoNot() {
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      URI link =
          Utils.actionTokenBuilder(
                  URI.create("https://auth.example/"), "synthetic-otl-token", "portal")
              .build("test");
      assertEquals(
          "https://auth.example/realms/test/login-actions/action-token?key=synthetic-otl-token&client_id=portal",
          link.toString());
      assertFalse(logs.text().contains("synthetic-otl-token"));
    }
  }

  @Test
  void missingOptionalConfigurationDoesNotLogTheTestCode() {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(Map.of(Utils.TEST_MODE_CODE_ATTRIBUTE, "synthetic-config-secret"));
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD))
        .thenReturn("+15550123456");
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertEquals("+15550123456", Utils.getMobile(config, user));
      assertEquals(
          List.of("ES"),
          Utils.getMultivalueString(config, Utils.VALID_COUNTRY_CODES, List.of("ES")));
      assertFalse(logs.text().contains("synthetic-config-secret"));
    }
  }
}
