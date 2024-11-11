// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.ArrayList;
import java.util.Collection;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.ProtocolMapperModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.protocol.ProtocolMapperUtils;
import org.keycloak.protocol.oidc.mappers.AbstractOIDCProtocolMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAccessTokenMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAttributeMapperHelper;
import org.keycloak.protocol.oidc.mappers.OIDCIDTokenMapper;
import org.keycloak.protocol.oidc.mappers.TokenIntrospectionTokenMapper;
import org.keycloak.protocol.oidc.mappers.UserInfoTokenMapper;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.IDToken;
import org.keycloak.util.JsonSerialization;

/**
 * Mappings UserModel.attribute to an ID Token claim. Token claim name can be a full qualified
 * nested object name, i.e. "address.country". This will create a nested json object within the toke
 * claim.
 */
@JBossLog
public class AuthorizedElectionsUserAttributeMapper extends AbstractOIDCProtocolMapper
    implements OIDCAccessTokenMapper,
        OIDCIDTokenMapper,
        UserInfoTokenMapper,
        TokenIntrospectionTokenMapper {

  private String keycloakUrl = System.getenv("KEYCLOAK_URL");
  private String tenantId = System.getenv("SUPER_ADMIN_TENANT_ID");
  private String clientId = System.getenv("KEYCLOAK_CLIENT_ID");
  private String clientSecret = System.getenv("KEYCLOAK_CLIENT_SECRET");
  private String hasuraEndpoint = System.getenv("HASURA_ENDPOINT");

  private static final List<ProviderConfigProperty> configProperties =
      new ArrayList<ProviderConfigProperty>();
  private static final String ARRAY_ATTRS = "use.array.attrs";
  private static final String ARRAY_ATTRS_LABEL = "JSON Array";
  private static final String ARRAY_ATTRS_HELP_TEXT = "Use a JSON array";

  static {
    ProviderConfigProperty property;
    property = new ProviderConfigProperty();
    property.setName(ProtocolMapperUtils.USER_ATTRIBUTE);
    property.setLabel(ProtocolMapperUtils.USER_MODEL_ATTRIBUTE_LABEL);
    property.setHelpText(ProtocolMapperUtils.USER_MODEL_ATTRIBUTE_HELP_TEXT);
    property.setType(ProviderConfigProperty.USER_PROFILE_ATTRIBUTE_LIST_TYPE);
    configProperties.add(property);
    OIDCAttributeMapperHelper.addAttributeConfig(
        configProperties, HasuraMultivaluedUserAttributeMapper.class);

    property = new ProviderConfigProperty();
    property.setName(ProtocolMapperUtils.MULTIVALUED);
    property.setLabel(ProtocolMapperUtils.MULTIVALUED_LABEL);
    property.setHelpText(ProtocolMapperUtils.MULTIVALUED_HELP_TEXT);
    property.setType(ProviderConfigProperty.BOOLEAN_TYPE);
    configProperties.add(property);

    property = new ProviderConfigProperty();
    property.setName(ProtocolMapperUtils.AGGREGATE_ATTRS);
    property.setLabel(ProtocolMapperUtils.AGGREGATE_ATTRS_LABEL);
    property.setHelpText(ProtocolMapperUtils.AGGREGATE_ATTRS_HELP_TEXT);
    property.setType(ProviderConfigProperty.BOOLEAN_TYPE);
    configProperties.add(property);

    property = new ProviderConfigProperty();
    property.setName(ARRAY_ATTRS);
    property.setLabel(ARRAY_ATTRS_LABEL);
    property.setHelpText(ARRAY_ATTRS_HELP_TEXT);
    property.setType(ProviderConfigProperty.BOOLEAN_TYPE);
    configProperties.add(property);
  }

  public static final String PROVIDER_ID = "authorized-elections-oidc-usermodel-attribute-mapper";

  public List<ProviderConfigProperty> getConfigProperties() {
    return configProperties;
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Authorized Election User Attribute";
  }

  @Override
  public String getDisplayCategory() {
    return TOKEN_MAPPER_CATEGORY;
  }

  @Override
  public String getHelpText() {
    return "Map a custom user multivalue attribute used to identify the voters authorized elections to a token claim, compatible with hasura.";
  }

  protected void setClaim(
      IDToken token, ProtocolMapperModel mappingModel, UserSessionModel userSession) {

    UserModel user = userSession.getUser();
    String attributeName = mappingModel.getConfig().get(ProtocolMapperUtils.USER_ATTRIBUTE);
    boolean aggregateAttrs =
        Boolean.valueOf(mappingModel.getConfig().get(ProtocolMapperUtils.AGGREGATE_ATTRS));
    Collection<String> attributeValue =
        KeycloakModelUtils.resolveAttribute(user, attributeName, aggregateAttrs);

    log.infov("Realm id: {0}", userSession.getRealm().getName());
    String electionEventId = userSession.getRealm().getName().split("\\-event\\-")[1];
    log.infov("Election Event id: {0}", electionEventId);

    Map<String, String> electionsAliasIds;
    try {
      electionsAliasIds = getAllElectionsFromElectionEvent(electionEventId, authenticate());
    } catch (Exception e) {
      e.printStackTrace();
      return;
    }

    List<String> authorizedElectionIds = new ArrayList<>();

    // If voter is not authorized to any election in this election event. We
    // authorize him to all
    // elections.
    if (attributeValue.isEmpty() || attributeValue == null) {
      log.infov(
          "No authorized elections: {0}",
          electionsAliasIds.keySet().stream().collect(Collectors.joining("|")));
      authorizedElectionIds.addAll(electionsAliasIds.keySet());
    } else {
      log.infov(
          "User has authorized elections: {0}",
          attributeValue.stream().collect(Collectors.joining("|")));
      authorizedElectionIds.addAll(attributeValue);
    }

    Stream<String> mappedAuthorizedElectionIds =
        authorizedElectionIds.stream()
            // Filter out elements that are not the alias. For elections that have alias to
            // null
            // key is the election id.
            .filter(
                electionAlias ->
                    (electionsAliasIds.get(electionAlias) != null
                        && !electionsAliasIds.get(electionAlias).equals(electionAlias)))
            // Map alias to election_id
            .map(electionAlias -> electionsAliasIds.get(electionAlias));

    String useArray = mappingModel.getConfig().get(ARRAY_ATTRS);
    if (Boolean.parseBoolean(useArray)) {
      OIDCAttributeMapperHelper.mapClaim(
          token, mappingModel, mappedAuthorizedElectionIds.collect(Collectors.toList()));
    } else {
      // Format the collection as a string
      String result =
          mappedAuthorizedElectionIds
              .map(s -> "\"" + s + "\"")
              .collect(Collectors.joining(", ", "{", "}"));
      log.infov("Result: {0}", result);
      OIDCAttributeMapperHelper.mapClaim(token, mappingModel, result);
    }
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint,
      boolean multivalued) {
    return createClaimMapper(
        name,
        userAttribute,
        tokenClaimName,
        claimType,
        accessToken,
        idToken,
        introspectionEndpoint,
        multivalued,
        false);
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint,
      boolean multivalued,
      boolean aggregateAttrs) {
    ProtocolMapperModel mapper =
        OIDCAttributeMapperHelper.createClaimMapper(
            name,
            userAttribute,
            tokenClaimName,
            claimType,
            accessToken,
            idToken,
            introspectionEndpoint,
            PROVIDER_ID);

    if (multivalued) {
      mapper.getConfig().put(ProtocolMapperUtils.MULTIVALUED, "true");
    }
    if (aggregateAttrs) {
      mapper.getConfig().put(ProtocolMapperUtils.AGGREGATE_ATTRS, "true");
    }

    return mapper;
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint) {
    return createClaimMapper(
        name,
        userAttribute,
        tokenClaimName,
        claimType,
        accessToken,
        idToken,
        introspectionEndpoint,
        false,
        false);
  }

  public String authenticate() {
    HttpClient client = HttpClient.newHttpClient();
    String url =
        this.keycloakUrl
            + "/realms/"
            + getTenantRealmName(this.tenantId)
            + "/protocol/openid-connect/token";
    Map<Object, Object> data = new HashMap<>();
    data.put("client_id", this.clientId);
    data.put("scope", "openid");
    data.put("client_secret", this.clientSecret);
    data.put("grant_type", "client_credentials");

    String form =
        data.entrySet().stream()
            .map(entry -> entry.getKey() + "=" + entry.getValue())
            .reduce((entry1, entry2) -> entry1 + "&" + entry2)
            .orElse("");
    log.info(form);
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .POST(HttpRequest.BodyPublishers.ofString(form))
            .build();

    CompletableFuture<HttpResponse<String>> responseFuture;
    responseFuture = client.sendAsync(request, HttpResponse.BodyHandlers.ofString());
    String responseBody = responseFuture.join().body();
    Object accessToken;
    try {
      log.info("responseBody " + responseBody);
      accessToken = JsonSerialization.readValue(responseBody, Map.class).get("access_token");
      log.info("authenticate " + accessToken.toString());
      return accessToken.toString();
    } catch (IOException e) {
      e.printStackTrace();
    }
    return responseBody;
  }

  private String getTenantRealmName(String realmName) {
    return "tenant-" + tenantId;
  }

  private Map<String, String> getAllElectionsFromElectionEvent(String electionEventId, String token)
      throws IOException, InterruptedException {
    HttpClient client = HttpClient.newHttpClient();
    String url = this.hasuraEndpoint;
    String requestBody =
        String.format(
            "{\"query\":\"query GetAllElectionsFromEvent {\\n"
                + //
                "  sequent_backend_election(where: {election_event_id: {_eq: \\\"%s\\\"}}) {\\n"
                + //
                "    id\\n"
                + //
                "    alias\\n"
                + //
                "    name\\n"
                + //
                "  }\\n"
                + //
                "}\\n"
                + //
                "\",\"variables\":null,\"operationName\":\"GetAllElectionsFromEvent\"}",
            electionEventId);
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer " + token)
            .POST(HttpRequest.BodyPublishers.ofString(requestBody))
            .build();
    HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());

    String body = response.body();

    log.infov("Response: {0}", body);

    ObjectMapper om = new ObjectMapper();
    JsonNode elections = om.readTree(body);

    Map<String, String> electionIds = new HashMap<>();

    for (JsonNode election : elections.get("data").get("sequent_backend_election")) {
      String id = election.get("id").textValue();
      String alias = election.get("alias").textValue();

      // Make sure to populate the list with all elections even if alias is not set
      String key = alias != null ? alias : id;

      log.infov("Key: {0}", key);
      log.infov("Id: {0}", id);
      log.infov("Alias: {0}", alias);

      // Check if two elections have the same alias and warn
      String found = electionIds.get(alias);
      if (found != null) {
        log.warnv(
            "Two elections found with the same alias: {0} id_1: {1} id_2: {2}", alias, found, id);
      }

      electionIds.put(key, id);
    }

    return electionIds;
  }
}
