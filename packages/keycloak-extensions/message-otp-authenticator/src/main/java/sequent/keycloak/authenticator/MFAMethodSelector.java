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
  // - value = list of associated required action ids
  // {
  //  	"otp": ["CONFIGURE_TOTP"],
  // 		"message-otp": ["message-otp-ra", "email-otp-ra", "mobile-otp-ra"]
  // }
  private static final Map<String, java.util.List<String>> credentialTypes =
      Map.of(
          // Normal OTP
          OTPCredentialModel.TYPE,
          java.util.List.of(UserModel.RequiredAction.CONFIGURE_TOTP.name()),

          // Message OTP (SMS and Email)
          MessageOTPCredentialModel.TYPE,
          java.util.List.of(
              ResetMessageOTPRequiredAction.PROVIDER_ID,
              ResetEmailOTPRequiredAction.PROVIDER_ID,
              ResetMobileOTPRequiredAction.PROVIDER_ID),

          // Passkeys
          WebAuthnCredentialModel.TYPE_PASSWORDLESS,
          java.util.List.of(WebAuthnPasswordlessRegisterFactory.PROVIDER_ID));

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

    // Only add the MFA method selector required action if:
    if (
    // 1. The selector itself is not already in the session's required actions
    authSession.getRequiredActions().stream().noneMatch(PROVIDER_ID::equals)
        &&
        // 2. The user does not have any credential of the supported types (OTP,
        //    message-otp, passkey)
        credentialTypes.keySet().stream()
            .noneMatch(
                type -> {
                  boolean ret =
                      user.credentialManager()
                          .getStoredCredentialsByTypeStream(type)
                          .findAny()
                          .isPresent();
                  log.info(
                      "evaluateTriggers(): credentialTypes: type=" + type + ", userHasAny=" + ret);
                  return ret;
                })
        &&
        // 3. The user does not already have any of the required actions for the
        //    supported credential types
        user.getRequiredActionsStream()
            .noneMatch(
                ra -> {
                  boolean match =
                      credentialTypes.values().stream()
                          .flatMap(java.util.List::stream)
                          .anyMatch(ra::equals);
                  log.info(
                      "evaluateTriggers(): user requiredAction="
                          + ra
                          + ", isSupportedType="
                          + match);
                  return match;
                })
        &&
        // 4. The session does not already have any of the required actions for
        //    the supported credential types
        authSession.getRequiredActions().stream()
            .noneMatch(
                ra -> {
                  boolean match =
                      credentialTypes.values().stream()
                          .flatMap(java.util.List::stream)
                          .anyMatch(ra::equals);
                  log.info(
                      "evaluateTriggers(): session requiredAction="
                          + ra
                          + ", isSupportedType="
                          + match);
                  return match;
                })) {
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

    // Flatten all required actions and filter only enabled ones
    java.util.List<String> enabledRequiredActions =
        credentialTypes.values().stream()
            .flatMap(java.util.List::stream)
            .filter(
                requiredActionId ->
                    context
                        .getRealm()
                        .getRequiredActionProvidersStream()
                        .anyMatch(
                            action -> {
                              log.infov(
                                  "action.id={0}, enabled={1}",
                                  action.getAlias(), action.isEnabled());
                              return action.isEnabled()
                                  && action.getAlias().equals(requiredActionId);
                            }))
            .distinct()
            .collect(Collectors.toList());

    form.setAttribute("requiredActions", enabledRequiredActions);
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
