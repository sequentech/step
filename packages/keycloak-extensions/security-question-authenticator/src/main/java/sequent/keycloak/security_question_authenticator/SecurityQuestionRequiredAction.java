// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.security_question_authenticator;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.security.MessageDigest;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.apache.commons.lang3.StringUtils;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.OTPCredentialModel;
import org.keycloak.sessions.AuthenticationSessionModel;

@JBossLog
public class SecurityQuestionRequiredAction implements RequiredActionProvider {

  // map of key-value pairs:
  // - key = credential type
  // - value = associated required action id
  // {
  //  	"otp": "CONFIGURE_TOTP",
  // 		"message-otp": "message-otp-ra"
  // }
  private static final Map<String, String> credentialTypes =
      Map.of(
          OTPCredentialModel.TYPE,
          UserModel.RequiredAction.CONFIGURE_TOTP.name(),
          "message-otp",
          "message-otp-ra");
  public static final String PROVIDER_ID = "security-question-ra";

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

    String alreadyExecuted = authSession.getAuthNote(Utils.ALREADY_EXECUTED_SECURITY_QUESTION);
    if (alreadyExecuted != null) {
      // finish, already executed!
      return;
    }

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
    log.info("authenticate()");
    Response challenge = context.form().createForm(Utils.SECURITY_QUESTION_FORM);
    context.challenge(challenge);
  }

  @Override
  public void processAction(RequiredActionContext context) {
    log.info("action()");
    boolean validated = validateAnswer(context);
    if (!validated) {
      // invalid
      context.challenge(
          context
              .form()
              .setAttribute("realm", context.getRealm())
              .setError("secretQuestionInvalid")
              .createErrorPage(Response.Status.BAD_REQUEST));
    } else {
      // valid
      AuthenticationSessionModel authSession = context.getAuthenticationSession();

      authSession.setAuthNote(Utils.ALREADY_EXECUTED_SECURITY_QUESTION, "true");
      context.success();
    }
  }

  protected boolean validateAnswer(RequiredActionContext context) {
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    String secretAnswer = formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD);
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).get();
    UserModel user = context.getUser();

    String numLastCharsString = config.getConfig().get(Utils.NUM_LAST_CHARS);
    String userAttributeName = config.getConfig().get(Utils.USER_ATTRIBUTE);
    String userAttributeValue = user.getFirstAttribute(userAttributeName);
    log.info(
        "comparing userAttributeValue="
            + userAttributeValue
            + ", secretAnswer="
            + secretAnswer
            + ", numLastChars="
            + numLastCharsString);
    if (userAttributeValue == null || numLastCharsString == null) {
      return false;
    }
    int numLastChars = Integer.parseInt(numLastCharsString);

    // We use constant time comparison for security reasons, to avoid timing
    // attacks
    boolean isValid =
        MessageDigest.isEqual(
            StringUtils.right(userAttributeValue, numLastChars).getBytes(),
            StringUtils.right(secretAnswer, numLastChars).getBytes());
    return isValid;
  }

  @Override
  public void close() {}
}
