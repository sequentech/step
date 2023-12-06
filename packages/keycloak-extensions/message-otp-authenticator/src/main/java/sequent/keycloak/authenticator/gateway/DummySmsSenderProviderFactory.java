package sequent.keycloak.authenticator.gateway;

import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import com.google.auto.service.AutoService;


@AutoService(SmsSenderProviderFactory.class)
public class DummySmsSenderProviderFactory implements SmsSenderProviderFactory {

    @Override
    public SmsSenderProvider create(KeycloakSession session) {
        return new DummySmsSenderProvider();
    }

    @Override
    public void init(Config.Scope config) {
    }

    @Override
    public void postInit(KeycloakSessionFactory factory) {
    }

    @Override
    public void close() {
    }

    @Override
    public String getId() {
        return "dummy";
    }

}
