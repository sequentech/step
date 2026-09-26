// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import com.twilio.Twilio;
import com.twilio.rest.verify.v2.service.Verification;
import com.twilio.rest.verify.v2.service.VerificationCreator;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.theme.Theme;
import org.mockito.MockedStatic;
import sequent.keycloak.authenticator.CapturedLogs;
import sequent.keycloak.authenticator.Utils;

class SmsSecretLoggingTest {
  @Test
  void dummyDeliveryDoesNotLogMessageBodies() throws Exception {
    try (CapturedLogs logs = new CapturedLogs(DummySmsSenderProvider.class)) {
      new DummySmsSenderProvider()
          .send("+15550123456", "OTP 482619 https://auth.example/?key=synthetic-token");
      assertFalse(logs.text().isEmpty());
      assertFalse(logs.text().contains("482619"));
      assertFalse(logs.text().contains("synthetic-token"));
    }
  }

  @Test
  void twilioDeliversOtpWithoutLoggingCodesCredentialsOrSkippedLinks() throws Exception {
    KeycloakSession session = mock(KeycloakSession.class, RETURNS_DEEP_STUBS);
    RealmModel realm = mock(RealmModel.class);
    UserModel user = mock(UserModel.class);
    Theme theme = mock(Theme.class);
    when(session.getContext().resolveLocale(user)).thenReturn(Locale.ENGLISH);
    when(session.theme().getTheme(Theme.Type.LOGIN)).thenReturn(theme);
    Properties messages = new Properties();
    messages.setProperty(Utils.SEND_CODE_SMS_I18N_KEY, "Your code is {1}");
    messages.setProperty(Utils.SEND_LINK_SMS_I18N_KEY, "Your link is {1}");
    when(theme.getEnhancedMessages(realm, Locale.ENGLISH)).thenReturn(messages);
    VerificationCreator creator = mock(VerificationCreator.class, RETURNS_SELF);
    Verification verification = mock(Verification.class);
    when(creator.create()).thenReturn(verification);
    when(verification.getSid()).thenReturn("synthetic-verification");
    try (CapturedLogs logs = new CapturedLogs(TwilioVerifySenderProvider.class);
        MockedStatic<Twilio> twilio = mockStatic(Twilio.class);
        MockedStatic<Verification> verifications = mockStatic(Verification.class)) {
      verifications
          .when(() -> Verification.creator("test-service", "+15550123456", "sms"))
          .thenReturn(creator);
      TwilioVerifySenderProvider provider = new TwilioVerifySenderProvider();
      assertEquals(
          "Your code is 482619",
          provider.send(
              "+15550123456",
              Utils.SEND_CODE_SMS_I18N_KEY,
              List.of("realm", "482619", "5"),
              realm,
              user,
              session));
      verify(creator).setCustomCode("482619");
      verify(session.getContext().getAuthenticationSession())
          .setAuthNote(TwilioVerifySenderProvider.SID_AUTH_NOTE, "synthetic-verification");
      assertEquals(
          "Your link is https://auth.example/?key=synthetic-token",
          provider.send(
              "+15550123456",
              Utils.SEND_LINK_SMS_I18N_KEY,
              List.of("realm", "https://auth.example/?key=synthetic-token", "5"),
              realm,
              user,
              session));
      assertFalse(logs.text().contains("482619"));
      assertFalse(logs.text().contains("synthetic-token"));
      assertFalse(logs.text().contains("synthetic-provider-secret"));
    }
  }
}
