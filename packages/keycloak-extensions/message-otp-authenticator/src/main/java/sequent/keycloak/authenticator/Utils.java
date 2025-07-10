// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;
import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.core.UriInfo;
import java.io.IOException;
import java.net.URI;
import java.text.MessageFormat;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Properties;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.DefaultActionToken;
import org.keycloak.common.util.SecretGenerator;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.email.freemarker.beans.ProfileBean;
import org.keycloak.events.Event;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.freemarker.model.UrlBean;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.Constants;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.Urls;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.services.resources.RealmsResource;
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.FreeMarkerException;
import org.keycloak.theme.Theme;
import org.keycloak.theme.beans.MessageFormatterMethod;
import org.keycloak.theme.freemarker.FreeMarkerProvider;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.otl.OTLActionToken;

@UtilityClass
@JBossLog
public class Utils {
  public final String CODE = "code";
  public final String CODE_LENGTH = "length";
  public final String CODE_TTL = "ttl";
  public final String SENDER_ID = "senderId";
  public final String ONE_TIME_LINK = "one-time-link";
  public final String OTL_VISITED = "one-time-link.visited";
  public static final String USER_ID = "userId";
  public final String TEL_USER_ATTRIBUTE = "telUserAttribute";
  public final String MESSAGE_COURIER_ATTRIBUTE = "messageCourierAttribute";
  public final String DEFERRED_USER_ATTRIBUTE = "deferredUserAttribute";
  public final String OTL_RESTORED_AUTH_NOTES_ATTRIBUTE = "otlRestoredAuthNotesAttribute";

  public final String SEND_CODE_SMS_I18N_KEY = "messageOtp.sendCode.sms.text";
  public final String SEND_CODE_EMAIL_SUBJECT = "messageOtp.sendCode.email.subject";
  public final String SEND_CODE_EMAIL_FTL = "send-code-email.ftl";
  public final String RESEND_ACTIVATION_TIMER = "resendCoudActivationTimer";

  public final String SEND_LINK_SMS_I18N_KEY = "messageOtp.sendLink.sms.text";
  public final String SEND_LINK_EMAIL_SUBJECT = "messageOtp.sendLink.email.subject";
  public final String SEND_LINK_EMAIL_FTL = "send-link-email.ftl";

  public static final String SEND_SUCCESS_SMS_I18N_KEY = "messageSuccessSms";
  public static final String SEND_SUCCESS_SMS_I18N_KEY_KIOSK = "messageSuccessSmsKiosk";
  public static final String SEND_SUCCESS_EMAIL_SUBJECT = "messageSuccessEmailSubject";
  public static final String SEND_SUCCESS_EMAIL_FTL = "success-email.ftl";
  public static final String SEND_PENDING_SMS_I18N_KEY = "messagePendingSms";
  public static final String SEND_PENDING_EMAIL_SUBJECT = "messagePendingEmailSubject";
  public static final String SEND_PENDING_EMAIL_FTL = "pending-email.ftl";
  public static final String SEND_REJECT_SMS_I18N_KEY = "messageRejectedSms";
  public static final String SEND_REJECT_EMAIL_SUBJECT = "messageRejectedEmailSubject";
  public static final String SEND_REJECT_EMAIL_FTL = "reject-email.ftl";
  public static final String SEND_SUCCESS_EMAIL_DIFF_POST_FTL = "success-email-diff-post.ftl";
  public static final String ERROR_MESSAGE_NOT_SENT = "messageNotSent";

  public static final String SEND_ERROR_EMAIL_SUBJECT = "registrationErrorEmailSubject";
  public static final String SEND_ERROR_EMAIL_FTL = "error-email.ftl";
  public static final String SEND_SUPPORT_ERROR_EMAIL_SUBJECT =
      "userRegistrationErrorNotificationSubject";
  public static final String SEND_SUPPORT_ERROR_EMAIL_FTL = "support-error-email.ftl";

  public static final String SEND_REGISTER_FAILED_SMS_I18N_KEY = "messageFailedSMS";

