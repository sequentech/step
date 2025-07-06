// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * RequiredActionProvider for resetting and verifying a user's mobile number using an OTP sent via
 * SMS.
 *
 * <p>Flow:
 *
 * <ol>
 *   <li>Prompts the user to enter a new mobile number.
 *   <li>Sends an OTP to the entered mobile number via SMS.
 *   <li>Prompts the user to enter the OTP.
 *   <li>On successful verification, saves an SMS OTP credential and updates the user's mobile
 *       number.
 * </ol>
 *
 * <p>All state is managed via AuthenticationSessionModel notes.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class ResetMobileOTPRequiredAction extends BaseResetMessageOTPRequiredAction
    implements RequiredActionFactory {
  public static final String PROVIDER_ID = "mobile-otp-ra";

  @Override
  protected String getProviderId() {
    return PROVIDER_ID;
  }

  @Override
  protected String getNoteKey(AuthenticationSessionModel authSession) {
    AuthenticatorConfigModel config = Utils.getConfig(authSession.getRealm()).orElse(null);
    if (config == null) {
      log.error("No configuration found for ResetMobileOTPRequiredAction");
      return "mobile";
    }
    String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
    if (mobileNumberAttribute != null && !mobileNumberAttribute.isEmpty()) {
      return mobileNumberAttribute;
    }
    return "mobile";
  }

  @Override
  protected Utils.MessageCourier getCourier() {
    return Utils.MessageCourier.SMS;
  }

  @Override
  protected String getI18nPrefix() {
    return "mobileOtp";
  }

  @Override
  protected void saveVerifiedValue(RequiredActionContext context, String value) {
    context.getUser().setAttribute("mobile", java.util.Collections.singletonList(value));
  }

  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  @Override
  public String getDisplayText() {
    return "Reset and Configure Mobile OTP";
  }

  @Override
  public RequiredActionProvider create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(org.keycloak.Config.Scope config) {}

  @Override
  public void postInit(org.keycloak.models.KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
