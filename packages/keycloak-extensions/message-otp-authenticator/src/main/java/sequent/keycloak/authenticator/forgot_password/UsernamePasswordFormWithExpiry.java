// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.keycloak.services.validation.Validation.FIELD_USERNAME;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.*;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.common.util.Time;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.ModelDuplicateException;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.ServicesLogger;
import org.keycloak.services.managers.AuthenticationManager;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.validation.Validation;

/**
 * This is just like the normal Username Password form, except with more features:
 *
 * <ul>
 *   <li>It allows to check if the password has expired or not.
 *   <li>It supports recaptcha v3 for login.
 *   <li>It allows to look up the user using one or more username attributes (for example username
 *       and tlf).
 * </ul>
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class UsernamePasswordFormWithExpiry extends AbstractUsernameFormAuthenticator
    implements Authenticator, AuthenticatorFactory {
  public static final String PROVIDER_ID = "expiry-username-password-form";
  public static final UsernamePasswordFormWithExpiry SINGLETON =
      new UsernamePasswordFormWithExpiry();
  public static final Requirement[] REQUIREMENT_CHOICES = {Requirement.REQUIRED};

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action()");
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    if (formData.containsKey("cancel")) {
      context.cancelLogin();
      return;
    }
    if (!validateForm(context, formData)) {
      return;
    }
    context.success();
  }

  protected boolean validateForm(
      AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    log.info("validateForm()");
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    boolean recaptchaEnabled =
        Utils.getBoolean(authConfig, Utils.RECAPTCHA_ENABLED_ATTRIBUTE, false);
    boolean recaptchaValidated = false;
    if (recaptchaEnabled) {
      String recaptchaSiteSecret =
          Utils.getString(authConfig, Utils.RECAPTCHA_SITE_SECRET_ATTRIBUTE).strip();
      Double recaptchaMinScore =
          Double.parseDouble(
              Utils.getString(authConfig, Utils.RECAPTCHA_MIN_SCORE_ATTRIBUTE, "1").strip());
      String captchaResponse = formData.getFirst(Utils.RECAPTCHA_G_RESPONSE);
      if (!Validation.isBlank(captchaResponse)) {
        recaptchaValidated =
            Utils.validateRecaptcha(
                context,
                recaptchaValidated,
                captchaResponse,
                recaptchaSiteSecret,
                recaptchaMinScore);
        log.infov("validateForm(): recaptchaValidated={0}", recaptchaValidated);
      }
    }

    if (recaptchaEnabled && !recaptchaValidated) {
      log.info("validateForm(): invalid recaptcha");
      formData.remove(Utils.RECAPTCHA_G_RESPONSE);
      context.failureChallenge(
          AuthenticationFlowError.INVALID_CREDENTIALS,
          context
              .form()
              .setError(Messages.RECAPTCHA_FAILED)
              .createErrorPage(Response.Status.BAD_REQUEST));
      return false;
    }

    boolean hideUserNotFound =
        Utils.getBoolean(context.getAuthenticatorConfig(), Utils.HIDE_USER_NOT_FOUND, false);

    if (!validateUserAndPassword(context, formData)) {
      log.info("validateForm(): invalid form");
      // We don't call context.failureChallenge() here because
      // validateUserAndPassword() already does that
      return false;
    }
    // If we reach here, password was validated. But now we need to check if
    // there's an user expiration attribute and if so, if it has expired
    // already. But of course, this only makese sense if password is not
    // disabled.
    boolean disablePassword = getDisablePassword(context);
    if (disablePassword) {
      return true;
    }

    UserModel user = getUser(context, formData);
    if (user == null) {
      if (!hideUserNotFound) {
        // should not happen. We have validated the form, so we should have
        // found both the username/email and password to be valid!
        log.info("validateForm(): user not found - should not happen");
        context.failureChallenge(
            AuthenticationFlowError.INTERNAL_ERROR,
            context.form().createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
        return false;
      } else {
        String username = formData.getFirst(AuthenticationManager.FORM_USERNAME).trim();
        EventBuilder event = context.getEvent();
        event.clone().detail(Details.USERNAME, username).error(Errors.USER_NOT_FOUND);
        context.clearUser();
        context.success();
      }
    }

    // get the user attribute name
    String passwordExpirationUserAttribute =
        Utils.getPasswordExpirationUserAttribute(context.getAuthenticatorConfig());
    if (passwordExpirationUserAttribute == null) {
      // shouldn't happen since we have a fall-back user attribute name
      log.info(
          "validateForm(): password expiration user attribute configuration is null - should not happen - return true");
      return true;
    }
    String passwordExpiration = user.getFirstAttribute(passwordExpirationUserAttribute);
    if (passwordExpiration == null) {
      // if password expiration is null it means the user doesn't have this
      // attribute set, and thus we can ignore and return true
      log.info("validateForm(): password expiration not set - return true");
      return true;
    }
    int passwordExpirationInt = Integer.valueOf(passwordExpiration);
    int currentTime = Time.currentTime();
    if (currentTime > passwordExpirationInt) {
      // the user has an expired password
      log.infov(
          "validateForm(): expired password, currentTime[{0}] > passwordExpirationInt[{1}]",
          currentTime, passwordExpirationInt);
      context.failureChallenge(
          AuthenticationFlowError.INVALID_CREDENTIALS,
          context
              .form()
              .setError(Messages.INVALID_PASSWORD)
              .createErrorPage(Response.Status.BAD_REQUEST));
      return false;
    }

    return true;
  }

  @Override
  public boolean validateUserAndPassword(
      AuthenticationFlowContext context, MultivaluedMap<String, String> inputData) {
    UserModel user = getUser(context, inputData);
    boolean shouldClearUserFromCtxAfterBadPassword =
        !isUserAlreadySetBeforeUsernamePasswordAuth(context);
    boolean disablePassword = getDisablePassword(context);

    return validateUser(context, user, inputData)
        && (disablePassword
            || validatePassword(context, user, inputData, shouldClearUserFromCtxAfterBadPassword));
  }

  @Override
  public boolean validateUser(
      AuthenticationFlowContext context, MultivaluedMap<String, String> inputData) {
    UserModel user = getUser(context, inputData);
    return user != null && validateUser(context, user, inputData);
  }

  private boolean validateUser(
      AuthenticationFlowContext context, UserModel user, MultivaluedMap<String, String> inputData) {
    boolean hideUserNotFound =
        Utils.getBoolean(context.getAuthenticatorConfig(), Utils.HIDE_USER_NOT_FOUND, false);
    if (!hideUserNotFound && user == null) {
      return false;
    }
    if (!hideUserNotFound && !enabledUser(context, user)) {
      return false;
    }

    String rememberMe = inputData.getFirst("rememberMe");
    boolean remember =
        context.getRealm().isRememberMe()
            && rememberMe != null
            && rememberMe.equalsIgnoreCase("on");
    if (remember) {
      context.getAuthenticationSession().setAuthNote(Details.REMEMBER_ME, "true");
      context.getEvent().detail(Details.REMEMBER_ME, "true");
    } else {
      context.getAuthenticationSession().removeAuthNote(Details.REMEMBER_ME);
    }
    context.setUser(user);
    return true;
  }

  private UserModel getUser(
      AuthenticationFlowContext context, MultivaluedMap<String, String> inputData) {
    if (isUserAlreadySetBeforeUsernamePasswordAuth(context)) {
      // Get user from the authentication context in case he was already set before this
      // authenticator
      UserModel user = context.getUser();
      testInvalidUser(context, user);
      return user;
    } else {
      // Normal login. In this case this authenticator is supposed to establish identity of the user
      // from the provided username
      return getUserFromForm(context, inputData);
    }
  }

  private UserModel getUserFromForm(
      AuthenticationFlowContext context, MultivaluedMap<String, String> inputData) {
    log.info("getUserFromForm(): start");
    String username = inputData.getFirst(AuthenticationManager.FORM_USERNAME);
    if (username == null || username.isEmpty()) {
      context.getEvent().error(Errors.USER_NOT_FOUND);
      Response challengeResponse =
          challenge(context, getDefaultChallengeMessage(context), FIELD_USERNAME);
      context.failureChallenge(AuthenticationFlowError.INVALID_USER, challengeResponse);
      return null;
    }

    // remove leading and trailing whitespace
    username = username.trim().toLowerCase();

    context.getEvent().detail(Details.USERNAME, username);
    context
        .getAuthenticationSession()
        .setAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME, username);

    UserModel user = null;
    try {
      List<String> usernameAttributes =
          Utils.getMultivalueString(
              context.getAuthenticatorConfig(),
              Utils.USERNAME_ATTRIBUTES,
              Utils.USERNAME_ATTRIBUTES_DEFAULT);
      user = findUser(context.getSession(), context.getRealm(), username, usernameAttributes);
    } catch (ModelDuplicateException mde) {
      ServicesLogger.LOGGER.modelDuplicateException(mde);

      // Could happen during federation import
      if (mde.getDuplicateFieldName() != null
          && mde.getDuplicateFieldName().equals(UserModel.EMAIL)) {
        setDuplicateUserChallenge(
            context,
            Errors.EMAIL_IN_USE,
            Messages.EMAIL_EXISTS,
            AuthenticationFlowError.INVALID_USER);
      } else {
        setDuplicateUserChallenge(
            context,
            Errors.USERNAME_IN_USE,
            Messages.USERNAME_EXISTS,
            AuthenticationFlowError.INVALID_USER);
      }

      return user;
    }

    // Read our new boolean config
    boolean hideUserNotFound =
        Utils.getBoolean(context.getAuthenticatorConfig(), Utils.HIDE_USER_NOT_FOUND, false);
    if (!hideUserNotFound) {
      testInvalidUser(context, user);
    }
    return user;
  }

  private UserModel findUser(
      KeycloakSession session, RealmModel realm, String username, List<String> usernameAttributes) {
    if (usernameAttributes != null && !usernameAttributes.isEmpty()) {
      for (String attribute : usernameAttributes) {
        // If email is one of the attributes use specific query
        if ("email".equalsIgnoreCase(attribute) && username.indexOf('@') != -1) {
          UserModel user = session.users().getUserByEmail(realm, username);
          if (user != null) {
            return user;
          }
          continue;
        }

        // If username is one of the attributes use specific query
        if ("username".equalsIgnoreCase(attribute)) {
          UserModel user = session.users().getUserByUsername(realm, username);
          if (user != null) {
            return user;
          }
          continue;
        }

        UserModel user =
            session
                .users()
                .searchForUserByUserAttributeStream(realm, attribute, username)
                .findFirst()
                .orElse(null);

        if (user != null) {
          return user;
        }
      }
    }

    return null;
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("action()");
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    String loginHint =
        context.getAuthenticationSession().getClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM);
    String rememberMeUsername = AuthenticationManager.getRememberMeUsername(context.getSession());

    if (context.getUser() != null) {
      LoginFormsProvider form = context.form();
      form.setAttribute(LoginFormsProvider.USERNAME_HIDDEN, true);
      form.setAttribute(LoginFormsProvider.REGISTRATION_DISABLED, true);
      context
          .getAuthenticationSession()
          .setAuthNote(USER_SET_BEFORE_USERNAME_PASSWORD_AUTH, "true");
    } else {
      context.getAuthenticationSession().removeAuthNote(USER_SET_BEFORE_USERNAME_PASSWORD_AUTH);
      if (loginHint != null || rememberMeUsername != null) {
        if (loginHint != null) {
          formData.add(AuthenticationManager.FORM_USERNAME, loginHint);
        } else {
          formData.add(AuthenticationManager.FORM_USERNAME, rememberMeUsername);
          formData.add("rememberMe", "on");
        }
      }
    }
    Response challengeResponse = challenge(context, formData);
    context.challenge(challengeResponse);
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  protected Response challenge(
      AuthenticationFlowContext context, MultivaluedMap<String, String> formData) {
    boolean disablePassword = getDisablePassword(context);

    LoginFormsProvider form = context.form();
    Utils.addRecaptchaChallenge(context, formData);

    if (formData.size() > 0) {
      form.setFormData(formData);
    }

    if (disablePassword) {
      return form.createPasswordReset();
    } else {
      return form.createLoginUsernamePassword();
    }
  }

  protected boolean getDisablePassword(AuthenticationFlowContext context) {
    Map<String, String> config = context.getAuthenticatorConfig().getConfig();
    String disablePasswordString = config.get(Utils.DISABLE_PASSWORD_ATTRIBUTE);
    return disablePasswordString != null && disablePasswordString.equals("true");
  }

  @Override
  protected Response challenge(AuthenticationFlowContext context, String error, String field) {
    boolean disablePassword = getDisablePassword(context);

    LoginFormsProvider form = context.form().setExecution(context.getExecution().getId());

    if (error != null) {
      if (field != null) {
        form.addError(new FormMessage(field, error));
      } else {
        form.setError(error);
      }
    }

    if (disablePassword) {
      return form.createPasswordReset();
    } else {
      return form.createLoginUsernamePassword();
    }
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    // never called
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // never called
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getReferenceCategory() {
    return PasswordCredentialModel.TYPE;
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public String getDisplayType() {
    return "Username Password Form - allowing password expiration";
  }

  @Override
  public String getHelpText() {
    return "Validates a username and password from login form. Also checks if the password has expired.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            Utils.USERNAME_ATTRIBUTES,
            "User attributes used as username",
            "User attributes used as username. For Example: email or phone number",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            Utils.USERNAME_ATTRIBUTES_DEFAULT),
        new ProviderConfigProperty(
            Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE,
            "User attribute for Password Expiration Date",
            "User attribute to use storing the Password Expiration Date. Should be read-only.",
            ProviderConfigProperty.STRING_TYPE,
            Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT),
        new ProviderConfigProperty(
            Utils.DISABLE_PASSWORD_ATTRIBUTE,
            "Disable Password Field",
            "Just enter the username field. Used for example as the form in Forgot Password",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            Utils.RECAPTCHA_SITE_KEY_ATTRIBUTE,
            "reCAPTCHA v3 Site Key",
            "",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            Utils.RECAPTCHA_SITE_SECRET_ATTRIBUTE,
            "reCAPTCHA v3 Site Secret",
            "",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            Utils.RECAPTCHA_MIN_SCORE_ATTRIBUTE,
            "reCAPTCHA v3 Minimum Score",
            "",
            ProviderConfigProperty.STRING_TYPE,
            "0.5"),
        new ProviderConfigProperty(
            Utils.RECAPTCHA_ACTION_NAME_ATTRIBUTE,
            "reCAPTCHA v3 Action Name",
            "",
            ProviderConfigProperty.STRING_TYPE,
            Utils.RECAPTCHA_ACTION_NAME_ATTRIBUTE_DEFAULT),
        new ProviderConfigProperty(
            Utils.RECAPTCHA_ENABLED_ATTRIBUTE,
            "Enable reCAPTCHA v3",
            "",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            Utils.HIDE_USER_NOT_FOUND,
            "Hide 'User Not Found' Error",
            "If enabled, will show a generic error even if user does not exist, preventing user enumeration attacks.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false));
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }
}
