// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.Response;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.Utils.MessageCourier;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;

/**
 * Abstract base for Keycloak required actions that reset and verify a user's contact information
 * (email or mobile) using an OTP (One-Time Password) sent via a message courier (email or SMS).
 *
 * <p>Flow:
 *
 * <ol>
 *   <li>Prompts the user to enter a new contact value (email or mobile number).
 *   <li>Sends an OTP to the entered value using the configured courier.
 *   <li>Prompts the user to enter the OTP.
 *   <li>On successful verification, saves a credential and updates the user's contact info.
 * </ol>
 *
 * <p>All state is managed via AuthenticationSessionModel notes. Subclasses must provide:
 *
 * <ul>
 *   <li>getProviderId() - the required action provider ID
 *   <li>getNoteKey() - the session note key for the contact value (e.g., "email" or "mobile")
 *   <li>getCourier() - the message courier type (EMAIL or SMS)
 *   <li>getI18nPrefix() - the i18n prefix for message keys (e.g., "emailOtp" or "mobileOtp")
 *   <li>saveVerifiedValue() - logic to persist the verified contact value to the user
 * </ul>
 *
 * <p>Templates and i18n keys are fully generic and parameterized by the i18nPrefix.
 */
@JBossLog
public abstract class BaseResetMessageOTPRequiredAction implements RequiredActionProvider {
  static final String TOO_MANY_ATTEMPTS_KEY = "messageOtp.auth.tooManyAttempts";

  /**
   * Returns the FTL template for the contact entry form (email or mobile). Subclasses may override
   * to use a different template.
   */
  protected String getEntryFtl() {
    return "message-otp.enter-contact.ftl";
  }

  /**
   * Returns the FTL template for the OTP entry form. Subclasses may override to use a different
   * template.
   */
  protected String getOtpFtl() {
    return "message-otp.enter-otp.ftl";
  }

  /** Returns the required action provider ID (e.g., "email-otp-ra"). */
  protected abstract String getProviderId();

  /**
   * Returns the session note key for the contact value (e.g., "email" or "mobile"). Now takes
   * authSession for dynamic keys.
   */
  protected abstract String getNoteKey(AuthenticationSessionModel authSession);

  /** Returns the message courier type (EMAIL or SMS). */
  protected abstract Utils.MessageCourier getCourier();

  /** Returns the i18n prefix for message keys (e.g., "emailOtp" or "mobileOtp"). */
  protected abstract String getI18nPrefix();

  /**
   * Saves the verified contact value to the user (e.g., set email or mobile attribute).
   *
   * @param context RequiredActionContext
   * @param value The verified contact value
   */
  protected abstract void saveVerifiedValue(RequiredActionContext context, String value);

  /** Capitalizes the first letter of a string (utility for i18n key construction). */
  protected String capitalize(String s) {
    if (s == null || s.isEmpty()) return s;
    return s.substring(0, 1).toUpperCase() + s.substring(1);
  }

