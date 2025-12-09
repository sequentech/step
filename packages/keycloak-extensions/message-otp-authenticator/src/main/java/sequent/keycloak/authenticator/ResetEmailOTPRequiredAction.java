// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * RequiredActionProvider for resetting and verifying a user's email address using an OTP sent to
 * the provided email.
 *
 * <p>Flow:
 *
 * <ol>
 *   <li>Prompts the user to enter a new email address.
 *   <li>Sends an OTP to the entered email address.
 *   <li>Prompts the user to enter the OTP.
 *   <li>On successful verification, saves an email OTP credential and updates the user's email.
 * </ol>
 *
 * <p>All state is managed via AuthenticationSessionModel notes.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class ResetEmailOTPRequiredAction extends BaseResetMessageOTPRequiredAction
    implements RequiredActionFactory {
  public static final String PROVIDER_ID = "email-otp-ra";

  @Override
  protected String getProviderId() {
    return PROVIDER_ID;
  }

  @Override
  protected String getNoteKey(AuthenticationSessionModel authSession) {
    return "email";
  }

  @Override
  protected Utils.MessageCourier getCourier() {
    return Utils.MessageCourier.EMAIL;
  }

  @Override
  protected String getI18nPrefix() {
    return "resetEmailOtp";
  }

  @Override
  protected void saveVerifiedValue(RequiredActionContext context, String value) {
    context.getUser().setEmail(value);
    context.getUser().setEmailVerified(true);
  }

  /** Indicates this required action supports being initiated by the user or admin. */
  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  @Override
  public String getDisplayText() {
    return "Reset and Configure Email OTP";
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
