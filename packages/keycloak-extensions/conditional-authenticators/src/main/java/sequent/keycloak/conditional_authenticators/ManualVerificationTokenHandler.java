// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import java.util.List;

import org.keycloak.TokenVerifier.Predicate;
import org.keycloak.authentication.actiontoken.AbstractActionTokenHandler;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.authentication.actiontoken.TokenUtils;
import org.keycloak.events.EventBuilder;
import org.keycloak.services.managers.AuthenticationManager;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.events.Errors;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;

import com.google.auto.service.AutoService;

import lombok.extern.jbosslog.JBossLog;

@JBossLog
@AutoService(ActionTokenHandlerFactory.class)
public class ManualVerificationTokenHandler 
    extends AbstractActionTokenHandler<ManualVerificationToken>
{
    public static final String VERIFIED_ATTRIBUTE = "sequent.read-only.id-card-number-validated";
    public static final String VERIFIED_VALUE = "VERIFIED";
    public ManualVerificationTokenHandler() {
        super(
          /* id = */ ManualVerificationToken.TOKEN_TYPE,
          /* tokenClass = */ ManualVerificationToken.class,
          /* defaultErrorMessage = */  Messages.INVALID_CODE,
          /* defaultEventType = */ EventType.RESET_PASSWORD,
          /* defaultEventError = */ Errors.NOT_ALLOWED
        );

        log.info("ManualVerificationTokenHandler");
    }

    // getVerifiers() needs not to be empty, so we verify email (which should
    // checkout always, even if it's null)
    @Override
    public Predicate<? super ManualVerificationToken>[] getVerifiers(
        ActionTokenContext<ManualVerificationToken> tokenContext
    ) {
        return TokenUtils.predicates(
            //TokenUtils.checkThat(
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
            //),
            verifyEmail(tokenContext)
        );
    }

    @Override
    public Response handleToken(
        ManualVerificationToken token,
        ActionTokenContext<ManualVerificationToken> tokenContext
    ) {
        log.info("handleToken(): start");
        AuthenticationSessionModel authSession = tokenContext
            .getAuthenticationSession();
        UserModel user = authSession.getAuthenticatedUser();
        log.info("handleToken(): user = " + user.getUsername());

        KeycloakSession session = tokenContext.getSession();

        //String redirectUri = RedirectUtils.verifyRedirectUri(
        //    tokenContext.getSession(),
        //    token.getRedirectUri(),
        //    authSession.getClient()
        //);
        String redirectUri = token.getRedirectUri();

        if (redirectUri != null) {
            log.info("handleToken(): setting redirectUri=" + redirectUri);
            authSession.setAuthNote(
                AuthenticationManager.SET_REDIRECT_URI_AFTER_REQUIRED_ACTIONS,
                "true"
            );

            authSession.setRedirectUri(redirectUri);
            authSession.setClientNote(
                OIDCLoginProtocol.REDIRECT_URI_PARAM,
                redirectUri
            );
        } else {
            log.info("handleToken(): NOT setting redirectUri, it's null");
        }

        user.setEmailVerified(true);
        user.setAttribute(VERIFIED_ATTRIBUTE, List.of(VERIFIED_VALUE));
        log.info("handleToken(): user.VERIFIED_ATTRIBUTE = " + user.getFirstAttribute(VERIFIED_ATTRIBUTE));

        authSession.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD.name());
        user.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD.name());
        tokenContext.getEvent().success();
        tokenContext
            .setEvent(tokenContext.getEvent().clone().event(EventType.LOGIN));

        String nextAction = AuthenticationManager.nextRequiredAction(
            session,
            authSession,
            tokenContext.getRequest(),
            tokenContext.getEvent()
        );
        return AuthenticationManager.redirectToRequiredActions(
            session,
            authSession.getRealm(),
            authSession,
            tokenContext.getUriInfo(),
            nextAction
        );
    }

    // to execute again, you will need a new token
    @Override
    public boolean canUseTokenRepeatedly(
        ManualVerificationToken token,
        ActionTokenContext<ManualVerificationToken> tokenContext
    ) {
        return false;
    }
}
