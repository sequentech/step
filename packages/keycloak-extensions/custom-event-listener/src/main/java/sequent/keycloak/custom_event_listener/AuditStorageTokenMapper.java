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
import sequent.keycloak.protocol.oidc.mappers.StaffAuthorizationMapper;

/**
 * Token issuance, including refresh, must stop before audit capacity is exhausted. In tenant realms
 * service accounts, administrators and election board members keep receiving tokens.
 */
@AutoService(ProtocolMapper.class)
@JBossLog
public class AuditStorageTokenMapper extends AbstractOIDCProtocolMapper
    implements OIDCAccessTokenMapper {
  public static final String PROVIDER_ID = "sequent-audit-storage-guard";

  @FunctionalInterface
  interface Probe {
    boolean available() throws SQLException;
  }

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
      default ->
          throw new IllegalStateException("AUDIT_STORAGE_POLICY must be required or disabled");
    };
  }

  static boolean eventRealm(String name) {
    if (name == null || !name.startsWith("tenant-")) return false;
    int split = name.indexOf("-event-", 7);
    return split > 7 && split + 7 < name.length();
  }

  /** Election event realms and tenant realms carry the guard; other realms do not. */
  static boolean protectedRealm(String name) {
    return eventRealm(name) || StaffAuthorizationMapper.staffRealm(name);
  }

  /**
   * Service accounts, administrators and election board members of a tenant realm keep receiving
   * tokens without a database read, so that they can restore audit capacity.
   */
  static boolean recoveryAccount(RealmModel realm, UserSessionModel user) {
    UserModel account = user == null ? null : user.getUser();
    if (account == null) return false;
    if (account.getServiceAccountClientLink() != null) return true;
    return StaffAuthorizationMapper.administratorOrBoard(
        name -> {
          RoleModel role = realm.getRole(name);
          return role != null && account.hasRole(role);
        });
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    enforcementRequired = required(System.getenv());
    if (!enforcementRequired) return;
    probe = QueueDatabase.sharedFromEnvironment()::auditStorageAvailable;
    // Client models, not caller-supplied claims, determine which realm is protected.
    // Import/update hooks keep this an unconditional client mapper; no optional
    // client scope or access.token.claim setting can disable it.
    factory.register(
        event -> {
          if (event instanceof ClientModel.ClientCreationEvent created) {
            install(created.getCreatedClient());
          } else if (event instanceof ClientModel.ClientUpdatedEvent updated) {
            install(updated.getUpdatedClient());
          } else if (event instanceof ClientModel.ClientProtocolUpdatedEvent updated) {
            install(updated.getClient());
          } else if (event instanceof PostMigrationEvent) {
            KeycloakModelUtils.runJobInTransaction(
                factory,
                session ->
                    session
                        .realms()
                        .getRealmsStream()
                        .filter(r -> protectedRealm(r.getName()))
                        .forEach(
                            r ->
                                session
                                    .clients()
                                    .getClientsStream(r)
                                    .forEach(AuditStorageTokenMapper::install)));
          }
        });
  }

  static void install(ClientModel client) {
    if (!protectedRealm(client.getRealm().getName())
        || (client.getProtocol() != null && !"openid-connect".equals(client.getProtocol()))) return;
    if (client.getProtocolMappersStream().anyMatch(p -> PROVIDER_ID.equals(p.getProtocolMapper())))
      return;
    ProtocolMapperModel guard = new ProtocolMapperModel();
    guard.setName("Required audit storage admission");
    guard.setProtocol("openid-connect");
    guard.setProtocolMapper(PROVIDER_ID);
    guard.setConfig(Map.of());
    client.addProtocolMapper(guard);
  }

  @Override
  public AccessToken transformAccessToken(
      AccessToken token,
      ProtocolMapperModel mapper,
      KeycloakSession session,
      UserSessionModel user,
      ClientSessionContext client) {
    if (!enforcementRequired) return token;
    RealmModel realm = session.getContext().getRealm();
    if (eventRealm(realm.getName())) {
      admit(
          "Voter token refused: audit-storage health could not be verified",
          "Voting is temporarily unavailable. Please try again later.");
    } else if (StaffAuthorizationMapper.staffRealm(realm.getName())
        && !recoveryAccount(realm, user)) {
      admit(
          "Tenant-realm token refused: audit-storage health could not be verified",
          "Sign-in is temporarily unavailable. Please try again later.");
    }
    return token;
  }

  private void admit(String unverified, String refusal) {
    boolean allowed = false;
    try {
      allowed = probe.available();
    } catch (SQLException | RuntimeException failure) {
      log.warn(unverified);
    }
    if (!allowed) {
      throw new ErrorResponseException(
          "temporarily_unavailable", refusal, Response.Status.SERVICE_UNAVAILABLE);
    }
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Required audit storage admission";
  }

  @Override
  public String getDisplayCategory() {
    return TOKEN_MAPPER_CATEGORY;
  }

  @Override
  public String getHelpText() {
    return "Refuses access-token issuance and refresh when audit storage is unavailable, except to service accounts, administrators and election board members of a tenant realm.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of();
  }

  @Override
  public int getPriority() {
    return -100;
  }
}
