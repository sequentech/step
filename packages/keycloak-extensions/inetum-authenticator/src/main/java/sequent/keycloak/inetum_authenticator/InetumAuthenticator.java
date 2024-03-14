package sequent.keycloak.inetum_authenticator;

import sequent.keycloak.inetum_authenticator.Utils;
import org.apache.commons.lang3.StringUtils;
import org.keycloak.http.HttpCookie;
import org.keycloak.http.HttpResponse;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import jakarta.ws.rs.core.Cookie;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.Collections;
import java.util.List;
import java.security.MessageDigest;

@JBossLog
public class InetumAuthenticator implements Authenticator
{
    @Override
    public void authenticate(AuthenticationFlowContext context)
    {
        // Authentication is successful if the user already has the user's 
        // validation status attribute set to true, otherwise initiate a new 
        // flow and show form
        log.info("authenticate()");
        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        UserModel user = context.getUser();

        if (user != null)
        {
            String statusAttributeName = config
                .getConfig()
                .get(Utils.USER_STATUS_ATTRIBUTE);
            String statusAttributeValue = user.getFirstAttribute(statusAttributeName);
            log.info("checking statusAttributeValue=" + statusAttributeValue);
            boolean validated = (
                statusAttributeValue != null && statusAttributeValue.equals("TRUE")
            );

            log.info("validated=" + validated);
            if (validated)
            {
                log.info("validated IS TRUE, pass");
                context.success();
                return;
            }
        }

        log.info("validated is NOT TRUE, rendering the form");
        Response challenge = context
            .form()
            .createForm(Utils.INETUM_FORM);
        context.challenge(challenge);
    }
 
    @Override
    public void action(AuthenticationFlowContext context)
    {
        log.info("action()");
        boolean validated = validateAnswer(context);
        if (!validated)
        {
			// invalid
			AuthenticationExecutionModel execution = context.getExecution();
			if (execution.isRequired())
            {
				context.failureChallenge(
					AuthenticationFlowError.INVALID_CREDENTIALS,
					context
						.form()
						.setAttribute("realm", context.getRealm())
						.setError("authInvalid")
						.createForm(Utils.INETUM_FORM)
				);
			} else if (execution.isConditional() || execution.isAlternative())
            {
				context.attempted();
			}
        } else {
            // valid
            context.success();
        }
    }
 
    protected boolean validateAnswer(AuthenticationFlowContext context)
    {
        return true;
    }
 
    @Override
    public boolean requiresUser() {
        return false;
    }
 
     @Override
     public boolean configuredFor(
        KeycloakSession session,
        RealmModel realm,
        UserModel user
    ) {
        return false;
    }
 
     @Override
     public void setRequiredActions(
        KeycloakSession session,
        RealmModel realm,
        UserModel user
    ) {
    }

    @Override
    public void close() { }
}
 