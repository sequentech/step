package sequent.keycloak.security_question_authenticator;

import lombok.extern.jbosslog.JBossLog;
import jakarta.ws.rs.core.Response;
import org.apache.commons.lang3.StringUtils;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import jakarta.ws.rs.core.MultivaluedMap;
import java.util.function.Consumer;
import java.util.Optional;
import java.security.MessageDigest;

@JBossLog
public class SecurityQuestionRequiredAction implements RequiredActionProvider {
	public static final String PROVIDER_ID = "security-question-ra";

	@Override
	public InitiatedActionSupport initiatedActionSupport() {
		return InitiatedActionSupport.SUPPORTED;
	}

	@Override
	public void evaluateTriggers(RequiredActionContext context) {
	}

	@Override
	public void requiredActionChallenge(RequiredActionContext context)
    {
        log.info("authenticate()");
        Response challenge = context
            .form()
            .createForm(Utils.SECURITY_QUESTION_FORM);
        context.challenge(challenge);
    }

	@Override
	public void processAction(RequiredActionContext context)
    {
        log.info("action()");
        boolean validated = validateAnswer(context);
        if (!validated)
        {
			// invalid
            context.challenge(
                context
                    .form()
                    .setAttribute("realm", context.getRealm())
                    .setError("secretQuestionInvalid")
                    .createErrorPage(Response.Status.BAD_REQUEST)
            );
        } else {
            // valid
            context.success();
        }
	}
 
    protected boolean validateAnswer(RequiredActionContext context)
    {
        MultivaluedMap<String, String> formData = context
            .getHttpRequest()
            .getDecodedFormParameters();
        String secretAnswer = formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD);
        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
		KeycloakSession session = context.getSession();
		UserModel user = context.getUser();
		AuthenticationSessionModel authSession = context
			.getAuthenticationSession();

        String numLastCharsString = config
            .getConfig()
            .get(Utils.NUM_LAST_CHARS);
        String userAttributeValue = config
            .getConfig()
            .get(Utils.USER_ATTRIBUTE);
        if (userAttributeValue == null || numLastCharsString == null) {
            return false;
        }
        int numLastChars =Integer.parseInt(numLastCharsString);

		// We use constant time comparison for security reasons, to avoid timing
		// attacks
		boolean isValid = MessageDigest.isEqual(
			StringUtils.right(userAttributeValue, numLastChars).getBytes(),
			StringUtils.right(secretAnswer, numLastChars).getBytes()
		);
        return isValid;
    }
 

	@Override
	public void close() {
	}
}
