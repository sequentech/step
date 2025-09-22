// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.*;
import org.keycloak.authentication.authenticators.resetcred.AbstractSetRequiredActionAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class RemoveUserAttribute extends AbstractSetRequiredActionAuthenticator {
  public static final String PROVIDER_ID = "remove-user-attribute";
  public static final String CONF_USER_ATTRIBUTE = "userAttribute";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate()");
    UserModel user = context.getUser();
    if (user == null) {
      log.info("authenticate(): user is null, return");
      return;
    }
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    String userAttributeKey = authConfig.getConfig().get(RemoveUserAttribute.CONF_USER_ATTRIBUTE);

    if (context.getExecution().isRequired()
        || (context.getExecution().isConditional() && configuredFor(context))) {
      log.infov("authenticate(): removing userAttributeKey={0}", userAttributeKey);
      user.removeAttribute(userAttributeKey);
    }
    context.success();
  }

  protected boolean configuredFor(AuthenticationFlowContext context) {
    return true;
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONF_USER_ATTRIBUTE,
            "User attribute to remove",
            "User attribute to remove from the user.",
            ProviderConfigProperty.STRING_TYPE,
            ""));
  }

  @Override
  public String getDisplayType() {
    return "User Attribute - remove";
  }

  @Override
  public String getHelpText() {
    return "Removes the specified user attribute.";
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
