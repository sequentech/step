// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.UriBuilder;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.TimeUnit;
import lombok.extern.jbosslog.JBossLog;
import org.jboss.logging.Logger;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.actiontoken.resetcred.ResetCredentialsActionToken;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailTemplateProvider;
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
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.NeutralPace;
import sequent.keycloak.authenticator.Utils.MessageCourier;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.NoticeRecipient;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ResetCredentialNotification implements Authenticator, AuthenticatorFactory {

  private static final Logger logger = Logger.getLogger(ResetCredentialNotification.class);
  public static final String PROVIDER_ID = "reset-credential-notification";
  public static final String RESET_LINK_MESSAGE_KEY = "forgotPassword.sms.text";

  /** "If an account exists, we sent the link to its contact." */
  static final String SUCCESS_NEUTRAL_MESSAGE_KEY = "forgotPassword.success.contact.message";

  public static final String TEL_USER_ATTRIBUTE = "sequent.read-only.mobile-number";

  /**
   * The page never tells whether an account exists: the same message, and no sooner than the
   * default {@link NeutralPace} floor after the request began, with or without one (R7-06).
   */
  @Override
  public void authenticate(AuthenticationFlowContext context) {
    long start = NeutralPace.start();
    boolean neutral = true;
    try {
      neutral = sendResetLink(context);
    } finally {
      if (neutral) {
        NeutralPace.hold(start, NeutralPace.floor(null));
      }
    }
  }

  /**
   * @return whether the answer is the neutral page (not the action-token shortcut, which only a
   *     known account reaches and which isn't held)
   */
  private boolean sendResetLink(AuthenticationFlowContext context) {
    UserModel user = context.getUser();
    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();

    // Unknown users get the same page as known ones: the page never tells whether an account
    // exists, nor how it is reached.
    if (user == null) {
      context.forkWithSuccessMessage(successMessage());
      return true;
    }

    String actionTokenUserId =
        authenticationSession.getAuthNote(DefaultActionTokenKey.ACTION_TOKEN_USER_ID);
    if (actionTokenUserId != null && Objects.equals(user.getId(), actionTokenUserId)) {
      logger.debugf("Skipping notification screen for user '%s'", user.getId());
      context.success();
      return false;
    }

    EventBuilder event = context.getEvent();

    int validityInSecs =
        context
            .getRealm()
            .getActionTokenGeneratedByUserLifespan(ResetCredentialsActionToken.TOKEN_TYPE);
    String link = resetLink(context, user, validityInSecs);
    long expirationInMinutes = TimeUnit.SECONDS.toMinutes(validityInSecs);

    // The link goes to the voter's one channel; a refusal or a failure is recorded with its reason
    // and never sent elsewhere. The page is the same whatever happened.
    Optional<String> notSent = sendNotification(context, user, link, expirationInMinutes);
    EventBuilder resetEvent = event.clone().event(EventType.SEND_RESET_PASSWORD).user(user);
    if (notSent.isEmpty()) {
      resetEvent.success();
    } else {
      resetEvent.detail("reason", notSent.get()).error(Errors.EMAIL_SEND_FAILED);
    }
    context.forkWithSuccessMessage(successMessage());
    return true;
  }

  /** The reset link: an action token for this user and authentication session. */
  protected String resetLink(
      AuthenticationFlowContext context, UserModel user, int validityInSecs) {
    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
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
    return UriBuilder.fromUri(
            context.getActionTokenUrl(
                token.serialize(context.getSession(), context.getRealm(), context.getUriInfo())))
        .build()
        .toString();
  }

  /**
   * What the voter is told after asking for a reset link, whatever happened: it names neither the
   * channel nor whether an account exists.
   */
  public static FormMessage successMessage() {
    return new FormMessage(null, SUCCESS_NEUTRAL_MESSAGE_KEY);
  }

  /**
   * Sends the link on the voter's channel.
   *
   * @return empty when it was handed over (harvest took it or may have sent it, or Keycloak's
   *     provider sent it); otherwise the reason it wasn't, which is logged
   */
  private Optional<String> sendNotification(
      AuthenticationFlowContext context, UserModel user, String link, long expirationInMinutes) {
    try {
      KeycloakSession session = context.getSession();
      sequent.keycloak.authenticator.Utils.NoticeOutcome outcome =
          sequent.keycloak.authenticator.Utils.sendNotice(
              session,
              context.getRealm(),
              user,
              NoticeRecipient.fromUser(user, TEL_USER_ATTRIBUTE),
              MessageCourier.CHOSEN,
              user.getEmail(),
              user.getFirstAttribute(TEL_USER_ATTRIBUTE),
              RESET_LINK_MESSAGE_KEY,
              () -> List.of(String.valueOf(expirationInMinutes), link),
              context);
      if (outcome.keycloak().isEmpty()) {
        if (!outcome.handedOver()) {
          logger.warnf("The reset link was not sent: %s", outcome.failureReason());
          return Optional.of(outcome.failureReason());
        }
        return Optional.empty();
      }
      MessageCourier courier = outcome.keycloak().get().courier();
      if (courier == MessageCourier.EMAIL) {
        session
            .getProvider(EmailTemplateProvider.class)
            .setRealm(context.getRealm())
            .setUser(user)
            .setAuthenticationSession(context.getAuthenticationSession())
            .sendPasswordReset(link, expirationInMinutes);
        logger.info("Reset link sent by email");
        return Optional.empty();
      }
      if (courier == MessageCourier.SMS) {
        SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
        if (smsSenderProvider.sendsOnlyCodes()) {
          logger.warnf(
              "The reset link was not sent: %s",
              MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
          return Optional.of(MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
        }
        List<String> attributes = List.of(String.valueOf(expirationInMinutes), link);
        smsSenderProvider.send(
            outcome.keycloak().get().mobileNumber(),
            RESET_LINK_MESSAGE_KEY,
            attributes,
            context.getRealm(),
            user,
            session);
        logger.info("Reset link sent by SMS");
        return Optional.empty();
      }
      logger.warn("No channel to send the reset link to");
      return Optional.of(MessageReason.NO_CONTACT_CHANNEL.reason());
    } catch (Exception e) {
      logger.errorf(
          "Failed to send the reset link; not sent elsewhere: %s", e.getClass().getSimpleName());
      return Optional.of("send error: " + e.getClass().getSimpleName());
    }
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
    return "Send Reset Notification (contact channel)";
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
    return "Send the reset link on the user's one contact channel.";
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
