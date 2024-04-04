package sequent.keycloak.inetum_authenticator;

import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;

import com.google.auto.service.AutoService;

import lombok.extern.jbosslog.JBossLog;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Lookups an user using a field 
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class LookupAndUpdateUser implements Authenticator, AuthenticatorFactory {

    public static final String PROVIDER_ID = "lookup-and-update-user";
    public static final String SEARCH_ATTRIBUTES = "search-attributes";
    public static final String UNSET_ATTRIBUTES = "unset-attributes";
    public static final String UPDATE_ATTRIBUTES = "update-attributes";

    @Override
    public void authenticate(AuthenticationFlowContext context) {
        log.info("authenticate(): start");
        // Retrieve the configuration
        AuthenticatorConfigModel config = context.getAuthenticatorConfig();
        Map<String, String> configMap = config.getConfig();

        // Extract the attributes to search and update from the configuration
        String searchAttributes = configMap.get(SEARCH_ATTRIBUTES);
        String unsetAttributes = configMap.get(UNSET_ATTRIBUTES);
        String updateAttributes = configMap.get(UPDATE_ATTRIBUTES);

        // Parse attributes lists
        List<String> searchAttributesList = parseAttributesList(searchAttributes);
        List<String> unsetAttributesList = parseAttributesList(unsetAttributes);
        List<String> updateAttributesList = parseAttributesList(updateAttributes);

        // Lookup user by attributes in authNotes
        UserModel user = lookupUserByAuthNotes(context, searchAttributesList);

        // check user was found
        if (user == null) {
            log.error("authenticate(): user not found");
            context.attempted();
            return;
        }
        // check user has no credentials yet
        else if (user.credentialManager().getStoredCredentialsStream().count() > 0) {
            log.error("authenticate(): user found but already has credentials");
            context.attempted();
            return;
        }

        // check that the user doesn't have set any of the unset attributes
        boolean unsetAttributesChecked =
            checkUnsetAttributes(user, context, unsetAttributesList);

        if (!unsetAttributesChecked) {
            log.error("authenticate(): some user unset attributes are set");
            context.attempted();
            return;
        }

        // User was found and is verified to be an updateable user: we then
        // update user attributes and set it as the current auth context user
        // for other authentication models in the authentication flow
        updateUserAttributes(user, context, updateAttributesList);
        context.setUser(user);
        log.info("authenticate(): success");
        context.success();
    }

    private UserModel lookupUserByAuthNotes(
        AuthenticationFlowContext context, List<String> attributes
    ) {
        log.info("lookupUserByAuthNotes(): start");
        KeycloakSession session = context.getSession();
        RealmModel realm = context.getRealm();
        Stream<UserModel> userStream = null;

        for (String attribute : attributes) {
            String value = context
                .getAuthenticationSession()
                .getAuthNote(attribute);
            if (value != null) {
                Stream<UserModel> currentStream = session
                    .users()
                    .searchForUserByUserAttributeStream(realm, attribute, value);

                if (userStream == null) {
                    userStream = currentStream;
                } else {
                    // Intersect the current stream with the accumulated stream
                    // to match users on all attributes
                    Set<String> userIds = userStream
                        .map(UserModel::getId)
                        .collect(Collectors.toSet());
                    userStream = currentStream
                        .filter(user -> userIds.contains(user.getId()));
                }
            }
        }

        if (userStream != null) {
            // Return the first user that matches all attributes, if any
            Optional<UserModel> userOptional = userStream.findFirst();
            return userOptional.orElse(null);
        }

        return null;
    }

    private boolean checkUnsetAttributes(
        UserModel user,
        AuthenticationFlowContext context,
        List<String> attributes
    ) {
        Map<String, List<String>> userAttributes = user.getAttributes();
        for (String attributeName : attributes) {
            if (userAttributes.containsKey(attributeName)) {
                log.info("checkUnsetAttributes(): user has attribute " + attributeName);
                return false;
            }
        }
        return true;
    }


    private void updateUserAttributes(
        UserModel user,
        AuthenticationFlowContext context,
        List<String> attributes
    ) {
        for (String attribute : attributes) {
            String value = context
                .getAuthenticationSession()
                .getAuthNote(attribute);
            if (value != null) {
                if (attribute.equals("username")) {
                    user.setUsername(value);
                } else if(attribute.equals("email")) {
                    user.setEmail(value);
                }
                user.setSingleAttribute(attribute, value);
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
        // No action required
    }

    @Override
    public boolean requiresUser() {
        // This authenticator does not necessarily require an existing user
        return false;
    }

    @Override
    public boolean configuredFor(
        KeycloakSession session,
        org.keycloak.models.RealmModel realm,
        UserModel user
    ) {
        // Applicable for any user
        return true;
    }

    @Override
    public void setRequiredActions(
        KeycloakSession session,
        org.keycloak.models.RealmModel realm,
        UserModel user
    ) {
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
    public void init(Config.Scope config) {

    }

    @Override
    public void postInit(KeycloakSessionFactory factory) {

    }

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
        // Define configuration properties
        return List.of(
            new ProviderConfigProperty(
                SEARCH_ATTRIBUTES,
                "Search Attributes",
                "Comma-separated list of attributes to use for searching the user in auth notes.", 
                ProviderConfigProperty.STRING_TYPE,
                ""
            ),
            new ProviderConfigProperty(
                UNSET_ATTRIBUTES,
                "Unset Attributes",
                "Comma-separated list of attributes that the user needs to have unset and otherwise the authenticator should fail.",
                ProviderConfigProperty.STRING_TYPE,
                ""
            ),
            new ProviderConfigProperty(
                UPDATE_ATTRIBUTES,
                "Update Attributes",
                "Comma-separated list of attributes to update for the user from auth notes.",
                ProviderConfigProperty.STRING_TYPE,
                ""
            )
        );
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
}
