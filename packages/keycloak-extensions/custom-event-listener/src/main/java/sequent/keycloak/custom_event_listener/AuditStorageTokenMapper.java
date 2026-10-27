// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.sql.SQLException;
import java.util.List;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.*;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.models.utils.PostMigrationEvent;
import org.keycloak.protocol.ProtocolMapper;
import org.keycloak.protocol.oidc.mappers.AbstractOIDCProtocolMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAccessTokenMapper;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.AccessToken;
import org.keycloak.services.ErrorResponseException;

/** Token issuance, including refresh, must stop before audit capacity is exhausted. */
@AutoService(ProtocolMapper.class)
@JBossLog
public class AuditStorageTokenMapper extends AbstractOIDCProtocolMapper
    implements OIDCAccessTokenMapper {
  public static final String PROVIDER_ID = "sequent-audit-storage-guard";
  @FunctionalInterface interface Probe { boolean available() throws SQLException; }
  private boolean enforcementRequired;
  private Probe probe = () -> false;

  public AuditStorageTokenMapper() {}
  AuditStorageTokenMapper(boolean required, Probe probe) {
    this.enforcementRequired = required;
    this.probe = probe;
  }
  static boolean required(Map<String, String> env) {
    return switch (env.getOrDefault("AUDIT_STORAGE_POLICY", "disabled")) {
      case "required" -> true;
      case "disabled" -> false;
      default -> throw new IllegalStateException("AUDIT_STORAGE_POLICY must be required or disabled");
    };
  }
  static boolean eventRealm(String name) {
    if (name == null || !name.startsWith("tenant-")) return false;
    int split = name.indexOf("-event-", 7);
    return split > 7 && split + 7 < name.length();
  }
  @Override public void postInit(KeycloakSessionFactory factory) {
    enforcementRequired = required(System.getenv());
    if (!enforcementRequired) return;
    probe = QueueDatabase.sharedFromEnvironment()::auditStorageAvailable;
    // Client models, not caller-supplied claims, determine which realm is protected.
    // Import/update hooks keep this an unconditional client mapper; no optional
    // client scope or access.token.claim setting can disable it.
    factory.register(event -> {
      if (event instanceof ClientModel.ClientCreationEvent created) {
        install(created.getCreatedClient());
      } else if (event instanceof ClientModel.ClientUpdatedEvent updated) {
        install(updated.getUpdatedClient());
      } else if (event instanceof ClientModel.ClientProtocolUpdatedEvent updated) {
        install(updated.getClient());
      } else if (event instanceof PostMigrationEvent) {
        KeycloakModelUtils.runJobInTransaction(factory, session ->
            session.realms().getRealmsStream().filter(r -> eventRealm(r.getName()))
                .forEach(r -> session.clients().getClientsStream(r).forEach(AuditStorageTokenMapper::install)));
      }
    });
  }
  static void install(ClientModel client) {
    if (!eventRealm(client.getRealm().getName())
        || (client.getProtocol() != null && !"openid-connect".equals(client.getProtocol()))) return;
    if (client.getProtocolMappersStream().anyMatch(p -> PROVIDER_ID.equals(p.getProtocolMapper()))) return;
    ProtocolMapperModel guard = new ProtocolMapperModel();
    guard.setName("Required audit storage admission");
    guard.setProtocol("openid-connect");
    guard.setProtocolMapper(PROVIDER_ID);
    guard.setConfig(Map.of());
    client.addProtocolMapper(guard);
  }
  @Override public AccessToken transformAccessToken(AccessToken token, ProtocolMapperModel mapper,
      KeycloakSession session, UserSessionModel user, ClientSessionContext client) {
    if (!enforcementRequired || !eventRealm(session.getContext().getRealm().getName())) return token;
    boolean allowed = false;
    try {
      allowed = probe.available();
    } catch (SQLException | RuntimeException failure) {
      log.warn("Voter token refused: audit-storage health could not be verified");
    }
    if (!allowed) {
      throw new ErrorResponseException("temporarily_unavailable",
          "Voting is temporarily unavailable. Please try again later.", Response.Status.SERVICE_UNAVAILABLE);
    }
    return token;
  }
  @Override public String getId() { return PROVIDER_ID; }
  @Override public String getDisplayType() { return "Required audit storage admission"; }
  @Override public String getDisplayCategory() { return TOKEN_MAPPER_CATEGORY; }
  @Override public String getHelpText() { return "Refuses voter access-token issuance and refresh when audit storage is unavailable."; }
  @Override public List<ProviderConfigProperty> getConfigProperties() { return List.of(); }
  @Override public int getPriority() { return -100; }
}
