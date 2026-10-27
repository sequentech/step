// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.protocol.oidc.mappers;

import java.util.*;
import org.keycloak.models.*;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.models.utils.PostMigrationEvent;
import org.keycloak.protocol.oidc.mappers.AbstractOIDCProtocolMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAccessTokenMapper;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.AccessToken;

/** Enforces administrator/board separation on the signed token consumed by
 * both Harvest and Hasura. A second assigned role cannot restore election-data
 * access to an administrator. Machine service accounts are separate identities. */
public final class StaffAuthorizationMapper extends AbstractOIDCProtocolMapper
    implements OIDCAccessTokenMapper {
  public static final String PROVIDER_ID = "sequent-staff-authorization";
  private static final String CLAIM = "https://hasura.io/jwt/claims";
  private static final Set<String> ADMIN_CAPABILITIES = Set.of("admin-user", "role-write", "user-write",
      "signing-rules-write", "signing-issuers-write", "self-test-settings-write", "signing-checks-write",
      "signing-certificates-register", "signing-certificates-revoke", "role-create", "user-create", "tenant-write");
  private static final Set<String> SETTINGS = Set.of(
      "admin-user", "election-event-create", "user-read", "user-create", "user-write", "user-delete",
      "user-permission-read", "role-read", "role-write", "role-create", "role-delete",
      "tenant-read", "tenant-write", "tenant-create", "tenant-delete", "settings-read", "settings-write",
      "trustee-read", "trustee-create", "trustee-write", "trustee-delete", "ca-read", "ca-write",
      "certificate-authority-read", "certificate-authority-write",
      "self-test-read", "self-test-run", "self-test-settings-write", "election-event-signatures-tab",
      "signing-rules-read", "signing-rules-write", "signing-checks-read", "signing-checks-write",
      "signing-certificates-read", "signing-certificates-register", "signing-certificates-revoke",
      "signing-issuers-read", "signing-issuers-write", "template-read", "template-write", "template-create",
      "communication-template-read", "communication-template-write", "communication-template-create",
      "messaging-config-write");

  static Set<String> separated(Collection<String> roles) {
    Set<String> result = new HashSet<>(roles);
    if (roles.stream().noneMatch(ADMIN_CAPABILITIES::contains)) return result;
    result.retainAll(SETTINGS);
    return result;
  }
  static boolean staffRealm(String name) {
    return name != null && name.startsWith("tenant-") && !name.contains("-event-");
  }
  static void install(ClientModel client) {
    if (!staffRealm(client.getRealm().getName()) ||
        (client.getProtocol()!=null && !"openid-connect".equals(client.getProtocol()))) return;
    if (client.getProtocolMappersStream().anyMatch(p -> PROVIDER_ID.equals(p.getProtocolMapper()))) return;
    ProtocolMapperModel mapper = new ProtocolMapperModel();
    mapper.setName("Required administrator and board separation");
    mapper.setProtocol("openid-connect"); mapper.setProtocolMapper(PROVIDER_ID);
    mapper.setConfig(Map.of("access.token.claim","true"));
    client.addProtocolMapper(mapper);
  }
  @Override public void postInit(KeycloakSessionFactory factory) {
    factory.register(event -> {
      if (event instanceof ClientModel.ClientCreationEvent created) install(created.getCreatedClient());
      else if (event instanceof ClientModel.ClientUpdatedEvent updated) install(updated.getUpdatedClient());
      else if (event instanceof ClientModel.ClientProtocolUpdatedEvent updated) install(updated.getClient());
      else if (event instanceof PostMigrationEvent) KeycloakModelUtils.runJobInTransaction(factory, session ->
          session.realms().getRealmsStream().filter(r -> staffRealm(r.getName())).forEach(r ->
              session.clients().getClientsStream(r).forEach(StaffAuthorizationMapper::install)));
    });
  }
  @Override public AccessToken transformAccessToken(AccessToken token, ProtocolMapperModel model,
      KeycloakSession session, UserSessionModel user, ClientSessionContext context) {
    if (user.getUser().getServiceAccountClientLink()!=null || !staffRealm(session.getContext().getRealm().getName())) return token;
    Object raw = token.getOtherClaims().get(CLAIM);
    if (!(raw instanceof Map<?,?> claims)) return token;
    Object allowed = claims.get("x-hasura-allowed-roles");
    if (!(allowed instanceof Collection<?> values)) return token;
    Set<String> roles=new HashSet<>();
    values.forEach(value -> roles.add(String.valueOf(value)));
    if (token.getRealmAccess()!=null && token.getRealmAccess().getRoles()!=null) roles.addAll(token.getRealmAccess().getRoles());
    Set<String> separated=separated(roles);
    Map<String,Object> replacement=new HashMap<>();
    claims.forEach((key,value)->replacement.put(String.valueOf(key),value));
    replacement.put("x-hasura-allowed-roles",new ArrayList<>(separated));
    if (roles.stream().anyMatch(ADMIN_CAPABILITIES::contains)) replacement.put("x-hasura-default-role", separated.contains("admin-user") ? "admin-user" : separated.stream().sorted().findFirst().orElse("no-access"));
    token.getOtherClaims().put(CLAIM,replacement);
    if (token.getRealmAccess()!=null) token.getRealmAccess().roles(separated);
    return token;
  }
  @Override public String getId(){return PROVIDER_ID;}
  @Override public String getDisplayType(){return "Administrator and board separation";}
  @Override public String getDisplayCategory(){return TOKEN_MAPPER_CATEGORY;}
  @Override public String getHelpText(){return "Limits administrator tokens to account and security settings interfaces.";}
  @Override public List<ProviderConfigProperty> getConfigProperties(){return List.of();}
  @Override public int getPriority(){return 10000;}
}
