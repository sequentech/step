package sequent.keycloak.inetum_authenticator;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.HashMap;
import java.util.Map;
import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.models.KeycloakSession;
import com.google.gson.Gson;
import lombok.extern.jbosslog.JBossLog;
import sequent.keycloak.inetum_authenticator.types.AuthToken;

@JBossLog
public class CustomEventListereProvider implements EventListenerProvider {
    
    private final KeycloakSession session;
    private String keycloakUrl = System.getenv("KEYCLOAK_URL");
    private String tenantId = System.getenv("SUPER_ADMIN_TENANT_ID");
    private String clientId = System.getenv("KEYCLOAK_CLIENT_ID");
    private String clientSecret = System.getenv("KEYCLOAK_CLIENT_SECRET");
    public CustomEventListereProvider(KeycloakSession session) {
        this.session = session;
        // realmName = session.getContext().getRealm().getName();
    }
    @Override
    public void close() {
        // TODO Auto-generated method stub
    }

    @Override
    public void onEvent(Event event) {
        log.info("an event was fired ");
        authenticate();
    }

    public void authenticate() {
            HttpClient client = HttpClient.newHttpClient();
            String url = "http://keycloak:8090/realms/"+ getTenantRealmName(this.tenantId) + "/protocol/openid-connect/token";
    
            Map<Object, Object> data = new HashMap<>();
            data.put("client_id", this.clientId);
            data.put("scope", "openid");
            data.put("client_secret", this.clientSecret);
            data.put("grant_type", "client_credentials");
    
            String form = data.entrySet()
                    .stream()
                    .map(entry -> entry.getKey() + "=" + entry.getValue())
                    .reduce((entry1, entry2) -> entry1 + "&" + entry2)
                    .orElse("");
            log.info(form);
            HttpRequest request = HttpRequest.newBuilder()
                    .uri(URI.create(url))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .POST(HttpRequest.BodyPublishers.ofString(form))
                    .build();
    
            HttpResponse<String> response;
            try {
                response = client.send(request, HttpResponse.BodyHandlers.ofString());
                String responseBody = response.body();
                Gson gson = new Gson();
                AuthToken authToken = gson.fromJson(responseBody, AuthToken.class);
                log.info("access_token: " + authToken.getAccess_token());
            } catch (IOException e) {
                log.info("IOException: " + e.getMessage());
            } catch (InterruptedException e) {
                // TODO Auto-generated catch block
                log.info("IOException: " + e.getMessage());
            }
    
            // Assuming the response is in JSON format and contains the access_token
            // Extract the JWT (access_token) from the response JSON
            // You can use a JSON parsing library like Jackson or Gson to extract the token
        }

    @Override
    public void onEvent(AdminEvent event, boolean includeRepresentation) {
        // TODO Auto-generated method stub
        log.info("an admin event was fired, realmName: ");
    }

    private String getTenantRealmName(String realmName) {
        return  "tenant-" + tenantId;
    }
    
} 
