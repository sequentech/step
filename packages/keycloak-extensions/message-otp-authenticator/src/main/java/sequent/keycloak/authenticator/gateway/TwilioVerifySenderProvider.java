// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import com.twilio.Twilio;
import com.twilio.rest.verify.v2.service.Verification;
import java.io.IOException;
import java.text.MessageFormat;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;
import sequent.keycloak.authenticator.Utils;

@JBossLog
public class TwilioVerifySenderProvider implements SmsSenderProvider {
  // Find your Account SID and Auth Token at twilio.com/console
  // and set the environment variables. See http://twil.io/secure
  public static final String ACCOUNT_SID = System.getenv("TWILIO_ACCOUNT_SID");
  public static final String SERVICE_SID = System.getenv("TWILIO_SERVICE_SID");
  public static final String AUTH_TOKEN = System.getenv("TWILIO_AUTH_TOKEN");
  public static final String SID_AUTH_NOTE = new String("SID_TWILIO");

  TwilioVerifySenderProvider() {
    log.info("Initializing Twilio Verify SMS provider");
    Twilio.init(ACCOUNT_SID, AUTH_TOKEN);
  }

  @Override
  public String send(
      String phoneNumber,
      String messageKey,
      List<String> attributes,
      RealmModel realm,
      UserModel user,
      KeycloakSession session)
      throws IOException {

    Locale locale = session.getContext().resolveLocale(user);

    Theme theme = session.theme().getTheme(Theme.Type.LOGIN);
    Properties messages = theme.getEnhancedMessages(realm, locale);
    String formattedMessage =
        new MessageFormat(messages.getProperty(messageKey, messageKey), locale)
            .format(attributes.toArray());

    if (!messageKey.equals(Utils.SEND_CODE_SMS_I18N_KEY)) {
      log.infov("Skipping non-OTP Twilio Verify SMS to {0}", phoneNumber);
      return formattedMessage;
    }

    AuthenticationSessionModel authSession = session.getContext().getAuthenticationSession();
    if (authSession == null) {
      log.errorv("NULL authSession={0}", authSession);
      throw new IOException("NULL authSession");
    }

    String otpCode = attributes.get(1);
    log.infov("Sending Twilio Verify SMS to {0}", phoneNumber);

    Verification verification =
        Verification.creator(SERVICE_SID, phoneNumber, "sms").setCustomCode(otpCode).create();
    String sid = verification.getSid();
    authSession.setAuthNote(SID_AUTH_NOTE, sid);
    log.infov("Sent Twilio Verify SMS to {0}, sid={1}", phoneNumber, sid);

    return formattedMessage;
  }

  @Override
  public void sendFeedback(
      String phoneNumber,
      boolean success,
      RealmModel realm,
      UserModel user,
      KeycloakSession session)
      throws IOException {
    AuthenticationSessionModel authSession = session.getContext().getAuthenticationSession();
    if (authSession == null) {
      log.errorv("NULL authSession={0}", authSession);
      throw new IOException("NULL authSession");
    }
    String sid = authSession.getAuthNote(SID_AUTH_NOTE);

    log.infov(
        "**Sending Twilio Verify SMS**:\n\t- phoneNumber={0}\n\t- success={1}",
        phoneNumber, success);
    Verification verification =
        Verification.updater(
                SERVICE_SID,
                sid,
                (success) ? Verification.Status.APPROVED : Verification.Status.CANCELED)
            .update();

    log.infov(
        "**SENT Twilio Verify SMS**:\n\t- phoneNumber={0}\n\t- success={1}\n\t- resultSid={2}",
        phoneNumber, success, verification.getSid());
  }

  @Override
  public void close() {}
}