  public static final String ID_NUMBER_ATTRIBUTE = "sequent.read-only.id-card-number";
  public static final String PHONE_NUMBER_ATTRIBUTE = "sequent.read-only.mobile-number";

  public static final String ID_NUMBER = "ID_number";
  public static final String PHONE_NUMBER = "Phone_number";
  public static final String USER_PROFILE_ATTRIBUTES = "user_profile_attributes";
  public static final String AUTHENTICATOR_CLASS_NAME = "authenticator_class_name";

  public static final String EVENT_TYPE_COMMUNICATIONS = "communications";
  public static final String TEST_MODE_ATTRIBUTE = "test-mode";
  public static final String TEST_MODE_CODE_ATTRIBUTE = "test-mode-code";
  public static final String MAX_RECEIVER_REUSE = "max-receiver-reuse";
  public static final String VALID_COUNTRY_CODES = "valid-country-codes";
  public static final List<String> VALID_COUNTRY_CODES_DEFAULT =
      Collections.unmodifiableList(Arrays.asList());

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

  String escapeJson(String value) {
    return value != null
        ? value.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
        : null;
  }

  void sendFeedback(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean isSuccess,
      boolean deferredUser,
      boolean isOtl)
      throws IOException {
    log.info("sendFeedback(): start");
    String mobileNumber = null;

    // Handle deferred user
    if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
    } else {
      mobileNumber = Utils.getMobile(config, user);
    }
    log.infov("sendFeedback(): mobileNumber=`{0}`", mobileNumber);

