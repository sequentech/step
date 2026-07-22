// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.directgrant.AbstractDirectGrantAuthenticator;
import org.keycloak.events.Errors;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * IVR-facing Direct Grant analog of {@link MultiAttributePasswordAuthenticator}: resolves a voter
 * from one or more configured attributes (e.g. date of birth) plus a PIN submitted together in a
 * single {@code grant_type=password} request, without a username. Shares its resolution rules with
 * the browser authenticator via {@link MultiAttributeCredentialResolver}.
 *
 * <p>Unlike the browser authenticator, there is no {@code matchAttributes} config property here.
 * The {@code field}/{@code max_digits}/{@code kind}/{@code maps_to}/{@code prompt_key} properties
 * this authenticator declares are read generically by the {@code ivr-config-provider} module's
 * {@code IvrConfigResourceProvider} to describe each DTMF collection step to the IVR Lambda - see
 * that module for the full contract. Rather than duplicate that same field list in a second,
 * parallel config property (risking the two drifting out of sync), this authenticator derives its
 * own resolution inputs directly from {@code maps_to}/{@code kind}: every entry whose {@code kind}
 * is {@code identifier} is a Keycloak user attribute to match (also the Direct Grant POST parameter
 * name its value arrives under); exactly one entry must be {@code secret} - the password/PIN
 * parameter name.
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class MultiAttributePasswordDirectGrantAuthenticator
    extends AbstractDirectGrantAuthenticator {
  public static final String PROVIDER_ID = "multi-attribute-password-direct-grant";

  /**
   * Config keys read by {@code ivr-config-provider}'s {@code Constants} - kept in sync manually
   * since the two modules intentionally don't share a compile-time dependency.
   */
  public static final String CONFIG_FIELD = "field";

  public static final String CONFIG_MAX_DIGITS = "max_digits";
  public static final String CONFIG_KIND = "kind";
  public static final String CONFIG_MAPS_TO = "maps_to";
  public static final String CONFIG_PROMPT_KEY = "prompt_key";

  public static final String KIND_IDENTIFIER = "identifier";
  public static final String KIND_SECRET = "secret";

  public static final Requirement[] REQUIREMENT_CHOICES = {Requirement.REQUIRED};

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    List<String> mapsTos = Utils.getMultivalueString(authConfig, CONFIG_MAPS_TO, List.of());
    List<String> kinds = Utils.getMultivalueString(authConfig, CONFIG_KIND, List.of());

    if (mapsTos.isEmpty() || mapsTos.size() != kinds.size()) {
      log.warn(
          "authenticate(): misconfigured or mismatched maps_to/kind, cannot resolve identifying"
              + " attributes");
      fail(context);
      return;
    }

    List<String> matchAttributes = new ArrayList<>();
    String passwordField = null;
    for (int i = 0; i < mapsTos.size(); i++) {
      if (KIND_SECRET.equalsIgnoreCase(kinds.get(i))) {
        passwordField = mapsTos.get(i);
      } else {
        matchAttributes.add(mapsTos.get(i));
      }
    }
    if (passwordField == null || matchAttributes.isEmpty()) {
      log.warn(
          "authenticate(): config must declare exactly one 'secret' kind entry and at least one"
              + " 'identifier' entry");
      fail(context);
      return;
    }

    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    Map<String, String> submittedValues = new HashMap<>();
    for (String attribute : matchAttributes) {
      submittedValues.put(attribute, formData.getFirst(attribute));
    }
    String password = formData.getFirst(passwordField);

    Optional<UserModel> user =
        MultiAttributeCredentialResolver.resolveAuthenticatedUser(
            context.getSession(), context.getRealm(), matchAttributes, submittedValues, password);

    if (user.isPresent()) {
      context.setUser(user.get());
      context.success();
    } else {
      fail(context);
    }
  }

  private void fail(AuthenticationFlowContext context) {
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);
    Response challengeResponse =
        errorResponse(
            Response.Status.BAD_REQUEST.getStatusCode(),
            "invalid_grant",
            "Invalid user credentials");
    context.failure(AuthenticationFlowError.INVALID_USER, challengeResponse);
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getDisplayType() {
    return "Multi-Attribute + Password (Direct Grant)";
  }

  @Override
  public String getReferenceCategory() {
    return PasswordCredentialModel.TYPE;
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public String getHelpText() {
    return "Authenticates a caller (e.g. via IVR) by matching one or more configured user"
        + " attributes (all must match the same user) plus a password/PIN submitted together in"
        + " a single Direct Grant request. Does not require a username.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONFIG_FIELD,
            "IVR field labels",
            "One label per collected field, in order (e.g. dob, pin). Shown to the IVR Lambda"
                + " only, not used for resolution.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_MAX_DIGITS,
            "Max digits per field",
            "Maximum DTMF digits per field, same order as the other field lists (e.g. 8).",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_KIND,
            "Kind per field",
            "\"identifier\" or \"secret\" per field, same order. Exactly one field must be"
                + " \"secret\" (the password/PIN); the rest identify the voter and must jointly"
                + " match a single account.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_MAPS_TO,
            "Direct Grant parameter per field",
            "The grant_type=password POST parameter name each field's value arrives under, same"
                + " order. For \"identifier\" fields this is also the Keycloak user attribute"
                + " matched against; the \"secret\" field's value should normally be \"password\".",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null),
        new ProviderConfigProperty(
            CONFIG_PROMPT_KEY,
            "IVR prompt key per field (optional)",
            "Overrides the IVR Lambda's default prompt key per field, same order. Leave empty to"
                + " use the Lambda's well-known defaults.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null));
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
