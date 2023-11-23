package sequent.keycloak.authenticator.forgot_password;

import org.keycloak.models.DefaultActionTokenKey;
import org.keycloak.Config;
import org.keycloak.authentication.*;
import org.keycloak.authentication.authenticators.broker.AbstractIdpAuthenticator;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.validation.Validation;
import com.google.auto.service.AutoService;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import lombok.extern.jbosslog.JBossLog;

import java.util.List;

/**
 * Choose an user by providing both username and email. Used in the Forgot
 * Password flow, as the first step.
 */
@AutoService(AuthenticatorFactory.class)
@JBossLog
public class ChooseUser implements Authenticator, AuthenticatorFactory {

    public static final String PROVIDER_ID = "forgot-password-choose-user";
    public static final String FORM_FTL = "forgot-password-choose-user.ftl";
    public static final Requirement[] REQUIREMENT_CHOICES = {
            Requirement.REQUIRED
    };

    @Override
    public void authenticate(AuthenticationFlowContext context)
    {
        String existingUserId = context
            .getAuthenticationSession()
            .getAuthNote(AbstractIdpAuthenticator.EXISTING_USER_INFO);
        if (existingUserId != null)
        {
            UserModel existingUser = AbstractIdpAuthenticator
                .getExistingUser(
                    context.getSession(),
                    context.getRealm(),
                    context.getAuthenticationSession()
                );

            log.infov(
                "Forget-password triggered when reauthenticating user after first broker login. Prefilling reset-credential-choose-user screen with user '{0}' ", existingUser.getUsername()
            );
            context.setUser(existingUser);
            Response challenge = context.form().createPasswordReset();
            context.challenge(challenge);
            return;
        }

        String actionTokenUserId = context
            .getAuthenticationSession()
            .getAuthNote(DefaultActionTokenKey.ACTION_TOKEN_USER_ID);
        if (actionTokenUserId != null)
        {
            UserModel existingUser = context
                .getSession()
                .users()
                .getUserById(context.getRealm(), actionTokenUserId);

            // Action token logics handles checks for user ID validity and user
            // being enabled
            log.infov(
                "Forget-password triggered when reauthenticating user after authentication via action token. Skipping reset-credential-choose-user screen and using user '{0}' ",
                existingUser.getUsername()
            );
            context.setUser(existingUser);
            context.success();
            return;
        }

        context
            .challenge(
                context
                    .form()
                    .setAttribute("realm", context.getRealm())
                    .createForm(FORM_FTL)
            );
    }

    @Override
    public void action(AuthenticationFlowContext context)
    {
        EventBuilder event = context.getEvent();
        MultivaluedMap<String, String> formData = context
            .getHttpRequest()
            .getDecodedFormParameters();
        AuthenticatorConfigModel config = context.getAuthenticatorConfig();
        
        // Get username form input
        String reqUsername = formData.getFirst("username");
        if (reqUsername == null || reqUsername.trim().isEmpty())
        {
            event.error(Errors.USERNAME_MISSING);
            context.failureChallenge(
                AuthenticationFlowError.INVALID_USER,
                context
                    .form()
                    .addError(
                        new FormMessage(
                            Validation.FIELD_USERNAME,
                            Messages.MISSING_USERNAME
                        )
                    )
                    .createErrorPage(Response.Status.BAD_REQUEST)
            );
            return;
        }
        reqUsername = reqUsername.trim();

        // Get email form input
        String reqEmail = formData.getFirst("email");
        if (reqEmail == null || reqEmail.trim().isEmpty())
        {
            event.error(Errors.INVALID_USER_CREDENTIALS);
            context.failureChallenge(
                AuthenticationFlowError.INVALID_CREDENTIALS,
                context
                    .form()
                    .addError(
                        new FormMessage(
                            Validation.FIELD_EMAIL,
                            Messages.MISSING_EMAIL
                        )
                    )
                    .createErrorPage(Response.Status.BAD_REQUEST)
            );
            return;
        }
        reqEmail = reqEmail.trim();

        // Start comparing with actual backend user data
        RealmModel realm = context.getRealm();
        UserModel user = context
            .getSession()
            .users()
            .getUserByUsername(realm, reqUsername);

        // Save in auth notes the attempted username and email
        context
            .getAuthenticationSession()
            .setAuthNote(
                AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME,
                reqUsername
            );
        context
            .getAuthenticationSession()
            .setAuthNote(
                Utils.ATTEMPTED_EMAIL,
                reqEmail
            );

        // we don't want people guessing usernames or emails, so if there is a
        // problem, just continue, but don't set the user a null user will
        // notify further executions, that this was a failure.
        if (
            user == null ||
            !Utils.constantTimeIsEqual(
                Utils.getEmail(config, user).getBytes(),
                reqEmail.getBytes()
            )
        ) {
            event
                .clone()
                .detail(Details.USERNAME, reqUsername)
                .detail(Details.EMAIL, reqEmail)
                .error(Errors.USER_NOT_FOUND);
            context.clearUser();
        }
        else if (!user.isEnabled()) {
            event
                .clone()
                .detail(Details.USERNAME, reqUsername)
                .detail(Details.EMAIL, reqEmail)
                .user(user).error(Errors.USER_DISABLED);
            context.clearUser();
        } else {
            context.setUser(user);
        }

        context.success();
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
        return true;
    }

    @Override
    public void setRequiredActions(
        KeycloakSession session, 
        RealmModel realm, 
        UserModel user
    ) {

    }

    @Override
    public String getDisplayType() {
        return "Choose User by Username and email";
    }

    @Override
    public String getReferenceCategory() {
        return null;
    }

    @Override
    public boolean isConfigurable() {
        return true;
    }

    @Override
    public Requirement[] getRequirementChoices() {
        return REQUIREMENT_CHOICES;
    }

    @Override
    public boolean isUserSetupAllowed() {
        return false;
    }

    @Override
    public String getHelpText() {
        return "Choose a user to reset credentials for, using the username and email as required input";
    }

	@Override
	public List<ProviderConfigProperty> getConfigProperties()
    {
		return List.of(
			new ProviderConfigProperty(
				Utils.EMAIL_USER_ATTRIBUTE,
				"Email User Attribute",
				"Name of the user attribute used to retrieve the email address of the user. Please make sure this is a read-only attribute for security reasons.", 
				ProviderConfigProperty.STRING_TYPE,
				Utils.EMAIL_ADDRESS_FIELD
			)
		);
	}

    @Override
    public void close() {

    }

    @Override
    public Authenticator create(KeycloakSession session) {
        return this;
    }

    @Override
    public void init(Config.Scope config) {

    }

    @Override
    public void postInit(KeycloakSessionFactory factory) {

    }

    @Override
    public String getId() {
        return PROVIDER_ID;
    }
}
