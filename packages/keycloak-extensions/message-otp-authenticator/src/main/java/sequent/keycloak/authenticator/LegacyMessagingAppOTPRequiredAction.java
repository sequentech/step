// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import org.keycloak.authentication.RequiredActionFactory;

/**
 * The contact change under its first-release id, {@code messaging-app-otp-ra}: realms and users
 * that still name it keep working, with the same behaviour as {@code message-channel-ra}. A
 * required action's id is its factory's, so the alias is a second factory.
 */
@AutoService(RequiredActionFactory.class)
public class LegacyMessagingAppOTPRequiredAction extends ResetMessagingAppOTPRequiredAction {

  @Override
  public String getId() {
    return LEGACY_PROVIDER_ID;
  }

  @Override
  public String getDisplayText() {
    return "Change how we contact you (previous id)";
  }
}
