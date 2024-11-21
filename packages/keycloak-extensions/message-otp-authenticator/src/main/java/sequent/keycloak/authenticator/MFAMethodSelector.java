// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.Map;
import java.util.stream.Collectors;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.authentication.requiredactions.WebAuthnPasswordlessRegisterFactory;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.OTPCredentialModel;
import org.keycloak.models.credential.WebAuthnCredentialModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;

/**
 * Required Action that requires users to choose at least one 2nd auth factor method, thus
 * implementing MFA enforment.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class MFAMethodSelector implements RequiredActionFactory, RequiredActionProvider {
  public static final String PROVIDER_ID = "mfa-method-selector";
  private static final String TPL_SELECTOR = "selector-2fa.ftl";

  // map of key-value pairs:
  // - key = credential type
  // - value = associated required action id
  // {
  //  	"otp": "CONFIGURE_TOTP",
  // 		"message-otp": "message-otp-ra"
  // }
  private static final Map<String, String> credentialTypes =
      Map.of(
          // Normal OTP
          OTPCredentialModel.TYPE,
          UserModel.RequiredAction.CONFIGURE_TOTP.name(),

          // Message OTP
          MessageOTPCredentialModel.TYPE,
          ResetMessageOTPRequiredAction.PROVIDER_ID,

          // Passkeys
          WebAuthnCredentialModel.TYPE_PASSWORDLESS,
          WebAuthnPasswordlessRegisterFactory.PROVIDER_ID);

  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {
    log.info("evaluateTriggers()");
    // self registering if user doesn't have already one out of the
    // configured credential types
    UserModel user = context.getUser();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();

    if (authSession.getRequiredActions().stream().noneMatch(PROVIDER_ID::equals)
        && credentialTypes.keySet().stream()
            .noneMatch(
                type -> {
                  boolean ret =
                      user.credentialManager()
                          .getStoredCredentialsByTypeStream(type)
                          .findAny()
                          .isPresent();
                  // TODO: The following doesn't work, for some unknown
                  // reason
                  // boolean ret = user
                  // 	.credentialManager()
                  // 	.isConfiguredFor(type);
                  log.info(
                      "evaluateTriggers(): credentiaTypes: type=" + type + ", userHasAny=" + ret);
                  return ret;
                })
        && user.getRequiredActionsStream().noneMatch(credentialTypes::containsValue)
        && authSession.getRequiredActions().stream().noneMatch(credentialTypes::containsValue)) {
      log.info("evaluateTriggers(): adding required action");
      authSession.addRequiredAction(PROVIDER_ID);
    }
  }

  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    // initial form
    LoginFormsProvider form = context.form();
    context.getRealm().getRequiredActionProvidersStream();
    form.setAttribute("realm", context.getRealm());
    form.setAttribute("user", context.getUser());

    Map<String, String> filteredCredentialTypes =
        credentialTypes.entrySet().stream()
            .filter(
                entry -> {
                  String requiredActionId = entry.getValue();
                  return context
                      .getRealm()
                      .getRequiredActionProvidersStream()
                      .anyMatch(
                          action -> {
                            log.infov(
                                "action.id={0}, enabled={1}",
                                action.getAlias(), action.isEnabled());
                            return (action.isEnabled()
                                && action.getAlias().equals(requiredActionId));
                          });
                })
            .collect(Collectors.toMap(Map.Entry::getKey, Map.Entry::getValue));
    form.setAttribute("credentialOptions", filteredCredentialTypes);
    context.challenge(form.createForm(TPL_SELECTOR));
  }

  @Override
  public void processAction(RequiredActionContext context) {
    // submitted form
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    String requiredActionName = formData.getFirst("requiredActionName");

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    authSession.addRequiredAction(requiredActionName);

    authSession.removeRequiredAction(PROVIDER_ID);
    context.success();
  }

  @Override
  public String getDisplayText() {
    return "Configure MFA method to use";
  }

  @Override
  public RequiredActionProvider create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
