// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.security.SecureRandom;
import java.util.*;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.*;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailException;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.*;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.ServicesLogger;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class SendNewPassword implements Authenticator, AuthenticatorFactory {

  public static final String PROVIDER_ID = "forgot-password-send-new-password";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    UserModel user = context.getUser();
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
    String attemptedUsername =
        authenticationSession.getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
    String attemptedEmail = authenticationSession.getAuthNote(Utils.ATTEMPTED_EMAIL);

    // we don't want people guessing usernames, so if there was a problem
    // obtaining the user, the user will be null. just reset login for with
    // a success message
    if (user == null) {
      context.forkWithSuccessMessage(new FormMessage(Messages.EMAIL_SENT));
      return;
    }

    String actionTokenUserId =
        authenticationSession.getAuthNote(DefaultActionTokenKey.ACTION_TOKEN_USER_ID);
    if (actionTokenUserId != null && Objects.equals(user.getId(), actionTokenUserId)) {
      log.infov(
          "Forget-password triggered when reauthenticating user after authentication via action token. Skipping {0} screen and using user '{1}' ",
          PROVIDER_ID, user.getUsername());
      context.success();
      return;
    }

    EventBuilder event = context.getEvent();
    String userEmail = user.getEmail();

    // we don't want people guessing usernames, so if there is a problem, just continuously
    // challenge
    if (userEmail == null || userEmail.trim().length() == 0) {
      event.user(user).detail(Details.USERNAME, attemptedUsername).error(Errors.INVALID_EMAIL);

      context.forkWithSuccessMessage(new FormMessage(Messages.EMAIL_SENT));
      return;
    }

    // Set password
    String temporaryPassword = createTemporaryPassword(context);
    event.event(EventType.UPDATE_PASSWORD);
    user.credentialManager().updateCredential(UserCredentialModel.password(temporaryPassword));
    int expirationSeconds = Utils.getPasswordExpirationSeconds(config);

    // Mark password to expire
    String expirationUserAttribute = Utils.getPasswordExpirationUserAttribute(config);
    int absoluteExpirationInSecs = Time.currentTime() + expirationSeconds;
    user.setSingleAttribute(expirationUserAttribute, String.valueOf(absoluteExpirationInSecs));

    // Send email with password
    try {
      Utils.sendNewPasswordNotification(context.getSession(), user, temporaryPassword);
      event
          .clone()
          .event(EventType.SEND_RESET_PASSWORD)
          .user(user)
          .detail(Details.USERNAME, attemptedUsername)
          .detail(Details.EMAIL, attemptedEmail)
          .detail(Details.CODE_ID, authenticationSession.getParentSession().getId())
          .success();
      context.forkWithSuccessMessage(new FormMessage(Messages.EMAIL_SENT));
    } catch (EmailException error) {
      event
          .clone()
          .event(EventType.SEND_RESET_PASSWORD)
          .detail(Details.USERNAME, attemptedUsername)
          .user(user)
          .error(Errors.EMAIL_SEND_FAILED);
      ServicesLogger.LOGGER.failedToSendPwdResetEmail(error);
      Response challenge =
          context
              .form()
              .setError(Messages.EMAIL_SENT_ERROR)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR);
      context.failure(AuthenticationFlowError.INTERNAL_ERROR, challenge);
    }
  }

  private String createTemporaryPassword(AuthenticationFlowContext context) {
    RealmModel realm = context.getRealm();
    UserModel user = context.getUser();
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    int passwordLength = Utils.getPasswordLength(authConfig);
    String charList = Utils.getPasswordChars(authConfig);
    PasswordPolicyManagerProvider policyManager =
        context.getSession().getProvider(PasswordPolicyManagerProvider.class);

    String generatedPassword;
    do {
      generatedPassword = generateRandomPassword(charList, passwordLength);
    } while (policyManager.validate(realm, user, generatedPassword) != null);

    return generatedPassword;
  }

  private String generateRandomPassword(String charList, int length) {
    SecureRandom random = new SecureRandom();
    StringBuilder sb = new StringBuilder();

    for (int i = 0; i < length; i++) {
      int randomIndex = random.nextInt(charList.length());
      sb.append(charList.charAt(randomIndex));
    }

    return sb.toString();
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    context.getUser().setEmailVerified(true);
    context.success();
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public String getDisplayType() {
    return "Send Forgot Password Email";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  public static final AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Send email to user with new temporal password.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            Utils.PASSWORD_CHARS,
            "Allowed password characters",
            "List of characters used for generating the random password",
            ProviderConfigProperty.STRING_TYPE,
            Utils.PASSWORD_CHARS_DEFAULT),
        new ProviderConfigProperty(
            Utils.PASSWORD_LENGTH,
            "Number of characters",
            "Number of characters to use in the temporary password",
            ProviderConfigProperty.STRING_TYPE,
            Utils.PASSWORD_LENGTH_DEFAULT),
        new ProviderConfigProperty(
            Utils.PASSWORD_EXPIRATION_SECONDS,
            "Password Expiration (seconds)",
            "Password Expiration in seconds",
            ProviderConfigProperty.STRING_TYPE,
            Utils.PASSWORD_EXPIRATION_SECONDS_DEFAULT),
        new ProviderConfigProperty(
            Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE,
            "User attribute for Password Expiration Date",
            "User attribute to use storing the Password Expiration Date. Should be read-only.",
            ProviderConfigProperty.STRING_TYPE,
            Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT));
  }

  @Override
  public void close() {}

  @Override
  public Authenticator create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
