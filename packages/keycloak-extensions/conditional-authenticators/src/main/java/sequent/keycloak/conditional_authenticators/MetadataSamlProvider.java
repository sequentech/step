package sequent.keycloak.conditional_authenticators;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.ext.Provider;
import java.util.HashMap;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authorization.util.Tokens;
import org.keycloak.models.KeycloakSession;
import org.keycloak.representations.AccessToken;
import org.keycloak.services.resource.RealmResourceProvider;

// curl -v http://127.0.0.1:8090/realms/master/saml/broker/simplesamlphp/endpoint

@JBossLog
@Provider
public class MetadataSamlProvider implements RealmResourceProvider {
  protected final KeycloakSession session;
  protected final AccessToken token;

  @GET
  @Path("/saml/broker/simplesamlphp/endpoint")
  public Object getEndpoint() {
    log.info("ping");
    Map<String, String> response = new HashMap<>();
    response.put("answer", "pong");
    return Response.ok(response).build();
  }

  @POST
  @Path("/saml/broker/simplesamlphp/endpoint")
  public Object postEndpoint() {
    log.info("ping");
    Map<String, String> response = new HashMap<>();
    response.put("answer", "pong");
    return Response.ok(response).build();
  }

  @Override
  public Object getResource() {
    log.info("getResource");
    return this;
  }

  @Override
  public void close() {
    log.info("close");
  }

  public MetadataSamlProvider(KeycloakSession session) {
    log.info("ManualVerificationProvider");
    this.session = session;
    this.token = Tokens.getAccessToken(session);
  }
}
