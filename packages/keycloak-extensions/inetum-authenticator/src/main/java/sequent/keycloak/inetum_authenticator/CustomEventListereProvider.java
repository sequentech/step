package sequent.keycloak.inetum_authenticator;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.models.KeycloakSession;
import org.keycloak.util.JsonSerialization;

import com.fasterxml.jackson.databind.util.JSONPObject;
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
    private String harvestUrl = System.getenv("HARVEST_DOMAIN");
    private String access_token;
    public CustomEventListereProvider(KeycloakSession session) {
        this.session = session;
        authenticate();

    }
    @Override
    public void close() {}

    @Override
    public void onEvent(Event event) {
        if (this.access_token == null) {
            authenticate();
        }
        logEvent();
    }

    public void authenticate() {
            HttpClient client = HttpClient.newHttpClient();
            String url = this.keycloakUrl + "/realms/"+ getTenantRealmName(this.tenantId) + "/protocol/openid-connect/token";
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
    
            CompletableFuture<HttpResponse<String>> responseFuture;
                responseFuture = client.sendAsync(request, HttpResponse.BodyHandlers.ofString());
                responseFuture.thenAccept(response -> {
                    String responseBody = response.body();
                    Object access_token;
                    try {
                        access_token = JsonSerialization.readValue(responseBody, Map.class).get("access_token");
                        this.access_token = access_token.toString();
                    } catch (IOException e) {
                        e.printStackTrace();
                    }
                });
        }

    @Override
    public void onEvent(AdminEvent event, boolean includeRepresentation) {
        log.info("an admin event was fired, realmName: ");
    }

    private String getTenantRealmName(String realmName) {
        return  "tenant-" + tenantId;
    }

    private void logEvent() {
        HttpClient client = HttpClient.newHttpClient();
        String url = this.harvestUrl + "/immudb/log-event";

        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(url))
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer " + this.access_token)
                .POST(HttpRequest.BodyPublishers.ofString("{\"event\": \"test\"}"))
                .build();
        CompletableFuture<HttpResponse<String>> response = client.sendAsync(request, HttpResponse.BodyHandlers.ofString());
        response.thenAccept(res -> {    
            log.info("success");
        }).exceptionally(e -> {
            log.error("error");
            return null;
        });
    }
     
} 
