package sequent.keycloak.conditional_authenticators;

import org.keycloak.TokenVerifier.Predicate;
import org.keycloak.authentication.actiontoken.AbstractActionTokenHandler;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.authentication.actiontoken.TokenUtils;
import org.keycloak.authentication.actiontoken.idpverifyemail.IdpVerifyAccountLinkActionToken;
import org.keycloak.authentication.actiontoken.resetcred.ResetCredentialsActionToken;
import org.keycloak.events.Errors;
import org.keycloak.services.messages.Messages;
import org.keycloak.events.EventType;

import jakarta.ws.rs.core.Response;

import com.google.auto.service.AutoService;

import lombok.extern.jbosslog.JBossLog;

@JBossLog
@AutoService(ActionTokenHandlerFactory.class)
public class ManualVerificationTokenHandler 
    extends AbstractActionTokenHandler<ManualVerificationToken>
{
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
            verifyEmail(tokenContext)
        );
    }

    @Override
    public Response handleToken(
        ManualVerificationToken token,
        ActionTokenContext<ManualVerificationToken> tokenContext
    ) {
        log.info("handleToken");
        return (Response)new Object();
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
