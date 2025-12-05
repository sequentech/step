// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriBuilder;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.TimeUnit;
import lombok.extern.jbosslog.JBossLog;
import org.jboss.logging.Logger;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.actiontoken.resetcred.ResetCredentialsActionToken;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.DefaultActionTokenKey;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ResetCredentialNotification implements Authenticator, AuthenticatorFactory {

  private static final Logger logger = Logger.getLogger(ResetCredentialNotification.class);
  public static final String PROVIDER_ID = "reset-credential-notification";
  private static final String smsMessageKey = "forgotPassword.sms.text";
  public static final String TEL_USER_ATTRIBUTE = "sequent.read-only.mobile-number";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    UserModel user = context.getUser();
    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
    String username =
        authenticationSession.getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);

    // Handle missing user
    if (user == null) {
      String successMessageKey = (username != null && username.contains("@")) ? "email" : "sms";
      context.forkWithSuccessMessage(
          new FormMessage(null, "forgotPassword.success.display.message", successMessageKey));
      return;
    }

    String actionTokenUserId =
        authenticationSession.getAuthNote(DefaultActionTokenKey.ACTION_TOKEN_USER_ID);
    if (actionTokenUserId != null && Objects.equals(user.getId(), actionTokenUserId)) {
      logger.debugf("Skipping notification screen for user '%s'", user.getUsername());
      context.success();
      return;
    }

    EventBuilder event = context.getEvent();

    // Get the reset link
    int validityInSecs =
        context
            .getRealm()
            .getActionTokenGeneratedByUserLifespan(ResetCredentialsActionToken.TOKEN_TYPE);
    int absoluteExpirationInSecs = Time.currentTime() + validityInSecs;

    String authSessionEncodedId =
        AuthenticationSessionCompoundId.fromAuthSession(authenticationSession).getEncodedId();
    ResetCredentialsActionToken token =
        new ResetCredentialsActionToken(
            user.getId(),
            user.getEmail(),
            absoluteExpirationInSecs,
            authSessionEncodedId,
            authenticationSession.getClient().getClientId());

    String link =
        UriBuilder.fromUri(
                context.getActionTokenUrl(
                    token.serialize(
                        context.getSession(), context.getRealm(), context.getUriInfo())))
            .build()
            .toString();

    long expirationInMinutes = TimeUnit.SECONDS.toMinutes(validityInSecs);

    // Attempt to send notification
    String email = user.getEmail();
    boolean notificationSent = sendNotification(context, user, link, expirationInMinutes);

    if (notificationSent) {
      String successMessageKey = email != null ? "email" : "sms";
      event
          .clone()
          .event(EventType.SEND_RESET_PASSWORD)
          .user(user)
          .detail(Details.USERNAME, username)
          .success();
      context.forkWithSuccessMessage(
          new FormMessage(null, "forgotPassword.success.display.message", successMessageKey));
    } else {
      event
          .clone()
          .event(EventType.SEND_RESET_PASSWORD)
          .user(user)
          .detail(Details.USERNAME, username)
          .error(Errors.EMAIL_SEND_FAILED);
      Response challenge =
          context
              .form()
              .setError(Messages.EMAIL_SENT_ERROR)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR);
      context.failure(AuthenticationFlowError.INTERNAL_ERROR, challenge);
    }
  }

  private boolean sendNotification(
      AuthenticationFlowContext context, UserModel user, String link, long expirationInMinutes) {
    try {
      // Check for email
      String email = user.getEmail();
      if (email != null && !email.trim().isEmpty()) {
        context
            .getSession()
            .getProvider(EmailTemplateProvider.class)
            .setRealm(context.getRealm())
            .setUser(user)
            .setAuthenticationSession(context.getAuthenticationSession())
            .sendPasswordReset(link, expirationInMinutes);
        logger.infof("Reset link sent via email to user %s", user.getUsername());
        return true;
      }

      String phoneNumber = user.getFirstAttribute(TEL_USER_ATTRIBUTE);
      if (phoneNumber != null && !phoneNumber.trim().isEmpty()) {
        log.info("phone number found sending sms" + phoneNumber);
        KeycloakSession session = context.getSession();
        SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
        List<String> attributes = List.of(String.valueOf(expirationInMinutes), link);
        smsSenderProvider.send(
            phoneNumber, smsMessageKey, attributes, context.getRealm(), user, session);
        return true;
      }

      logger.warnf("No contact method available for user %s", user.getUsername());
    } catch (Exception e) {
      logger.error("Failed to send reset notification", e);
    }
    return false;
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
    return "Send Reset Notification (Email/SMS)";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
    return false;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return new AuthenticationExecutionModel.Requirement[] {
      AuthenticationExecutionModel.Requirement.REQUIRED
    };
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Send reset link to user via email or SMS.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return null;
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
