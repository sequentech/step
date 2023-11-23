package sequent.keycloak.authenticator.gateway;

import org.keycloak.provider.ProviderFactory;
import com.google.auto.service.AutoService;


@AutoService(ProviderFactory.class)
public interface SmsSenderProviderFactory 
    extends ProviderFactory<SmsSenderProvider>
{
}
