// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;
import java.io.IOException;
import java.text.MessageFormat;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Properties;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.common.util.SecretGenerator;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.email.freemarker.beans.ProfileBean;
import org.keycloak.forms.login.freemarker.model.UrlBean;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.FreeMarkerException;
import org.keycloak.theme.Theme;
import org.keycloak.theme.beans.MessageFormatterMethod;
import org.keycloak.theme.freemarker.FreeMarkerProvider;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;

@UtilityClass
@JBossLog
public class Utils {
  public final String CODE = "code";
  public final String CODE_LENGTH = "length";
  public final String CODE_TTL = "ttl";
  public final String SENDER_ID = "senderId";
  public final String TEL_USER_ATTRIBUTE = "telUserAttribute";
  public final String MESSAGE_COURIER_ATTRIBUTE = "messageCourierAttribute";
  public final String DEFERRED_USER_ATTRIBUTE = "deferredUserAttribute";
  public final String SEND_CODE_SMS_I18N_KEY = "messageOtp.sendCode.sms.text";
  public final String SEND_CODE_EMAIL_SUBJECT = "messageOtp.sendCode.email.subject";
  public final String SEND_CODE_EMAIL_FTL = "send-code-email.ftl";
  public final String RESEND_ACTIVATION_TIMER = "resendCoudActivationTimer";

  public static final String SEND_SUCCESS_SUBJECT = "messageSuccessEmailSubject";
  public static final String SEND_SUCCESS_SMS_I18N_KEY = "messageSuccessSms";
  public static final String SEND_SUCCESS_EMAIL_FTL = "success-email.ftl";

  public enum MessageCourier {
    SMS,
    EMAIL,
    BOTH,
    NONE;

    // Method to convert a string value to a NotificationType
    public static MessageCourier fromString(String type) {
      if (type != null) {
        for (MessageCourier messageCourier : MessageCourier.values()) {
          if (type.equalsIgnoreCase(messageCourier.name())) {
            return messageCourier;
          }
        }
      }
      throw new IllegalArgumentException("No constant with text " + type + " found");
    }
  }

  /** Sends code and also sets the auth notes related to the code */
  void sendCode(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean deferredUser)
      throws IOException, EmailException {
    log.info("sendCode(): start");
    String mobileNumber = null;
    String emailAddress = null;

    if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
    } else {
      mobileNumber = Utils.getMobile(config, user);
      emailAddress = user.getEmail();
    }
    log.infov("sendCode(): mobileNumber=`{0}`", mobileNumber);
    log.infov("sendCode(): emailAddress=`{0}`", emailAddress);

    int length = Integer.parseInt(config.getConfig().get(Utils.CODE_LENGTH));
    int ttl = Integer.parseInt(config.getConfig().get(Utils.CODE_TTL));

