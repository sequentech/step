// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;

/** Conditional Client Authenticator that checks if the email is verified. */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ConditionalEmailVerified
    implements ConditionalAuthenticator, ConditionalAuthenticatorFactory {
  public static final String PROVIDER_ID = "conditional-email-verified";
  public static final String CONF_NEGATE = "negate";

  public static final ConditionalEmailVerified SINGLETON = new ConditionalEmailVerified();

  private static final Requirement[] REQUIREMENT_CHOICES = {
    Requirement.REQUIRED, Requirement.DISABLED
  };

  @Override
  public boolean matchCondition(AuthenticationFlowContext context) {
    log.info("matchCondition()");
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    if (authConfig == null) {
      log.infov("matchCondition(): NULL found authConfig={0}", authConfig);
      return false;
    }
    log.infov("matchCondition(): alias={0}", authConfig.getAlias());
    if (authConfig.getConfig() == null) {
      log.infov("matchCondition(): NULL found authConfig.getConfig()={0}", authConfig.getConfig());
      return false;
    }

    boolean negateOutput = Boolean.parseBoolean(authConfig.getConfig().get(CONF_NEGATE));
    log.infov("matchCondition(): negateOutput={0}", negateOutput);

    UserModel user = context.getUser();
    if (user == null) {
      log.infov("matchCondition(): NULL found user={0}", user);
      return false;
    }
    boolean emailVerified = user.isEmailVerified();
    boolean result = (emailVerified != negateOutput);

    log.info("matchCondition(): emailVerified = " + emailVerified);
    log.info("matchCondition(): negateOutput = " + negateOutput);
    log.info("matchCondition(): result = " + result);

    log.infov(
        "matchCondition(): emailVerified={0}, negateOutput[{1}], result={1}",
        emailVerified, negateOutput, result);
    return result;
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action()");
    // Not used
  }

  @Override
  public boolean requiresUser() {
    log.info("requiresUser()");
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // Not used
  }

  @Override
  public void init(Scope config) {
    // no-op
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    // no-op
  }

  @Override
  public void close() {
    // no-op
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Condition - User Email Verified";
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
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONF_NEGATE,
            "Negate output",
            "Apply a NOT to the check result.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false));
  }

  @Override
  public ConditionalAuthenticator getSingleton() {
    return SINGLETON;
  }
}
