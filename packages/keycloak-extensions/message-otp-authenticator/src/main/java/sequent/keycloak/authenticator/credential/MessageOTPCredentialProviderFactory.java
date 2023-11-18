package sequent.keycloak.authenticator.credential;

import org.keycloak.credential.CredentialProvider;
import org.keycloak.credential.CredentialProviderFactory;
import org.keycloak.models.KeycloakSession;

public class MessageOTPCredentialProviderFactory
    implements CredentialProviderFactory<MessageOTPCredentialProvider>
{
    public final static String PROVIDER_ID = "message-otp";

    @Override
    public CredentialProvider<MessageOTPCredentialModel> create(
        KeycloakSession session
    ) {
        return new MessageOTPCredentialProvider(session);
    }

    @Override
    public String getId() {
        return PROVIDER_ID;
    }
}