    String code = SecretGenerator.getInstance().randomString(length, SecretGenerator.DIGITS);
    authSession.setAuthNote(Utils.CODE, code);
    authSession.setAuthNote(
        Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + (ttl * 1000L)));
    RealmModel realm = authSession.getRealm();
    String realmName = getRealmName(realm);

    if (mobileNumber != null) {
      log.infov("sendCode(): mobileNumber TRIM=`{0}`", mobileNumber.trim());
      log.infov("sendCode(): mobileNumber LENGTH=`{0}`", mobileNumber.trim().length());
    }
    log.infov("sendCode(): messageCourier=`{0}`", messageCourier);

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (messageCourier == MessageCourier.SMS || messageCourier == MessageCourier.BOTH)) {
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      List<String> smsAttributes =
          ImmutableList.of(realmName, code, String.valueOf(Math.floorDiv(ttl, 60)));

      smsSenderProvider.send(
          mobileNumber.trim(), Utils.SEND_CODE_SMS_I18N_KEY, smsAttributes, realm, user, session);
    } else {
      log.infov("sendCode(): NOT Sending SMS to=`{0}`", mobileNumber);
    }

    if (emailAddress != null
        && emailAddress.trim().length() > 0
        && (messageCourier == MessageCourier.EMAIL || messageCourier == MessageCourier.BOTH)) {
      log.infov("sendCode(): Sending email to=`{0}`", emailAddress.trim());
      EmailTemplateProvider emailTemplateProvider =
          session.getProvider(EmailTemplateProvider.class);

      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realmName);
      messageAttributes.put("code", code);
      messageAttributes.put("ttl", Math.floorDiv(ttl, 60));

      List<Object> subjAttr = ImmutableList.of(realmName);
      log.infov("sendCode(): Sending email: prepared messageAttributes");

      try {
        if (deferredUser) {
          sendEmail(
              session,
              realm,
              user,
              Utils.SEND_CODE_EMAIL_SUBJECT,
              subjAttr,
              Utils.SEND_CODE_EMAIL_FTL,
              messageAttributes,
              emailAddress.trim());
        } else {
          emailTemplateProvider
              .setRealm(realm)
              .setUser(user)
              .setAttribute("realmName", realmName)
              .send(
                  Utils.SEND_CODE_EMAIL_SUBJECT,
                  subjAttr,
                  Utils.SEND_CODE_EMAIL_FTL,
                  messageAttributes);
        }
      } catch (EmailException error) {
        log.debug("sendCode(): Exception sending email", error);
        throw error;
      }
    } else {
      log.infov("sendCode(): NOT Sending email to=`{0}`", emailAddress);
    }
  }

  String getMobile(AuthenticatorConfigModel config, UserModel user) {
    log.infov("getMobile()");
    if (config == null) {
      log.infov("getMobile(): NULL config={0}", config);
      return user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null || !mapConfig.containsKey(Utils.TEL_USER_ATTRIBUTE)) {
      log.infov("getEmail(): NullOrNotFound mapConfig={0}", mapConfig);
      return user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    }
    String telUserAttribute = mapConfig.get(Utils.TEL_USER_ATTRIBUTE);

    String mobile = user.getFirstAttribute(telUserAttribute);
    log.infov("getMobile(): telUserAttribute={0}, mobile={1}", telUserAttribute, mobile);
    return mobile;
  }

  Optional<AuthenticatorConfigModel> getConfig(RealmModel realm) {
    // Using streams to find the first matching configuration
    // TODO: We're assuming there's only one instance in this realm of this
    // authenticator
    Optional<AuthenticatorConfigModel> configOptional =
        realm
            .getAuthenticationFlowsStream()
            .flatMap(flow -> realm.getAuthenticationExecutionsStream(flow.getId()))
            .filter(
                model -> {
                  boolean ret =
                      (model.getAuthenticator() != null
                          && model
                              .getAuthenticator()
                              .equals(MessageOTPAuthenticatorFactory.PROVIDER_ID));
                  return ret;
                })
            .map(model -> realm.getAuthenticatorConfigById(model.getAuthenticatorConfig()))
            .findFirst();
    return configOptional;
  }

  /** We use constant time comparison for security reasons, to avoid timing attacks */
  boolean constantTimeIsEqual(byte[] digesta, byte[] digestb) {
    if (digesta.length != digestb.length) {
      return false;
    }

    int result = 0;
    // time-constant comparison
    for (int i = 0; i < digesta.length; i++) {
      result |= digesta[i] ^ digestb[i];
    }
    return result == 0;
  }

  public static String getRealmName(RealmModel realm) {
    return Strings.isNullOrEmpty(realm.getDisplayName()) ? realm.getName() : realm.getDisplayName();
  }

  protected EmailTemplate processEmailTemplate(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      String subjectKey,
      List<Object> subjectAttributes,
      String template,
      Map<String, Object> attributes)
      throws EmailException {
    try {
      Theme theme = session.theme().getTheme(Theme.Type.EMAIL);
      Locale locale = session.getContext().resolveLocale(user);
      attributes.put("locale", locale);

      Properties messages = theme.getEnhancedMessages(realm, locale);
      attributes.put("msg", new MessageFormatterMethod(locale, messages));

      attributes.put("properties", theme.getProperties());
      attributes.put("realmName", realm.getName());
      if (user != null) {
        attributes.put("user", new ProfileBean(user, session));
      }
      KeycloakUriInfo uriInfo = session.getContext().getUri();
      attributes.put("url", new UrlBean(realm, theme, uriInfo.getBaseUri(), null));

      String subject =
          new MessageFormat(messages.getProperty(subjectKey, subjectKey), locale)
              .format(subjectAttributes.toArray());
      String textTemplate = String.format("text/%s", template);
      String textBody;
      FreeMarkerProvider freeMarker = session.getProvider(FreeMarkerProvider.class);
      try {
        textBody = freeMarker.processTemplate(attributes, textTemplate, theme);
      } catch (final FreeMarkerException e) {
        throw new EmailException("Failed to template plain text email.", e);
      }
      String htmlTemplate = String.format("html/%s", template);
      String htmlBody;
      try {
        htmlBody = freeMarker.processTemplate(attributes, htmlTemplate, theme);
      } catch (final FreeMarkerException e) {
        throw new EmailException("Failed to template html email.", e);
      }
      return new EmailTemplate(subject, textBody, htmlBody);
    } catch (Exception e) {
      throw new EmailException("Failed to template email", e);
    }
  }

  protected void sendEmail(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      String subjectFormatKey,
      List<Object> subjectAttributes,
      String bodyTemplate,
      Map<String, Object> bodyAttributes,
      String address)
      throws EmailException {
    try {
      EmailTemplate emailTemplate =
          processEmailTemplate(
              session,
              realm,
              user,
              subjectFormatKey,
              subjectAttributes,
              bodyTemplate,
              bodyAttributes);
      EmailSenderProvider emailSender = session.getProvider(EmailSenderProvider.class);

      emailSender.send(
          realm.getSmtpConfig(),
          address,
          emailTemplate.getSubject(),
          emailTemplate.getTextBody(),
          emailTemplate.getHtmlBody());
    } catch (EmailException e) {
      throw e;
    } catch (Exception e) {
      throw new EmailException("Failed to template email", e);
    }
  }

  protected static class EmailTemplate {

    private String subject;
    private String textBody;
    private String htmlBody;

    public EmailTemplate(String subject, String textBody, String htmlBody) {
      this.subject = subject;
      this.textBody = textBody;
      this.htmlBody = htmlBody;
    }

    public String getSubject() {
      return subject;
    }

    public String getTextBody() {
      return textBody;
    }

    public String getHtmlBody() {
      return htmlBody;
    }
  }

  protected static String getOtpAddress(
      Utils.MessageCourier courier,
      boolean deferredUser,
      AuthenticatorConfigModel config,
      AuthenticationSessionModel authSession,
      UserModel user) {
    String mobileNumber = null;
    String emailAddress = null;

    if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
    } else {
      mobileNumber = Utils.getMobile(config, user);
      emailAddress = user.getEmail();
    }
    switch (courier) {
      case EMAIL:
        return obscureEmail(emailAddress);
      case SMS:
        return obscurePhoneNumber(mobileNumber);
      case BOTH:
        return emailAddress != null ? obscureEmail(emailAddress) : obscurePhoneNumber(mobileNumber);
    }
    return emailAddress;
  }

  protected static String obscurePhoneNumber(String phoneNumber) {
    if (phoneNumber == null) {
      return phoneNumber;
    }
    return phoneNumber.substring(0, 4)
        + "*".repeat(phoneNumber.length() - 7)
        + phoneNumber.substring(phoneNumber.length() - 3);
  }

  protected static String obscureEmail(String email) {
    int atIndex = email.indexOf('@');
    if (atIndex == -1 || atIndex < 2) {
      return email;
    }

    String firstPart = email.substring(0, 2);
    String domainPart = email.substring(atIndex + 1);
    String maskedLocal = firstPart + "*".repeat(atIndex - 2);

    int lastDotIndex = domainPart.lastIndexOf('.');
    String domain, tld;

    if (lastDotIndex != -1) {
      domain = domainPart.substring(0, lastDotIndex);
      tld = domainPart.substring(lastDotIndex);
    } else {
      domain = domainPart;
      tld = "";
    }

    String maskedDomain = domain;
    if (domain.length() >= 2) {
      maskedDomain = "*".repeat(domain.length() - 2) + domain.substring(domain.length() - 2);
    }
    return maskedLocal + "@" + maskedDomain + tld;
  }

  public static void sendConfirmation(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber)
      throws EmailException, IOException {
    log.info("sendConfirmation(): start");
    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's going to be
    // either email or mobileNumber.
    String username = user.getEmail() != null ? user.getEmail() : mobileNumber;
    log.infov("sendConfirmation(): username {0}", username);
    log.infov("sendConfirmation(): messageCourier {0}", messageCourier);

    String email = user.getEmail();

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendConfirmation(): sending email", username);
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realName);
      messageAttributes.put("username", username);

      emailTemplateProvider
          .setRealm(realm)
          .setUser(user)
          .setAttribute("realmName", realName)
          .setAttribute("username", username)
          .send(SEND_SUCCESS_SUBJECT, subjAttr, SEND_SUCCESS_EMAIL_FTL, messageAttributes);
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendConfirmation(): sending sms", username);

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      List<String> smsAttributes = ImmutableList.of(realName, username);

      smsSenderProvider.send(
          mobileNumber.trim(), SEND_SUCCESS_SMS_I18N_KEY, smsAttributes, realm, user, session);
    }
  }
}
