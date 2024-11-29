// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import static java.util.Arrays.asList;
import static sequent.keycloak.authenticator.Utils.sendConfirmation;
import static sequent.keycloak.authenticator.Utils.sendConfirmationDiffPost;

import com.fasterxml.jackson.databind.JsonMappingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.authentication.requiredactions.TermsAndConditions;
import org.keycloak.common.util.Time;
import org.keycloak.credential.CredentialModel;
import org.keycloak.events.Details;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RequiredActionProviderModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.protocol.AuthorizationEndpointBase;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.util.JsonSerialization;
import sequent.keycloak.authenticator.MessageOTPAuthenticator;
import sequent.keycloak.authenticator.Utils.MessageCourier;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

/** Lookups an user using a field */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class LookupAndUpdateUser implements Authenticator, AuthenticatorFactory {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String EMAIL_VERIFIED = "Email verified";

  public static final String PROVIDER_ID = "lookup-and-update-user";
  public static final String SEARCH_ATTRIBUTES = "search-attributes";
  public static final String UNSET_ATTRIBUTES = "unset-attributes";
  public static final String UPDATE_ATTRIBUTES = "update-attributes";
  public static final String AUTO_LOGIN = "auto-login";
  private static final String MESSAGE_COURIER_ATTRIBUTE = "messageCourierAttribute";
  private static final String TEL_USER_ATTRIBUTE = "telUserAttribute";
  public static final String AUTO_2FA = "auto-2fa";

  private String keycloakUrl = System.getenv("KEYCLOAK_URL");
  private String tenantId = System.getenv("SUPER_ADMIN_TENANT_ID");
  private String clientId = System.getenv("KEYCLOAK_CLIENT_ID");
  private String clientSecret = System.getenv("KEYCLOAK_CLIENT_SECRET");
  private String harvestUrl = System.getenv("HARVEST_DOMAIN");
  private String access_token;

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate(): start");

    authenticate();

    // Retrieve the configuration
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    // Extract the attributes to search and update from the configuration
    String searchAttributes = configMap.get(SEARCH_ATTRIBUTES);
    String unsetAttributes = configMap.get(UNSET_ATTRIBUTES);
    String updateAttributes = configMap.get(UPDATE_ATTRIBUTES);
    boolean autoLogin = Boolean.parseBoolean(configMap.get(AUTO_LOGIN));
    boolean auto2FA = Boolean.parseBoolean(configMap.get(AUTO_2FA));
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    // Parse attributes lists
    List<String> unsetAttributesList = parseAttributesList(unsetAttributes);
    List<String> updateAttributesList = parseAttributesList(updateAttributes);

    ObjectMapper om = new ObjectMapper();
    String password =
        context.getAuthenticationSession().getAuthNote(RegistrationPage.FIELD_PASSWORD);

    CredentialModel passwordModel = Utils.buildPassword(context.getSession(), password);
    CredentialModel otpCredential = MessageOTPCredentialModel.create(/* isSetup= */ true);
    List<CredentialModel> credentials = Arrays.asList(passwordModel, otpCredential);

    Map<String, Object> annotationsMap = new HashMap<>();
    annotationsMap.put(SEARCH_ATTRIBUTES, searchAttributes);
    annotationsMap.put(UPDATE_ATTRIBUTES, updateAttributes);
    annotationsMap.put("credentials", credentials);
    annotationsMap.put("sessionId", sessionId);

    UserModel user = null;
    RealmModel realm = context.getRealm();
    String realmId = realm.getId();

    // Build a new event for this authenticator
    Utils.buildEventDetails(
        context.newEvent().event(EventType.REGISTER),
        context.getAuthenticationSession(),
        user,
        context.getSession(),
        this.getClass().getSimpleName());

    // Send a verification to lookup user and generate an application with the data
    // gathered in
    // authnotes.
    JsonNode fieldsMatchNode = null;
    try {
      HttpResponse<String> verificationResponse =
          verifyApplication(
              getTenantId(context.getSession(), realmId),
              getElectionEventId(context.getSession(), realmId),
              null,
              null,
              Utils.buildApplicantData(context.getSession(), context.getAuthenticationSession()),
              om.writeValueAsString(annotationsMap),
              null);

      // Recover data from response
      JsonNode verificationResult = om.readTree(verificationResponse.body());

      // Check status
      if (verificationResponse.statusCode() != 200) {
        String response_message = verificationResult.get("message").textValue();
        context
            .getEvent()
            .detail("status_code", String.format("%d", verificationResponse.statusCode()))
            .detail("message", response_message)
            .error("Error generating approval.");
        context.attempted();
        context.failureChallenge(
            AuthenticationFlowError.INTERNAL_ERROR,
            context
                .form()
                .setError(Utils.ERROR_GENERATING_APPROVAL, sessionId)
                .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
        return;
      }

      String userId = verificationResult.get("user_id").textValue();
      String status = verificationResult.get("application_status").textValue();
      String type = verificationResult.get("application_type").textValue();
      String mismatches = verificationResult.get("mismatches").isNull() ? null : verificationResult.get("mismatches").textValue();
      fieldsMatchNode = verificationResult.get("fields_match");
      String fields_match = fieldsMatchNode.isNull() ? null : fieldsMatchNode.toString();
      log.infov(
          "Returned user with id {0}, approval status: {1}, type: {2}, missmatches: {3}, fields_matched: {4}",
          userId, status, type, mismatches, fields_match);

      // If an user was matched with automated verification use the id to recover it
      // from db.
      if (userId != null) {
        log.infov("Searching user with id: {0}, realmid: {1}", userId, realmId);
        UserProvider users = context.getSession().users();
        user = users.getUserById(realm, userId);

        // Set the details of the automatic verification
        context
            .getEvent()
            .detail("status", status)
            .detail("type", type)
            .detail("mismatches", mismatches)
            .detail("fields_matched", fields_match);
        log.infov("User after search: {0}", user);
      }

    } catch (JsonMappingException e) {
      e.printStackTrace();
      context.getEvent().error("Error processing generated approval: " + e.getMessage());
      return;
    } catch (IOException | InterruptedException e) {
      e.printStackTrace();
      context.getEvent().error("Error generating approval: " + e.getMessage());
      return;
    }

    // If no user was found show the manual verification screen
    if (user == null) {
      Response form = context.form().createForm("registration-manual-finish.ftl");
      context.challenge(form);
      return;
    }

    // If an user was found proceed with the normal flow. Set the current user.
    context.getEvent().user(user);

    // Fail the flow if the user already has credentials
    if (user.credentialManager().getStoredCredentialsStream().count() > 0) {
      log.error("authenticate(): user found but already has credentials");
      context.getEvent().error(Utils.ERROR_USER_HAS_CREDENTIALS);
      context.attempted();
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_USER_HAS_CREDENTIALS_ERROR, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
      return;
    }

    String email = user.getEmail();
    String username = user.getUsername();

    context
        .getEvent()
        .detail(Details.USERNAME, username)
        .detail(Details.REGISTER_METHOD, "form")
        .detail(Details.EMAIL, email);

    // Fail if the user does have set any of the specified attributes
    Optional<String> unsetAttributesChecked =
        checkUnsetAttributes(user, context, unsetAttributesList);

    if (unsetAttributesChecked.isPresent()) {
      log.error("authenticate(): some user unset attributes are set");
      context
          .getEvent()
          .error(Utils.ERROR_USER_ATTRIBUTES_NOT_UNSET + ": " + unsetAttributesChecked.get());
      context.attempted();
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_USER_ATTRIBUTES_NOT_UNSET_ERROR, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
      return;
    }

    // User was found and is verified to be an updateable user: we then
    // update user attributes and set it as the current auth context user
    // for other authentication models in the authentication flow
    log.info("authenticate(): updating user attributes..");
    updateUserAttributes(user, context, updateAttributesList);

    // Set email to verified if it was validated
    if (context.getAuthenticationSession().getAuthNote(EMAIL_VERIFIED) != null
        && context
            .getAuthenticationSession()
            .getAuthNote(EMAIL_VERIFIED)
            .equalsIgnoreCase("true")) {
      user.setEmailVerified(true);
    }

    log.info("authenticate(): done");

    // Success event, similar to RegistrationUserCreation.java in keycloak

    if (context.getRealm().isRegistrationEmailAsUsername()) {
      username = email;
    }

    user.setEnabled(true);

    if ("on".equals(context.getAuthenticationSession().getAuthNote("termsAccepted"))) {
      // if accepted terms and conditions checkbox, remove action and add
      // the attribute if enabled
      RequiredActionProviderModel tacModel =
          context
              .getRealm()
              .getRequiredActionProviderByAlias(
                  UserModel.RequiredAction.TERMS_AND_CONDITIONS.name());
      if (tacModel != null && tacModel.isEnabled()) {
        user.setSingleAttribute(
            TermsAndConditions.USER_ATTRIBUTE, Integer.toString(Time.currentTime()));
        context
            .getAuthenticationSession()
            .removeRequiredAction(UserModel.RequiredAction.TERMS_AND_CONDITIONS);
        user.removeRequiredAction(UserModel.RequiredAction.TERMS_AND_CONDITIONS);
      }
    }
    log.info("authenticate(): setUser");
    context.setUser(user);

    try {
      user.credentialManager().updateCredential(UserCredentialModel.password(password, false));
    } catch (Exception me) {
      user.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD);
    }

    context.getAuthenticationSession().setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, username);

    context.getEvent().user(user);
    context.getEvent().success();
    context.newEvent().event(EventType.LOGIN);
    context
        .getEvent()
        .client(context.getAuthenticationSession().getClient().getClientId())
        .detail(Details.REDIRECT_URI, context.getAuthenticationSession().getRedirectUri())
        .detail(Details.AUTH_METHOD, context.getAuthenticationSession().getProtocol());
    String authType = context.getAuthenticationSession().getAuthNote(Details.AUTH_TYPE);
    if (authType != null) {
      context.getEvent().detail(Details.AUTH_TYPE, authType);
    }

    if (auto2FA) {
      // Generate a MessageOTP credential for the user and remove the required
      // action
      MessageOTPCredentialProvider credentialProvider = getCredentialProvider(context.getSession());
      credentialProvider.createCredential(
          context.getRealm(),
          context.getUser(),
          MessageOTPCredentialModel.create(/* isSetup= */ true));
    }

    if (autoLogin) {
      context.getEvent().detail("auto_login", "true");
      context.getEvent().success();
      context.success();
    } else {
      context.clearUser();

      MessageCourier messageCourier =
          MessageCourier.fromString(config.getConfig().get(MESSAGE_COURIER_ATTRIBUTE));
      log.infov("authenticate(): messageCourier {0}", messageCourier);
      log.infov("authenticate(): user details {0}", user);

      if (!MessageCourier.NONE.equals(messageCourier)) {
        try {
          String telUserAttribute = config.getConfig().get(TEL_USER_ATTRIBUTE);
          String mobile = user.getFirstAttribute(telUserAttribute);
    
          // Get embassy value from fieldsMatchNode
          boolean embassyMatch = false;
          if (!fieldsMatchNode.isNull() && fieldsMatchNode.has("embassy")) {
            embassyMatch = fieldsMatchNode.get("embassy").asBoolean();
          }
    
          // Choose which confirmation function to use based on embassy match
          if (!embassyMatch) {
            sendConfirmationDiffPost(
                context.getSession(), context.getRealm(), user, messageCourier, mobile, context);
          } else {
            sendConfirmation(
                context.getSession(), context.getRealm(), user, messageCourier, mobile, context);
          }
        } catch (Exception error) {
          log.errorv("there was an error {0}", error);
          context.failureChallenge(
              AuthenticationFlowError.INTERNAL_ERROR,
              context
                  .form()
                  .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
                  .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
          return;
        }
      }

      // Force redirect to login even if initial flow was registration
      context
          .getAuthenticationSession()
          .setClientNote(
              AuthorizationEndpointBase.APP_INITIATED_FLOW, LoginActionsService.AUTHENTICATE_PATH);

      Response form = context.form().createForm("registration-finish.ftl");
      context.challenge(form);
    }
  }

  private Optional<String> checkUnsetAttributes(
      UserModel user, AuthenticationFlowContext context, List<String> attributes) {
    Map<String, List<String>> userAttributes = user.getAttributes();
    for (String attributeName : attributes) {
      if (attributeName.equals("email")) {
        // Only assume email is valid if it's verified
        if (user.isEmailVerified() && user.getEmail() != null && !user.getEmail().isBlank()) {
          log.info("checkUnsetAttributes(): user has email=" + user.getEmail());
          return Optional.of("User has email attribute set but it should be unset");
        }
      } else {
        if (userAttributes.containsKey(attributeName)
            && userAttributes.get(attributeName) != null
            && userAttributes.get(attributeName).size() > 0
            && userAttributes.get(attributeName).get(0) != null
            && !userAttributes.get(attributeName).get(0).isBlank()) {
          String formattedErrorMessage =
              "User has attribute "
                  + attributeName
                  + " with value="
                  + userAttributes.get(attributeName)
                  + " but it should be unset";
          log.error(formattedErrorMessage);
          return Optional.of(formattedErrorMessage);
        }
      }
    }
    return Optional.empty();
  }

  private void updateUserAttributes(
      UserModel user, AuthenticationFlowContext context, List<String> attributes) {
    for (String attribute : attributes) {
      List<String> values = Utils.getAttributeValuesFromAuthNote(context, attribute);
      if (values != null && !values.isEmpty() && !values.get(0).isBlank()) {
        if (attribute.equals("username")) {
          log.debugv("Setting attribute username to value={}", values.get(0));
          user.setUsername(values.get(0));
        } else if (attribute.equals("email")) {
          log.debugv("Setting attribute email to value={}", values.get(0));
          user.setEmail(values.get(0));
        }
        log.debugv("Setting attribute name={} to values={}", attribute, values);
        user.setAttribute(attribute, values);
      } else {
        log.debugv("No setting attribute name={} because it's blank or null", attribute);
      }
    }
  }

  private List<String> parseAttributesList(String attributes) {
    if (attributes == null || attributes.trim().isEmpty()) {
      return Collections.emptyList();
    }
    return List.of(attributes.split(","));
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action(): start");

    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();

    boolean autoLogin = Boolean.parseBoolean(configMap.get(AUTO_LOGIN));

    if (autoLogin) {
      context.success();
    }
  }

  @Override
  public boolean requiresUser() {
    // This authenticator does not necessarily require an existing user
    return false;
  }

  @Override
  public boolean configuredFor(
      KeycloakSession session, org.keycloak.models.RealmModel realm, UserModel user) {
    // Applicable for any user
    return true;
  }

  @Override
  public void setRequiredActions(
      KeycloakSession session, org.keycloak.models.RealmModel realm, UserModel user) {
    // No additional required actions
  }

  @Override
  public void close() {
    // No resources to close
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return new LookupAndUpdateUser();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public String getDisplayType() {
    return "Lookup User from Authentication Notes";
  }

  @Override
  public String getHelpText() {
    return "Looks up and optionally updates a user based on attributes stored in authentication notes.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    ProviderConfigProperty messageCourier =
        new ProviderConfigProperty(
            MESSAGE_COURIER_ATTRIBUTE,
            "Registration Success Courier",
            "Send a confirmation notification of registration success.",
            ProviderConfigProperty.LIST_TYPE,
            MessageCourier.NONE.name());
    messageCourier.setOptions(
        asList(
            MessageCourier.BOTH.name(),
            MessageCourier.SMS.name(),
            MessageCourier.EMAIL.name(),
            MessageCourier.NONE.name()));

    // Define configuration properties
    return List.of(
        new ProviderConfigProperty(
            SEARCH_ATTRIBUTES,
            "Search Attributes",
            "Comma-separated list of attributes to use for searching the user in auth notes.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UNSET_ATTRIBUTES,
            "Unset Attributes",
            "Comma-separated list of attributes that the user needs to have unset and otherwise the authenticator should fail.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UPDATE_ATTRIBUTES,
            "Update Attributes",
            "Comma-separated list of attributes to update for the user from auth notes.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            AUTO_LOGIN,
            "Login after registration",
            "If enabled the user will automatically login after registration.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            TEL_USER_ATTRIBUTE,
            "Telephone User Attribute",
            "Name of the user attribute used to retrieve the mobile telephone number of the user. Please make sure this is a read-only attribute for security reasons.",
            ProviderConfigProperty.STRING_TYPE,
            MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        new ProviderConfigProperty(
            AUTO_2FA,
            "Automatic 2FA Email/SMS",
            "If enabled will configure the users 2FA to use the Email or SMS provided during registration.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        messageCourier);
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public String getReferenceCategory() {
    return "user-lookup";
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  public MessageOTPCredentialProvider getCredentialProvider(KeycloakSession session) {
    log.info("getCredentialProvider()");
    return new MessageOTPCredentialProvider(session);
    // TODO: doesn't work - why?
    // return (MessageOTPCredentialProvider) session
    // .getProvider(
    // CredentialProvider.class,
    // MessageOTPCredentialProviderFactory.PROVIDER_ID
    // );
  }

  private HttpResponse<String> verifyApplication(
      String tenantId,
      String electionEventId,
      String areaId,
      String applicantId,
      String applicantData,
      String annotations,
      String labels)
      throws IOException, InterruptedException {
    HttpClient client = HttpClient.newHttpClient();
    String url = "http://" + this.harvestUrl + "/verify-application";
    String requestBody =
        String.format(
            "{\"tenant_id\": \"%s\", \"election_event_id\": \"%s\", \"area_id\": \"%s\", \"applicant_id\": \"%s\", \"applicant_data\" : %s, \"annotations\": %s, \"labels\": \"%s\"}",
            Utils.escapeJson(tenantId),
            Utils.escapeJson(electionEventId),
            Utils.escapeJson(areaId),
            Utils.escapeJson(applicantId),
            applicantData,
            annotations,
            Utils.escapeJson(labels));
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer " + this.access_token)
            .POST(HttpRequest.BodyPublishers.ofString(requestBody))
            .build();

    HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
    log.infov("Verification response: {0}", response);

    return response;
  }

  public void authenticate() {
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
      this.access_token = accessToken.toString();
    } catch (IOException e) {
      e.printStackTrace();
    }
  }

  private String getTenantRealmName(String realmName) {
    return "tenant-" + tenantId;
  }

  private String getElectionEventId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();
    String[] parts = realmName.split("event-");
    if (parts.length > 1) {
      return parts[1];
    }
    return null;
  }

  /**
   * Gets the tenant id from the realm name
   *
   * @param session
   * @param realmId
   * @return Tenant id found in the realm name or null if it wasn't present
   */
  public String getTenantId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();

    // Regular expression to match a UUID pattern
    Pattern uuidPattern =
        Pattern.compile(
            "\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b");
    Matcher matcher = uuidPattern.matcher(realmName);

    // Find the first match
    return matcher.find() ? matcher.group() : null;
  }
}
