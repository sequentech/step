package sequent.keycloak.authenticator.idp_initiated_sso;

import org.keycloak.Config;
import org.keycloak.Config.Scope;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;

import com.google.auto.service.AutoService;
import lombok.extern.jbosslog.JBossLog;

@JBossLog
@AutoService(RealmResourceProviderFactory.class)
public class SamlRedirectProviderFactory implements RealmResourceProviderFactory {

    // This is the "provider-id" used in the URL
    public static final String PROVIDER_ID = "redirect-provider";

  @Override
  public RealmResourceProvider create(KeycloakSession session) {
    log.info("create");
    return new SamlRedirectProvider(session);
  }

  @Override
  public void init(Config.Scope config) {
    log.info("init");
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    log.info("postInit");
  }

  @Override
  public void close() {
    log.info("close");
  }

  @Override
  public String getId() {
    log.info("getId");
    return PROVIDER_ID;
  }

  // No-arg constructor is implicit
    
}
