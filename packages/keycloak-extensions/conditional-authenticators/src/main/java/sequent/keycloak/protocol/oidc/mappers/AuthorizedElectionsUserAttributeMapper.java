// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
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
import java.util.concurrent.TimeUnit;
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
  private String clientId = System.getenv("KEYCLOAK_CLIENT_ID");
  private String clientSecret = System.getenv("KEYCLOAK_CLIENT_SECRET");
  private String hasuraEndpoint = System.getenv("HASURA_ENDPOINT");
  private final HttpClient client = HttpClient.newHttpClient();
  private final ObjectMapper objectMapper = new ObjectMapper();

  private static final List<ProviderConfigProperty> configProperties =
      new ArrayList<ProviderConfigProperty>();
  private static final String ARRAY_ATTRS = "use.array.attrs";
  private static final String ARRAY_ATTRS_LABEL = "JSON Array";
  private static final String ARRAY_ATTRS_HELP_TEXT = "Use a JSON array";

  private static final String CACHE_EXPIRE_ATTRS = "cache.attrs";
  private static final String CACHE_EXPIRE_LABEL = "Election Alias cache timeout";
  private static final String CACHE_EXPIRE_HELP_TEXT = "Number of Minutes before cache invalidates";

  // User attribute name for area_id
  private static final String AREA_ID_ATTRIBUTE = "area_id";

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

    Map<String, String> electionsAliasIds;
    String tenantId = null;
    String electionEventId = null;
    
    try {
      log.infov("Realm id: {0}", userSession.getRealm().getName());
      String name = userSession.getRealm().getName();
      String[] ids = name.replaceAll("tenant\\-", "").split("\\-event\\-");
      tenantId = ids[0];
      electionEventId = ids[1];
      log.infov("Election Event id: {0}", electionEventId);
      log.infov("Tenant Id: {0}", tenantId);
      electionsAliasIds = getAllElectionsFromElectionEvent(electionEventId, tenantId);
    } catch (Exception e) {
      log.error("Error getting elections from election event", e);
      return;
    }

    List<String> authorizedElectionIds = new ArrayList<>();

    // Priority 1: If user has explicit election assignments (as aliases), use them
    if (attributeValue != null && !attributeValue.isEmpty()) {
      log.infov(
          "User has explicitly authorized elections: {0}",
          attributeValue.stream().collect(Collectors.joining("|")));
      // The attributeValue contains aliases, we'll use them as-is
      // They will be mapped to IDs later in the stream processing
      authorizedElectionIds.addAll(attributeValue);
    } else {
      // Priority 2: Check if user has area_id attribute
      Collection<String> areaIdAttribute = KeycloakModelUtils.resolveAttribute(user, AREA_ID_ATTRIBUTE, false);
      
      if (areaIdAttribute != null && !areaIdAttribute.isEmpty()) {
        String areaId = areaIdAttribute.iterator().next(); // Take first area_id if multiple exist
        log.infov("User has area_id: {0}, looking up area-based elections", areaId);
        
        try {
          Map<String, List<String>> areaElectionsMap = getElectionsByArea(electionEventId, tenantId);
          List<String> areaElections = areaElectionsMap.get(areaId);
          
          if (areaElections != null && !areaElections.isEmpty()) {
            log.infov(
                "Found elections for area {0}: {1}",
                areaId,
                areaElections.stream().collect(Collectors.joining("|")));
            // Add election aliases for the elections in this area
            for (String electionId : areaElections) {
              // Find the alias for this election ID
              String alias = electionsAliasIds.entrySet().stream()
                  .filter(entry -> entry.getValue().equals(electionId))
                  .map(Map.Entry::getKey)
                  .findFirst()
                  .orElse(electionId); // If no alias found, use the ID itself
              authorizedElectionIds.add(alias);
            }
          } else {
            log.warnv("No elections found for area_id: {0}, falling back to all elections", areaId);
            authorizedElectionIds.addAll(electionsAliasIds.keySet());
          }
        } catch (Exception e) {
          log.error("Error fetching area-based elections, falling back to all elections", e);
          authorizedElectionIds.addAll(electionsAliasIds.keySet());
        }
      } else {
        // Priority 3: No explicit elections and no area_id - authorize all elections
        log.infov(
            "No authorized elections or area_id found, authorizing all elections: {0}",
            electionsAliasIds.keySet().stream().collect(Collectors.joining("|")));
        authorizedElectionIds.addAll(electionsAliasIds.keySet());
      }
    }

    Stream<String> mappedAuthorizedElectionIds =
        authorizedElectionIds.stream()
            // The key is either the alias or the id when alias is null. The value is always the id.
            // Then when key and value are equal (Ids) is because the alias was found to be null.
            .filter(electionAlias -> (electionsAliasIds.get(electionAlias) != null))
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

  public String authenticate(String tenantId) {
    HttpClient client = HttpClient.newHttpClient();
    String url =
        this.keycloakUrl
            + "/realms/"
            + getTenantRealmName(tenantId)
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

  private String getTenantRealmName(String tenantId) {
    return "tenant-" + tenantId;
  }

  // Cache results for each electionEventId with expiration after 5 minutes
  private final Cache<String, Map<String, String>> electionsCache =
      CacheBuilder.newBuilder().expireAfterWrite(5, TimeUnit.MINUTES).build();

  // Cache for area-based elections mapping: key = "tenantId:electionEventId", value = Map<areaId, List<electionId>>
  private final Cache<String, Map<String, List<String>>> areaElectionsCache =
      CacheBuilder.newBuilder().expireAfterWrite(5, TimeUnit.MINUTES).build();

  public Map<String, String> getAllElectionsFromElectionEvent(
      String electionEventId, String tenantId) throws IOException, InterruptedException {

    // Check cache first
    Map<String, String> cachedResult = electionsCache.getIfPresent(electionEventId);
    if (cachedResult != null) {
      return cachedResult;
    }

    String token = authenticate(tenantId);

    // Construct GraphQL query using a text block (Java 15+)
    String query =
        String.format(
            """
            query GetAllElectionsFromEvent {
              sequent_backend_election(where: {election_event_id: {_eq: "%s"}, tenant_id: {_eq: "%s"}}) {
                id
                alias
                name
              }
            }
            """,
            electionEventId, tenantId);

    // Build the JSON request body; escape quotes in the query if necessary
    String requestBody =
        String.format(
            "{\"query\":\"%s\",\"variables\":null,\"operationName\":\"GetAllElectionsFromEvent\"}",
            escapeJson(query));

    log.infov("requestBody: {0}", requestBody);

    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(hasuraEndpoint))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer " + token)
            .POST(HttpRequest.BodyPublishers.ofString(requestBody))
            .build();

    HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() != 200) {
      throw new RuntimeException(
          "HTTP error: " + response.statusCode() + " Body: " + response.body());
    }

    // Parse the JSON response
    JsonNode root = objectMapper.readTree(response.body());
    JsonNode electionsNode = root.path("data").path("sequent_backend_election");
    if (electionsNode.isMissingNode() || !electionsNode.isArray()) {
      throw new RuntimeException("Unexpected JSON structure: " + response.body());
    }

    StringBuilder keyAreaLog = new StringBuilder();
    Map<String, String> electionIds = new HashMap<>();
    for (JsonNode election : electionsNode) {
      String id = election.path("id").asText();
      // Use asText(null) so that if alias is missing it returns null.
      String alias = election.hasNonNull("alias") ? election.get("alias").asText() : null;
      String key = (alias != null && !alias.isEmpty()) ? alias : id;

      keyAreaLog.append(String.format("Key: %s, Id: %s, Alias: %s\t", key, id, alias));

      if (electionIds.containsKey(key)) {
        log.infov(
            "Warning: Two elections found with the same alias: {0} id_1: {1} id_2: {2}",
            alias, electionIds.get(key), id);
      }
      log.info(keyAreaLog.toString());
      electionIds.put(key, id);
    }

    // Cache the result for future calls
    electionsCache.put(electionEventId, electionIds);
    return electionIds;
  }

  /**
   * Get elections mapped by area for a given tenant and election event.
   * Returns a Map where key is area_id and value is List of election_ids for that area.
   */
  public Map<String, List<String>> getElectionsByArea(String electionEventId, String tenantId) 
      throws IOException, InterruptedException {
    
    String cacheKey = tenantId + ":" + electionEventId;
    
    // Check cache first
    Map<String, List<String>> cachedResult = areaElectionsCache.getIfPresent(cacheKey);
    if (cachedResult != null) {
      log.debugv("Using cached area elections for key: {0}", cacheKey);
      return cachedResult;
    }

    log.infov("Fetching area elections from Hasura for tenant: {0}, election event: {1}", tenantId, electionEventId);
    
    try {
      String token = authenticate(tenantId);

      // GraphQL query to get area_contest with nested contest information
      String query = String.format(
          """
          query GetElectionsByArea {
            sequent_backend_area_contest(where: {election_event_id: {_eq: "%s"}, tenant_id: {_eq: "%s"}}) {
              area_id
              contest_id
              contest {
                election_id
              }
            }
          }
          """,
          electionEventId, tenantId);

      String requestBody = String.format(
          "{\"query\":\"%s\",\"variables\":null,\"operationName\":\"GetElectionsByArea\"}",
          escapeJson(query));

      log.debugv("Area elections query: {0}", requestBody);

      HttpRequest request =
          HttpRequest.newBuilder()
              .uri(URI.create(hasuraEndpoint))
              .header("Content-Type", "application/json")
              .header("Authorization", "Bearer " + token)
              .POST(HttpRequest.BodyPublishers.ofString(requestBody))
              .build();

      HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
      
      if (response.statusCode() != 200) {
        log.errorv("HTTP error getting area elections: {0} Body: {1}", response.statusCode(), response.body());
        // Return empty map on error
        return new HashMap<>();
      }

      // Parse the JSON response
      JsonNode root = objectMapper.readTree(response.body());
      JsonNode areaContestsNode = root.path("data").path("sequent_backend_area_contest");
      
      if (areaContestsNode.isMissingNode() || !areaContestsNode.isArray()) {
        log.warnv("No area contests found or unexpected structure: {0}", response.body());
        return new HashMap<>();
      }

      // Build the map of area_id -> List<election_id>
      Map<String, List<String>> areaElectionsMap = new HashMap<>();
      
      for (JsonNode areaContest : areaContestsNode) {
        String areaId = areaContest.path("area_id").asText();
        JsonNode contestNode = areaContest.path("contest");
        
        if (!contestNode.isMissingNode() && contestNode.hasNonNull("election_id")) {
          String electionId = contestNode.path("election_id").asText();
          
          // Add election to the area's list
          areaElectionsMap.computeIfAbsent(areaId, k -> new ArrayList<>()).add(electionId);
          
          log.debugv("Area {0} -> Election {1}", areaId, electionId);
        }
      }

      log.infov("Loaded elections for {0} areas", areaElectionsMap.size());
      
      // Cache the result
      areaElectionsCache.put(cacheKey, areaElectionsMap);
      
      return areaElectionsMap;
      
    } catch (Exception e) {
      log.error("Error fetching area elections, returning empty map", e);
      return new HashMap<>();
    }
  }

  // Utility method to escape double quotes in the JSON string
  private String escapeJson(String text) {
    return text.replace("\"", "\\\"").replace("\n", "\\n");
  }
}
