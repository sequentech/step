package sequent.keycloak.authenticator.gateway;

import org.keycloak.provider.Provider;

public interface SmsSenderProvider extends Provider
{
    public void send(String phoneNumber, String message);
}
