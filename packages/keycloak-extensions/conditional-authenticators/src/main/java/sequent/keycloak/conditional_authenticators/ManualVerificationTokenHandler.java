// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static sequent.keycloak.authenticator.Utils.sendConfirmation;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.List;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.TokenVerifier.Predicate;
import org.keycloak.authentication.actiontoken.AbstractActionTokenHandler;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.authentication.actiontoken.TokenUtils;
import org.keycloak.events.Errors;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.services.managers.AuthenticationManager;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.Utils.MessageCourier;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

@JBossLog
@AutoService(ActionTokenHandlerFactory.class)
public class ManualVerificationTokenHandler
    extends AbstractActionTokenHandler<ManualVerificationToken> {

  // TODO: Make it configurable
  public static final String VERIFIED_ATTRIBUTE = "sequent.read-only.id-card-number-validated";
  public static final String TEL_USER_ATTRIBUTE = "sequent.read-only.mobile-number";
  public static final String VERIFIED_VALUE = "VERIFIED";

  public ManualVerificationTokenHandler() {
    super(
        /* id= */ ManualVerificationToken.TOKEN_TYPE,
        /* tokenClass= */ ManualVerificationToken.class,
        /* defaultErrorMessage= */ Messages.INVALID_CODE,
        /* defaultEventType= */ EventType.RESET_PASSWORD,
        /* defaultEventError= */ Errors.NOT_ALLOWED);

    log.info("ManualVerificationTokenHandler");
  }

  // getVerifiers() needs not to be empty, so we verify email (which should
  // checkout always, even if it's null)
  @Override
  public Predicate<? super ManualVerificationToken>[] getVerifiers(
      ActionTokenContext<ManualVerificationToken> tokenContext) {
    return TokenUtils.predicates(
        // TokenUtils.checkThat(
        //    // either redirect URI is not specified or must be valid for the client
        //    t ->
        //        t.getRedirectUri() == null ||
        //            RedirectUtils.verifyRedirectUri(
        //            tokenContext.getSession(),
        //            t.getRedirectUri(),
        //            tokenContext.getAuthenticationSession().getClient()
        //        ) != null,
        //    Errors.INVALID_REDIRECT_URI,
        //    Messages.INVALID_REDIRECT_URI
        // ),
        verifyEmail(tokenContext));
  }

  @Override
  public Response handleToken(
      ManualVerificationToken token, ActionTokenContext<ManualVerificationToken> tokenContext) {
    log.info("handleToken(): start");
    AuthenticationSessionModel authSession = tokenContext.getAuthenticationSession();
    UserModel user = authSession.getAuthenticatedUser();
    log.info("handleToken(): user = " + user.getUsername());

    KeycloakSession session = tokenContext.getSession();

    // String redirectUri = RedirectUtils.verifyRedirectUri(
    //    tokenContext.getSession(),
    //    token.getRedirectUri(),
    //    authSession.getClient()
    // );
    String redirectUri = token.getRedirectUri();

    if (redirectUri != null) {
      log.infov("handleToken(): setting redirectUri={0}", redirectUri);
      authSession.setAuthNote(
          AuthenticationManager.SET_REDIRECT_URI_AFTER_REQUIRED_ACTIONS, "true");

      authSession.setRedirectUri(redirectUri);
      authSession.setClientNote(OIDCLoginProtocol.REDIRECT_URI_PARAM, redirectUri);
    } else {
      log.info("handleToken(): NOT setting redirectUri, it's null");
    }

    user.setEmailVerified(true);
    user.setAttribute(VERIFIED_ATTRIBUTE, List.of(VERIFIED_VALUE));
    log.infov(
        "handleToken(): user.VERIFIED_ATTRIBUTE = {0}", user.getFirstAttribute(VERIFIED_ATTRIBUTE));

    authSession.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD.name());
    user.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD.name());

    Optional<AuthenticatorConfigModel> config =
        Utils.getConfig(tokenContext.getRealm(), ManualVerificationConfigAuthenticator.PROVIDER_ID);

    String telUserAttribute =
        config
            .map(c -> c.getConfig().get(ManualVerificationConfigAuthenticator.TEL_USER_ATTRIBUTE))
            .orElse(TEL_USER_ATTRIBUTE);

    log.infov("handleToken(): telUserAttribute configuration {0}", telUserAttribute);
    String mobile = user.getFirstAttribute(telUserAttribute);
    log.infov("handleToken(): user mobile {0}", mobile);

    var messageCourier =
        config
            .map(
                c ->
                    MessageCourier.fromString(
                        c.getConfig()
                            .get(ManualVerificationConfigAuthenticator.MESSAGE_COURIER_ATTRIBUTE)))
            .orElse(MessageCourier.BOTH);
    log.infov("handleToken(): messageCourier configuration {0}", messageCourier);

    try {
      sendConfirmation(
          tokenContext.getSession(),
          tokenContext.getRealm(),
          user,
          messageCourier,
          mobile,
          tokenContext);
    } catch (Exception error) {
      error.printStackTrace();
    }

    boolean auto2FA =
        config
            .map(
                c ->
                    Boolean.parseBoolean(
                        c.getConfig().get(ManualVerificationConfigAuthenticator.AUTO_2FA)))
            .orElse(false);
    log.infov("handleToken(): auto2FA configuration {0}", auto2FA);

    if (auto2FA) {
      // Generate a MessageOTP credential for the user and remove the required
      // action
      MessageOTPCredentialProvider credentialProvider =
          getCredentialProvider(tokenContext.getSession());
      credentialProvider.createCredential(
          tokenContext.getRealm(), user, MessageOTPCredentialModel.create(/* isSetup= */ true));
    }

    tokenContext.getEvent().success();
    tokenContext.setEvent(tokenContext.getEvent().clone().event(EventType.LOGIN));

    String nextAction =
        AuthenticationManager.nextRequiredAction(
            session, authSession, tokenContext.getRequest(), tokenContext.getEvent());
    return AuthenticationManager.redirectToRequiredActions(
        session, authSession.getRealm(), authSession, tokenContext.getUriInfo(), nextAction);
  }

  // to execute again, you will need a new token
  @Override
  public boolean canUseTokenRepeatedly(
      ManualVerificationToken token, ActionTokenContext<ManualVerificationToken> tokenContext) {
    return false;
  }

  public MessageOTPCredentialProvider getCredentialProvider(KeycloakSession session) {
    log.info("getCredentialProvider()");
    return new MessageOTPCredentialProvider(session);
    // TODO: doesn't work - why?
    // return (MessageOTPCredentialProvider) session
    // 	.getProvider(
    // 		CredentialProvider.class,
    // 		MessageOTPCredentialProviderFactory.PROVIDER_ID
    // 	);
  }
}