    if (isOtl) {
      log.info("sendFeedback(): isOtl so not sending feedback");
      return;
    }
    RealmModel realm = authSession.getRealm();

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (messageCourier == MessageCourier.SMS || messageCourier == MessageCourier.BOTH)) {
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      smsSenderProvider.sendFeedback(mobileNumber, isSuccess, realm, user, session);
    }
  }

  public List<String> getMultivalueString(
      AuthenticatorConfigModel config, String configKey, List<String> defaultValue) {
    log.debugv("getMultivalueString(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.debugv("getMultivalueString(): NULL config={0}", config);
      return defaultValue;
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.debugv("getMultivalueString(): NullOrNotFound mapConfig={0}", mapConfig);
      return defaultValue;
    }

    log.debugv("getMultivalueString(): value={0}", mapConfig.get(configKey));

    return Arrays.asList(mapConfig.get(configKey).split("##"));
  }

  String getMobileNumber(
      AuthenticatorConfigModel config,
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean deferredUser
  ) throws IOException {
    String mobileNumber = null;

    // Handle deferred user
    if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
    } else {
      mobileNumber = Utils.getMobile(config, user);
    }

    return mobileNumber;
  }

  String getEmailAddress(
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean deferredUser
  ) throws IOException {
    String emailAddress = null;

    // Handle deferred user
    if (deferredUser) {
      emailAddress = authSession.getAuthNote("email");
    } else {
      emailAddress = user.getEmail();
    }

    return emailAddress;
  }

  /** Sends code and also sets the auth notes related to the code */
  void sendCode(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean deferredUser,
      boolean isOtl,
      String[] otlAuthNotesNames,
      Object context)
      throws IOException, EmailException {
    log.info("sendCode(): start");
    String mobileNumber = Utils.getMobileNumber(config, user, authSession, deferredUser);
    String emailAddress = Utils.getEmailAddress(user, authSession, deferredUser);
    String code = null;
    
    log.infov("sendCode(): mobileNumber=`{0}`", mobileNumber);
    log.infov("sendCode(): emailAddress=`{0}`", emailAddress);

    int length = Integer.parseInt(config.getConfig().get(Utils.CODE_LENGTH));
    int ttl = Integer.parseInt(config.getConfig().get(Utils.CODE_TTL));
    authSession.setAuthNote(
        Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + (ttl * 1000L)));

    // Handle OTL/OTP
    if (isOtl) {
      code =
          generateOTL(
              authSession,
              session,
              ttl,
              otlAuthNotesNames,
              authSession.getRedirectUri(),
              deferredUser);
      authSession.setAuthNote(Utils.OTL_VISITED, "false");
    } else {
      code = SecretGenerator.getInstance().randomString(length, SecretGenerator.DIGITS);
      authSession.setAuthNote(Utils.CODE, code);
    }

    RealmModel realm = authSession.getRealm();
    String realmName = getRealmName(realm);

    if (mobileNumber != null) {
      log.infov("sendCode(): mobileNumber TRIM=`{0}`", mobileNumber.trim());
      log.infov("sendCode(): mobileNumber LENGTH=`{0}`", mobileNumber.trim().length());
    }
    log.infov("sendCode(): messageCourier=`{0}`", messageCourier);

    // Sending via SMS
    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (messageCourier == MessageCourier.SMS || messageCourier == MessageCourier.BOTH)) {
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      List<String> smsAttributes =
          ImmutableList.of(realmName, code, String.valueOf(Math.floorDiv(ttl, 60)));

      String smsTemplateKey = (isOtl) ? Utils.SEND_LINK_SMS_I18N_KEY : Utils.SEND_CODE_SMS_I18N_KEY;
      String formattedMessage =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTemplateKey, smsAttributes, realm, user, session);
      formattedMessage = maskCode(formattedMessage, code);
      communicationsLog(context, formattedMessage);
    } else {
      log.infov("sendCode(): NOT Sending SMS to=`{0}`", mobileNumber);
    }

    // Sending via Email
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
        String subjectKey = (isOtl) ? Utils.SEND_LINK_EMAIL_SUBJECT : Utils.SEND_CODE_EMAIL_SUBJECT;
        String ftlKey = (isOtl) ? Utils.SEND_LINK_EMAIL_FTL : Utils.SEND_CODE_EMAIL_FTL;
        String textBody =
            sendEmail(
                session,
                realm,
                user,
                subjectKey,
                subjAttr,
                ftlKey,
                messageAttributes,
                emailAddress.trim(),
                deferredUser,
                null);
        textBody = maskCode(textBody, code);
        communicationsLog(context, textBody);
      } catch (EmailException error) {
        log.debug("sendCode(): Exception sending email", error);
        throw error;
      }
    } else {
      log.infov("sendCode(): NOT Sending email to=`{0}`", emailAddress);
    }
  }

  /* Masks the auth code from the content body with stars */
  protected String maskCode(String content, String code) {
    return content.replaceAll(code, "*".repeat(code.length()));
  }

  void communicationsLog(Object context, String body) {
    if (context instanceof AuthenticationFlowContext) {
      logCommunications((AuthenticationFlowContext) context, body);
    } else if (context instanceof RequiredActionContext) {
      logCommunications((RequiredActionContext) context, body);
    } else {
      log.warn(
          "Unsupported context type for communications logging: " + context.getClass().getName());
    }
  }

  private <T> void logCommunications(T context, String body) {
    EventBuilder event = getEvent(context);
    if (event != null) {
      event.detail("type", EVENT_TYPE_COMMUNICATIONS).detail("msgBody", body).success();
    }
  }

  private EventBuilder getEvent(Object context) {
    if (context instanceof AuthenticationFlowContext) {
      return ((AuthenticationFlowContext) context).getEvent();
    } else if (context instanceof RequiredActionContext) {
      return ((RequiredActionContext) context).getEvent();
    }
    return null;
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

  public static String linkFromActionToken(
      KeycloakSession session, RealmModel realm, DefaultActionToken token) {
    UriInfo uriInfo = session.getContext().getUri();
    UriBuilder builder =
        actionTokenBuilder(
            uriInfo.getBaseUri(), token.serialize(session, realm, uriInfo), token.getIssuedFor());
    return builder.build(realm.getName()).toString();
  }

  UriBuilder actionTokenBuilder(URI baseUri, String tokenString, String clientId) {
    log.infof(
        "actionTokenBuilder(): baseUri: %s, tokenString: %s, clientId: %s",
        baseUri, tokenString, clientId);
    return Urls.realmBase(baseUri)
        .path(RealmsResource.class, "getLoginActionsService")
        .path(LoginActionsService.class, "executeActionToken")
        .queryParam(Constants.KEY, tokenString)
        .queryParam(Constants.CLIENT_ID, clientId);
  }

  String generateOTL(
      AuthenticationSessionModel authSession,
      KeycloakSession session,
      int ttl,
      String[] otlAuthNotesNames,
      String redirectUri,
      boolean isDeferredUser) {
    // Get necessary components from the context
    AuthenticationSessionCompoundId compoundId =
        AuthenticationSessionCompoundId.fromAuthSession(authSession);
    String sessionId = compoundId.getEncodedId();
    String userId =
        authSession.getAuthenticatedUser() == null
            ? authSession.getAuthNote(USER_ID)
            : authSession.getAuthenticatedUser().getId();
    RealmModel realm = authSession.getRealm();

    // Create the OTLActionToken with the necessary information
    OTLActionToken token =
        new OTLActionToken(
            userId,
            Time.currentTime() + ttl,
            sessionId, // Original compound session ID
            otlAuthNotesNames,
            isDeferredUser,
            redirectUri,
            authSession.getClient().getClientId());

    // Generate the OTL link
    return linkFromActionToken(session, realm, token);
  }
  ;

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

      Locale locale;
      if (user != null) {
        locale = session.getContext().resolveLocale(user);
      } else {
        locale = session.getContext().resolveLocale(null);
        if (locale == null) {
          String defaultLocale = realm.getDefaultLocale();
          if (defaultLocale != null) {
            locale = Locale.forLanguageTag(defaultLocale);
          } else {
            locale = Locale.getDefault();
          }
        }
      }
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

  protected String sendEmail(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      String subjectFormatKey,
      List<Object> subjectAttributes,
      String bodyTemplate,
      Map<String, Object> bodyAttributes,
      String address,
      boolean useEmailSender,
      String username)
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

      if (useEmailSender) {
        EmailSenderProvider emailSender = session.getProvider(EmailSenderProvider.class);
        emailSender.send(
            realm.getSmtpConfig(),
            address,
            emailTemplate.getSubject(),
            emailTemplate.getTextBody(),
            emailTemplate.getHtmlBody());

      } else {
        EmailTemplateProvider emailTemplateProvider =
            session.getProvider(EmailTemplateProvider.class);
        String realmName = getRealmName(realm);
        emailTemplateProvider.setRealm(realm).setUser(user).setAttribute("realmName", realmName);

        if (username != null && !username.isEmpty()) {
          emailTemplateProvider.setAttribute("username", username);
        }

        emailTemplateProvider.send(
            subjectFormatKey, subjectAttributes, bodyTemplate, bodyAttributes);
      }

      return String.format(
          "{\"to\": \"%s\", \"subject\": \"%s\", \"textBody\": \"%s\", \"htmlBody\": \"%s\"}",
          escapeJson(address),
          escapeJson(emailTemplate.getSubject()),
          escapeJson(emailTemplate.getTextBody()),
          escapeJson(emailTemplate.getHtmlBody() != null ? emailTemplate.getHtmlBody() : ""));
    } catch (EmailException e) {
      throw e;
    } catch (Exception e) {
      throw new EmailException("Failed to template email", e);
    }
  }

  // Sending Email Or SMS based on the enrollment
  public static void sendErrorNotificationToUser(
      KeycloakSession session, String realmId, Event event) throws EmailException, IOException {

    String email = event.getDetails().get("email");
    String mobileNumber = event.getDetails().get("sequent.read-only.mobile-number");

    boolean sendEmail = email != null && !email.isEmpty();
    boolean sendSms = !sendEmail && mobileNumber != null && !mobileNumber.isEmpty();

    if (sendEmail) {
      // Send email to the user
      sendErrorEmailToUser(session, realmId, email, event);
    }

    if (sendSms) {
      // Send SMS to the user
      sendErrorSmsToUser(session, realmId, mobileNumber, event);
    }
    // Send email to support
    sendSupportNotificationEmail(session, realmId, event);
  }

  // Sends an email to the user based on the event
  protected static void sendErrorEmailToUser(
      KeycloakSession session, String realmId, String email, Event event) throws EmailException {
    try {
      RealmModel realm = session.realms().getRealm(realmId);
      String errorCode = event.getDetails().get("code_id");

      Map<String, Object> attributes = new HashMap<>();
      attributes.put("errorCode", errorCode);

      List<Object> subjectAttributes = Collections.emptyList();

      sendEmail(
          session,
          realm,
          null,
          SEND_ERROR_EMAIL_SUBJECT,
          subjectAttributes,
          SEND_ERROR_EMAIL_FTL,
          attributes,
          email,
          /* useEmailSender */ true,
          /* username */ null);

      log.info("Error email sent to: " + email);
    } catch (EmailException error) {
      log.error("sendErrorEmailToUser(): Exception sending email", error);
      throw error;
    }
  }

  // Sending SMS to the user based on the event
  protected static void sendErrorSmsToUser(
      KeycloakSession session, String realmId, String mobileNumber, Event event)
      throws IOException {
    try {
      RealmModel realm = session.realms().getRealm(realmId);
      String errorCode = event.getDetails().get("code_id");

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      List<String> smsAttributes = ImmutableList.of(errorCode);

      smsSenderProvider.send(
          mobileNumber.trim(),
          SEND_REGISTER_FAILED_SMS_I18N_KEY,
          smsAttributes,
          realm,
          null,
          session);

      log.info("Error SMS sent to: " + mobileNumber);
    } catch (IOException e) {
      log.error("sendErrorSmsToUser(): Exception sending SMS", e);
      throw e;
    }
  }

  // Sending support email with event details
  protected static void sendSupportNotificationEmail(
      KeycloakSession session, String realmId, Event event) throws EmailException {
    try {
      RealmModel realm = session.realms().getRealm(realmId);

      String supportEmail = "no-reply@sequentech.io";

      Map<String, Object> attributes = new HashMap<>();
      attributes.put("event", event);

      List<Object> subjectAttributes = Collections.emptyList();

      sendEmail(
          session,
          realm,
          null,
          SEND_SUPPORT_ERROR_EMAIL_SUBJECT,
          subjectAttributes,
          SEND_SUPPORT_ERROR_EMAIL_FTL,
          attributes,
          supportEmail,
          /* useEmailSender */ true,
          /* username */ null);

      log.info("Support notification email sent to: " + supportEmail);
    } catch (EmailException error) {
      log.error("sendSupportNotificationEmail(): Exception sending email", error);
      throw error;
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
        return emailAddress != null && !emailAddress.isEmpty()
            ? obscureEmail(emailAddress)
            : obscurePhoneNumber(mobileNumber);
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

  /**
   * Gets the tenant id from the realm name
   *
   * @param session
   * @param realmId
   * @return Tenant id found in the realm name or null if it wasn't present
   */
  public String getTenantId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();

    // Regular expression to match a UUID pattern
    Pattern uuidPattern =
        Pattern.compile(
            "\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b");
    Matcher matcher = uuidPattern.matcher(realmName);

    // Find the first match
    return matcher.find() ? matcher.group() : null;
  }

  public String getElectionEventId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();
    String[] parts = realmName.split("event-");
    if (parts.length > 1) {
      return parts[1];
    }
    return null;
  }

  public String buildAuthUrl(KeycloakSession session, String realmId, String urlType) {
    String tenantId = getTenantId(session, realmId);
    String electionEventId = getElectionEventId(session, realmId);
    String baseUrl =
        session.getContext().getRealm().getClientByClientId("voting-portal").getRootUrl();
    if (baseUrl.endsWith("/")) {
      baseUrl = baseUrl.substring(0, baseUrl.length() - 1);
    }

    if (tenantId != null && electionEventId != null) {
      return String.format("%s/tenant/%s/event/%s/%s", baseUrl, tenantId, electionEventId, urlType);
    } else {
      log.warn("Tenant ID or Election Event ID is null");
      return null;
    }
  }

  public static String getClientName(Object context) {
    AuthenticationSessionModel authSession = null;
    if (context instanceof AuthenticationFlowContext) {
      authSession = ((AuthenticationFlowContext) context).getAuthenticationSession();
    } else if (context instanceof ActionTokenContext) {
      authSession = ((ActionTokenContext<?>) context).getAuthenticationSession();
    } else {
      throw new IllegalArgumentException("Unsupported context type");
    }
    return authSession.getClient().getName();
  }

  public static void sendConfirmation(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      Object context)
      throws EmailException, IOException {
    log.info("sendConfirmation(): start");

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
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
      messageAttributes.put("enrollmentUrl", buildAuthUrl(session, realm.getId(), "enroll"));
      messageAttributes.put("loginUrl", buildAuthUrl(session, realm.getId(), "login"));
      messageAttributes.put("isKiosk", getClientName(context).endsWith("-kiosk"));
      String textBody =
          sendEmail(
              session,
              realm,
              user,
              SEND_SUCCESS_EMAIL_SUBJECT,
              subjAttr,
              SEND_SUCCESS_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              false,
              username);
      communicationsLog(context, textBody);
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendConfirmation(): sending sms", username);
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      String url = buildAuthUrl(session, realm.getId(), "login");
      List<String> smsAttributes = ImmutableList.of(url, mobileNumber.trim());
      String smsTranslationKey =
          getClientName(context).endsWith("-kiosk")
              ? SEND_SUCCESS_SMS_I18N_KEY_KIOSK
              : SEND_SUCCESS_SMS_I18N_KEY;
      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTranslationKey, smsAttributes, realm, user, session);
      communicationsLog(context, formattedText);
    }
  }

  public static void sendConfirmationDiffPost(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      Object context)
      throws EmailException, IOException {
    log.info("sendConfirmationDiffPost(): start");

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = user.getEmail() != null ? user.getEmail() : mobileNumber;
    log.infov("sendConfirmationDiffPost(): username {0}", username);
    log.infov("sendConfirmationDiffPost(): messageCourier {0}", messageCourier);

    String email = user.getEmail();
    String embassy = user.getFirstAttribute("embassy");

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendConfirmationDiffPost(): sending email", username);
      log.infov("sendConfirmationDiffPost(): embassy {0}", embassy);
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realName);
      messageAttributes.put("username", username);
      messageAttributes.put("embassy", embassy);
      messageAttributes.put("enrollmentUrl", buildAuthUrl(session, realm.getId(), "enroll"));
      messageAttributes.put("loginUrl", buildAuthUrl(session, realm.getId(), "login"));

      String textBody =
          sendEmail(
              session,
              realm,
              user,
              SEND_SUCCESS_EMAIL_SUBJECT,
              subjAttr,
              SEND_SUCCESS_EMAIL_DIFF_POST_FTL,
              messageAttributes,
              email.trim(),
              false,
              username);
      communicationsLog(context, textBody);
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendConfirmation(): sending sms", username);

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      log.infov("sendCode(): Sending SMS to=`{0}`", mobileNumber.trim());
      String url = buildAuthUrl(session, realm.getId(), "login");
      List<String> smsAttributes = ImmutableList.of(url, mobileNumber.trim());
      String smsTranslationKey =
          getClientName(context).endsWith("-kiosk")
              ? SEND_SUCCESS_SMS_I18N_KEY_KIOSK
              : SEND_SUCCESS_SMS_I18N_KEY;
      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTranslationKey, smsAttributes, realm, user, session);
      communicationsLog(context, formattedText);
    }
  }

  public static String convertToString(
      HashMap<String, String> values, String separator, String format) {
    log.info("convertToString(): start");
    if (values == null || values.isEmpty()) {
      return ""; // Return an empty string if the map is null or empty
    }
    log.info("convertToString(): not empty");

    // Default format: "key: value"
    String entryFormat = (format != null && !format.isEmpty()) ? format : "%s: %s";

    return values.entrySet().stream()
        // Format each entry using String.format with the provided or default format
        .map(
            entry -> {
              String val = String.format(entryFormat, entry.getKey(), entry.getValue());
              log.infov("convertToString(): val={0}", val);
              return val;
            })
        // Join entries with the specified separator
        .collect(Collectors.joining(separator));
  }

  public static void sendManualCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      Object context)
      throws EmailException, IOException {
    log.info("sendManualCommunication(): start");

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = email != null ? email : mobileNumber;
    log.infov("sendManualCommunication(): username {0}", username);
    log.infov("sendManualCommunication(): messageCourier {0}", messageCourier);

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendManualCommunication(): sending email", username);
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("rejectReasonKey", rejectReasonKey);
      messageAttributes.put(
          "mismatchedFieldsPlain", convertToString(mismatchedFields, "\n", "- %s: %s"));
      messageAttributes.put(
          "mismatchedFieldsHtml",
          convertToString(mismatchedFields, "<br>", "- %s: <strong>%s</strong>"));

      String textBody =
          sendEmail(
              session,
              realm,
              null,
              SEND_PENDING_EMAIL_SUBJECT,
              subjAttr,
              SEND_PENDING_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              true,
              username);
      communicationsLog(context, textBody);
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendManualCommunication(): sending sms", username);

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendManualCommunication(): Sending SMS to=`{0}`", mobileNumber.trim());
      List<String> smsAttributes =
          ImmutableList.of(rejectReasonKey, convertToString(mismatchedFields, ", ", null));

      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), SEND_PENDING_SMS_I18N_KEY, smsAttributes, realm, null, session);
      communicationsLog(context, formattedText);
    }
  }

  public static void sendRejectCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      Object context)
      throws EmailException, IOException {
    log.info("sendRejectCommunication(): start");

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = email != null ? email : mobileNumber;
    log.infov("sendRejectCommunication(): username {0}", username);
    log.infov("sendRejectCommunication(): messageCourier {0}", messageCourier);

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendRejectCommunication(): sending email", username);
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("rejectReasonKey", rejectReasonKey);
      messageAttributes.put("missmatchedFields", convertToString(mismatchedFields, ", ", null));

      String textBody =
          sendEmail(
              session,
              realm,
              null,
              SEND_REJECT_EMAIL_SUBJECT,
              subjAttr,
              SEND_REJECT_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              true,
              username);
      communicationsLog(context, textBody);
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.infov("sendRejectCommunication(): sending sms", username);

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.infov("sendRejectCommunication(): Sending SMS to=`{0}`", mobileNumber.trim());
      List<String> smsAttributes =
          ImmutableList.of(rejectReasonKey, convertToString(mismatchedFields, ", ", null));

      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), SEND_REJECT_SMS_I18N_KEY, smsAttributes, realm, null, session);
      communicationsLog(context, formattedText);
    }
  }

  public void buildEventDetails(AuthenticationFlowContext context, String className) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    UserModel user = context.getUser();
    List<UPAttribute> realmsAttributesList = getRealmUserProfileAttributes(context.getSession());
    for (UPAttribute attribute : realmsAttributesList) {
      String authNoteValue = authSession.getAuthNote(attribute.getName());
      context.getEvent().detail(attribute.getName(), authNoteValue);
    }
    if (user != null) {
      context.getEvent().detail(USER_PROFILE_ATTRIBUTES, getUserAttributesString(user));
      context.getEvent().user(user.getId());
    } else {
      String userId = context.getAuthenticationSession().getAuthNote(USER_ID);
      context.getEvent().user(userId);
    }
    context.getEvent().detail(AUTHENTICATOR_CLASS_NAME, className);
  }

  public String getUserAttributesString(UserModel user) {
    Map<String, List<String>> attributes = user.getAttributes();
    ObjectMapper mapper = new ObjectMapper();
    ObjectNode attributesJson = mapper.createObjectNode();

    for (String attributeName : attributes.keySet()) {
      String value = attributes.get(attributeName).get(0);
      attributesJson.put(attributeName, value);
    }

    return attributesJson.toString();
  }

  public List<UPAttribute> getRealmUserProfileAttributes(KeycloakSession session) {
    UserProfileProvider userProfileProvider = session.getProvider(UserProfileProvider.class);
    UPConfig userProfileConfig = userProfileProvider.getConfiguration();
    return userProfileConfig.getAttributes();
  }
}
