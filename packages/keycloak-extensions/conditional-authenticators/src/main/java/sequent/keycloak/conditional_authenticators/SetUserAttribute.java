package sequent.keycloak.conditional_authenticators;
import lombok.extern.jbosslog.JBossLog;

import org.keycloak.authentication.authenticators.resetcred.AbstractSetRequiredActionAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import com.google.auto.service.AutoService;
import org.keycloak.authentication.*;

import java.util.List;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class SetUserAttribute
    extends AbstractSetRequiredActionAuthenticator
{
    public static final String PROVIDER_ID = "set-user-attribute";
    public static final String CONF_USER_ATTRIBUTE = "userAttribute";
    public static final String CONF_USER_ATTRIBUTE_VALUE = "userAttributeValue";

    @Override
    public void authenticate(AuthenticationFlowContext context)
    {
        log.info("authenticate()");
        UserModel user = context.getUser();
        if (user == null) {
            log.info("authenticate(): user is null, return");
            return;
        }
        AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
        String userAttributeKey = authConfig
            .getConfig()
            .get(SetUserAttribute.CONF_USER_ATTRIBUTE);
        String userAttributeValue = authConfig
            .getConfig()
            .get(SetUserAttribute.CONF_USER_ATTRIBUTE_VALUE);

        if (
            context.getExecution().isRequired() ||
            (
                context.getExecution().isConditional() &&
                configuredFor(context)
            )
        ) {
            log.infov(
                "authenticate(): removing userAttributeKey={0}",
                userAttributeKey
            );
            user.setSingleAttribute(userAttributeKey, userAttributeValue);
        }
        context.success();
    }

    protected boolean configuredFor(AuthenticationFlowContext context) {
        return true;
    }

    @Override
    public boolean isConfigurable() {
        return true;
    }

    @Override
    public List<ProviderConfigProperty> getConfigProperties() {
        return List.of(
            new ProviderConfigProperty(
				CONF_USER_ATTRIBUTE,
				"User attribute to set",
				"User attribute to set in the user.",
                ProviderConfigProperty.STRING_TYPE,
				""
			),
            new ProviderConfigProperty(
				CONF_USER_ATTRIBUTE_VALUE,
				"User attribute value",
				"User attribute value to set in the user.",
                ProviderConfigProperty.STRING_TYPE,
				""
			)
        );
    }


    @Override
    public String getDisplayType() {
        return "User Attribute - set";
    }

    @Override
    public String getHelpText() {
        return "Set an user attribute with a specific static value.";
    }

    @Override
    public String getId() {
        return PROVIDER_ID;
    }
}
