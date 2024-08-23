package sequent.keycloak.inetum_authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.Collections;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.provider.ProviderConfigProperty;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ShowFinishAndRestartFlow implements Authenticator, AuthenticatorFactory {

  public static final String PROVIDER_ID = "show-finish-and-restart-flow";
  private static final ShowFinishAndRestartFlow SINGLETON = new ShowFinishAndRestartFlow();

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("validate: start");

    Response form = context.form().createForm("registration-finish.ftl");
    context.forceChallenge(form);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action: start");

    UserSessionModel userSession =
        context
            .getSession()
            .sessions()
            .getUserSession(
                context.getRealm(), context.getAuthenticationSession().getParentSession().getId());

    if (userSession != null) {
      context.getSession().sessions().removeUserSession(context.getRealm(), userSession);
    }

    context.cancelLogin(); // This will stop the current authentication flow and effectively log out
    // the user
  }

  @Override
  public void close() {
    // No resources to close
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return false;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // No additional required actions
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Scope config) {
    // No init code required
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    // No post init code required
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Show Finish and Cancel Login";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
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

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Displays a template and finishes the authentication flow";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return Collections.emptyList();
  }
}