  /**
   * Entry point for the required action: shows either the contact entry or OTP form.
   *
   * @param context RequiredActionContext
   */
  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
    String noteKey = getNoteKey(authSession);
    String value = authSession.getAuthNote(noteKey);
    if (value == null) {
      // No contact value yet: prompt for entry
      context.challenge(createEntryForm(context, null, config));
    } else {
      // Contact value present: prompt for OTP
      context.challenge(createOTPForm(context, null, config));
    }
  }

  /**
   * Handles form submissions for both contact entry and OTP entry. Delegates to handleEntry or
   * handleOtpEntry.
   *
   * @param context RequiredActionContext
   */
  @Override
  public void processAction(RequiredActionContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
    String noteKey = getNoteKey(authSession);
    String value = authSession.getAuthNote(noteKey);
    if (value == null) {
      handleEntry(context, config, noteKey);
    } else {
      handleOtpEntry(context, value, config, noteKey);
    }
  }

  protected boolean isValidMobileNumber(String phoneNumber, AuthenticatorConfigModel config) {
    log.info("isValidMobileNumber phoneNumber = " + phoneNumber);
    List<String> validCountryCodes =
        Utils.getMultivalueString(
            config, Utils.VALID_COUNTRY_CODES, Utils.VALID_COUNTRY_CODES_DEFAULT);
    log.info("isValidMobileNumber validCountryCodes = " + validCountryCodes);
    if (null == phoneNumber) {
      return false;
    }
    if (null == validCountryCodes || validCountryCodes.isEmpty()) {
      return true;
    }

    String trimmedPhoneNumber = phoneNumber.trim();

    return validCountryCodes.stream()
        .anyMatch(countryCode -> trimmedPhoneNumber.startsWith(countryCode));
  }

  /**
   * Handles the contact entry step: validates, stores, and sends OTP to the contact value. Shows
   * error if invalid or sending fails.
   */
  protected void handleEntry(
      RequiredActionContext context, AuthenticatorConfigModel config, String noteKey) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    KeycloakSession session = context.getSession();
    String enteredValue = context.getHttpRequest().getDecodedFormParameters().getFirst("contact");
    if (!isValidInput(enteredValue)) {
      // Invalid input: show error
      context.challenge(
          createEntryForm(
              context,
              form -> form.setError(ErrorType.INVALID_INPUT.toString(getI18nPrefix())),
              config));
      return;
    }
    authSession.setAuthNote(noteKey, enteredValue);

    try {
      UserModel user = context.getUser();
      boolean deferredUser = true;
      MessageCourier courier = getCourier();
      log.info("handleEntry courier" + courier);
      if (MessageCourier.SMS == courier || MessageCourier.BOTH == courier) {
        String mobileNumber = Utils.getMobileNumber(config, user, authSession, deferredUser);
        if (!isValidMobileNumber(mobileNumber, config)) {
          authSession.removeAuthNote(noteKey);

          context.challenge(
              createEntryForm(
                  context,
                  form -> form.setError(ErrorType.INVALID_COUNTRY.toString(getI18nPrefix())),
                  config));
          return;
        }
      }
      // Send OTP code to the contact value
      Utils.sendCode(
          config,
          session,
          user,
          authSession,
          courier,
          deferredUser,
          /*isOtl*/ false,
          new String[0],
          context);
    } catch (Exception e) {
      // Sending failed: show error
      context.challenge(
          createEntryForm(
              context,
              form -> form.setError(ErrorType.SEND_ERROR.toString(getI18nPrefix())),
              config));
      return;
    }
    // Show OTP entry form
    context.challenge(createOTPForm(context, null, config));
  }

  private void handleOtpEntry(
      RequiredActionContext context,
      String value,
      AuthenticatorConfigModel config,
      String noteKey) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    KeycloakSession session = context.getSession();
    String change = context.getHttpRequest().getDecodedFormParameters().getFirst("changeValue");
    if ("true".equals(change)) {
      // User requested to change contact value
      clearEntry(authSession, noteKey);
      context.challenge(createEntryForm(context, null, config));
      return;
    }
    if ("true"
        .equals(
            context
                .getHttpRequest()
                .getDecodedFormParameters()
                .getFirst(MessageOTPAuthenticator.MESSENGER_STATUS_PARAM))) {
      Utils.refreshMessengerState(session, authSession);
      context.challenge(createOTPForm(context, null, config));
      return;
    }
    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);
    String codeLengthStr = getConfigValue(config, Utils.CODE_LENGTH, Utils.CODE_LENGTH_DEFAULT);
    int codeLength = Integer.parseInt(codeLengthStr);
    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst("code");
    if (resend != null && resend.equals("true")) {
      // Handle resend logic: only allow if enough time has passed
      if (!isSendAllowed(authSession, config)) {
        context.challenge(
            createOTPForm(
                context,
                form -> form.setError(ErrorType.RESEND_TIMER.toString(getI18nPrefix())),
                config));
        return;
      }
      MessageAttemptState state;
      try {
        UserModel user = context.getUser();
        boolean deferredUser = true;
        MessageCourier courier = getCourier();
        if (MessageCourier.SMS == courier || MessageCourier.BOTH == courier) {
          String mobileNumber = Utils.getMobileNumber(config, user, authSession, deferredUser);
          if (!isValidMobileNumber(mobileNumber, config)) {
            authSession.removeAuthNote(noteKey);
            context.challenge(
                createEntryForm(
                    context,
                    form -> form.setError(ErrorType.INVALID_COUNTRY.toString(getI18nPrefix())),
                    config));
            return;
          }
        }
        // Resend OTP code
        state =
            Utils.sendCode(
                config,
                session,
                user,
                authSession,
                courier,
                deferredUser,
                /*isOtl*/ false,
                new String[0],
                context);
      } catch (Exception e) {
        state = MessageAttemptState.FAILED;
      }
      if (state == MessageAttemptState.FAILED) {
        context.challenge(
            createOTPForm(
                context,
                form -> form.setError(ErrorType.SEND_ERROR.toString(getI18nPrefix())),
                config));
        return;
      }
      context.challenge(createOTPForm(context, null, config));
      return;
    }
    // A code invalidated by too many attempts, or one that was never sent, is only replaced by
    // asking for a new one.
    if (code == null && authSession.getAuthNote(Utils.CODE_ATTEMPTS) != null) {
      context.challenge(
          createOTPForm(context, form -> form.setError(TOO_MANY_ATTEMPTS_KEY), config));
      return;
    }
    if (code == null && Utils.sendFailed(authSession)) {
      context.challenge(
          createOTPForm(
              context,
              form -> form.setError(ErrorType.SEND_ERROR.toString(getI18nPrefix())),
              config));
      return;
    }
    // Validate OTP code
    if (enteredCode == null || code == null || ttl == null || enteredCode.length() != codeLength) {
      context.failure();
      return;
    }
    boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
    if (isValid) {
      // Check if code is expired
      if (Long.parseLong(ttl) < System.currentTimeMillis()) {
        context.challenge(
            createOTPForm(
                context,
                form -> form.setError(ErrorType.CODE_EXPIRED.toString(getI18nPrefix())),
                config));
        return;
      }

      Optional<String> refused = confirmVerified(context, value);
      if (refused.isPresent()) {
        authSession.removeAuthNote(Utils.CODE);
        context.challenge(createOTPForm(context, form -> form.setError(refused.get()), config));
        return;
      }

      // Check MAX_RECEIVER_REUSE limit
      String maxReuseStr = config != null ? config.getConfig().get(Utils.MAX_RECEIVER_REUSE) : null;
      if (maxReuseStr == null || maxReuseStr.trim().isEmpty()) {
        maxReuseStr = "1"; // Default value
      }
      int maxReuse = Integer.parseInt(maxReuseStr);
      if (maxReuse > 0) {
        int currentUsersWithSameValue = usersWithSameValue(context, value);
        if (currentUsersWithSameValue >= maxReuse) {
          context.challenge(
              createOTPForm(
                  context,
                  form -> form.setError(ErrorType.MAX_RECEIVER_REUSE.toString(getI18nPrefix())),
                  config));
          return;
        }
      }

      // Save credential and update user
      authSession.removeAuthNote(Utils.CODE);
      createCredential(context);
      saveVerifiedValue(context, value);
      context.getUser().removeRequiredAction(getProviderId());
      context.getAuthenticationSession().removeRequiredAction(getProviderId());
      context.success();
    } else {
      // Invalid code: show error. Wrong codes count against the code, like at sign-in.
      boolean exhausted =
          MessageOTPAuthenticator.registerFailedAttempt(
              authSession, MessageOTPAuthenticatorFactory.getConfigMap(config));
      String error =
          exhausted ? TOO_MANY_ATTEMPTS_KEY : ErrorType.CODE_INVALID.toString(getI18nPrefix());
      context.challenge(createOTPForm(context, form -> form.setError(error), config));
    }
  }

  /**
   * Whether a code may be sent now: no code was sent yet, the last send failed for certain, or the
   * resend timer of the last code elapsed.
   */
  protected boolean isSendAllowed(
      AuthenticationSessionModel authSession, AuthenticatorConfigModel config) {
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);
    return ttl == null
        || Utils.sendFailed(authSession)
        || Utils.isResendAllowed(
            ttl,
            getConfigValue(config, Utils.CODE_TTL, Utils.CODE_TTL_DEFAULT),
            getConfigValue(
                config, Utils.RESEND_ACTIVATION_TIMER, Utils.RESEND_ACTIVATION_TIMER_DEFAULT),
            System.currentTimeMillis());
  }

  /** Forgets the entered contact so the voter can enter another one. */
  protected void clearEntry(AuthenticationSessionModel authSession, String noteKey) {
    authSession.removeAuthNote(noteKey);
  }

  /**
   * A last check once the code was verified, before anything is saved.
   *
   * @return the message key of the reason the contact is refused, or empty to save it
   */
  protected Optional<String> confirmVerified(RequiredActionContext context, String value) {
    return Optional.empty();
  }

  /** How many voters already use the verified contact. */
  protected int usersWithSameValue(RequiredActionContext context, String value) {
    return countUsersWithSameValue(context, value, getCourier());
  }

  /** Creates the message OTP credential the first time a contact is verified. */
  protected void createCredential(RequiredActionContext context) {
    MessageOTPCredentialProvider credentialProvider =
        new MessageOTPCredentialProvider(context.getSession());
    credentialProvider.createCredential(
        context.getRealm(),
        context.getUser(),
        MessageOTPCredentialModel.create(/* isSetup= */ true));
  }

  /** Attributes a subclass adds to the OTP entry form. */
  protected void decorateOtpForm(RequiredActionContext context, LoginFormsProvider form) {}

  /**
   * Validates the contact value. Default: non-null, non-empty. Subclasses can override for stricter
   * validation.
   */
  protected boolean isValidInput(String value) {
    return value != null && !value.trim().isEmpty();
  }

  /**
   * Counts the number of users in the realm that have the same contact value (email or phone
   * number) as the provided value.
   *
   * @param context The required action context
   * @param value The contact value to check for
   * @return The number of users with the same contact value
   */
  private int countUsersWithSameValue(
      RequiredActionContext context, String value, Utils.MessageCourier courier) {
    int count = 0;

    if (courier == Utils.MessageCourier.EMAIL || courier == Utils.MessageCourier.BOTH) {
      // Search for users and filter by email match
      Map<String, String> params = new HashMap<>();
      params.put(UserModel.SEARCH, value);

      count +=
          (int)
              context
                  .getSession()
                  .users()
                  .searchForUserStream(context.getRealm(), params, null, null)
                  .filter(user -> value.equals(user.getEmail()))
                  .count();
    }

    if (courier == Utils.MessageCourier.SMS || courier == Utils.MessageCourier.BOTH) {
      count +=
          (int)
              context
                  .getSession()
                  .users()
                  .searchForUserByUserAttributeStream(
                      context.getRealm(), Utils.PHONE_NUMBER_ATTRIBUTE, value)
                  .count();
    }

    return count;
  }

  /** Creates the contact entry form, setting the i18nPrefix for the template. */
  protected Response createEntryForm(
      RequiredActionContext context,
      Consumer<LoginFormsProvider> formConsumer,
      AuthenticatorConfigModel config) {
    LoginFormsProvider form = context.form();
    form.setAttribute("i18nPrefix", getI18nPrefix());
    if (formConsumer != null) {
      formConsumer.accept(form);
    }
    return form.createForm(getEntryFtl());
  }

  /**
   * Reads a config value, falling back to the default when the realm's saved config has no entry
   * for the key: the defaults declared in {@link MessageOTPAuthenticatorFactory} are applied by the
   * admin console only, so a realm configured before a key existed simply doesn't have it, and the
   * OTP template crashes with InvalidReferenceException on a missing attribute.
   */
  protected String getConfigValue(
      AuthenticatorConfigModel config, String key, String defaultValue) {
    Map<String, String> configMap = config == null ? null : config.getConfig();
    String value = configMap == null ? null : configMap.get(key);
    return value == null ? defaultValue : value;
  }

  /** Creates the OTP entry form, setting all required attributes for the template. */
  protected Response createOTPForm(
      RequiredActionContext context,
      Consumer<LoginFormsProvider> formConsumer,
      AuthenticatorConfigModel config) {
    LoginFormsProvider form = context.form();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String noteKey = getNoteKey(authSession);
    String codeLength = getConfigValue(config, Utils.CODE_LENGTH, Utils.CODE_LENGTH_DEFAULT);
    String resendTimer =
        getConfigValue(
            config, Utils.RESEND_ACTIVATION_TIMER, Utils.RESEND_ACTIVATION_TIMER_DEFAULT);
    form.setAttribute("contact", authSession.getAuthNote(noteKey));
    form.setAttribute("codeLength", codeLength);
    form.setAttribute("resendTimer", resendTimer);
    form.setAttribute("ttl", getConfigValue(config, Utils.CODE_TTL, Utils.CODE_TTL_DEFAULT));
    form.setAttribute("codeJustSent", !Utils.sendFailed(authSession));
    form.setAttribute("i18nPrefix", getI18nPrefix());
    decorateOtpForm(context, form);
    if (formConsumer != null) {
      formConsumer.accept(form);
    }
    return form.createForm(getOtpFtl());
  }

  /** Enum for error types, serialized to string for FTL template usage. */
  protected enum ErrorType {
    INVALID_INPUT(".auth.error.invalidInput"),
    SEND_ERROR(".auth.error.sendError"),
    RESEND_TIMER(".auth.error.resendTimer"),
    CODE_EXPIRED(".auth.error.codeExpired"),
    CODE_INVALID(".auth.error.codeInvalid"),
    MAX_RECEIVER_REUSE(".auth.error.maxReceiverReuse"),
    INVALID_COUNTRY(".auth.error.invalidCountry");

    private final String value;

    ErrorType(String value) {
      this.value = value;
    }

    public String toString(String prefix) {
      return prefix + value;
    }
  }

  @Override
  public void close() {}
}
