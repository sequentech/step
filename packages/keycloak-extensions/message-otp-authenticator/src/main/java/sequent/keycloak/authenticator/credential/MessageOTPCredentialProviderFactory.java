package sequent.keycloak.authenticator.credential;

import org.jboss.logging.Logger;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.credential.CredentialProviderFactory;
import org.keycloak.models.KeycloakSession;

public class MessageOTPCredentialProviderFactory
    implements CredentialProviderFactory<MessageOTPCredentialProvider>
{
	private static final Logger logger = Logger
		.getLogger(MessageOTPCredentialProviderFactory.class);

    public final static String PROVIDER_ID = "message-otp-credential";

    @Override
    public CredentialProvider<MessageOTPCredentialModel> create(
        KeycloakSession session
    ) {
        logger.info("create()");
        return new MessageOTPCredentialProvider(session);
    }

    @Override
    public String getId() {
        return PROVIDER_ID;
    }
}
