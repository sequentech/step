// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.Response;
import java.util.Optional;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

@JBossLog
/**
 * RequiredActionProvider for resetting and verifying a user's email address
 * using an OTP sent to the provided email.
 *
 * <p>Flow:
 * <ol>
 *   <li>Prompts the user to enter a new email address.</li>
 *   <li>Sends an OTP to the entered email address.</li>
 *   <li>Prompts the user to enter the OTP.</li>
 *   <li>On successful verification, saves an email OTP credential and updates 
 *       the user's email.</li>
 * </ol>
 *
 * <p>All state is managed via AuthenticationSessionModel notes.
 */
public class ResetEmailOTPRequiredAction implements RequiredActionProvider {
    public static final String PROVIDER_ID = "email-otp-ra";
    private static final String FTL_EMAIL_ENTRY = "email-otp.enter-email.ftl";
    private static final String FTL_EMAIL_OTP = "email-otp.enter-otp.ftl";

    /** Session note key for the email address being verified. */
    public static final String NOTE_EMAIL_ADDRESS = "email-otp-address";
    /** Session note key for the OTP code. */
    public static final String NOTE_OTP_CODE = "email-otp-code";
    /** Session note key for the OTP expiry timestamp. */
    public static final String NOTE_OTP_TTL = "email-otp-ttl";

    /**
     * Indicates this required action supports being initiated by the user or
     * admin.
     */
    @Override
    public InitiatedActionSupport initiatedActionSupport() {
        return InitiatedActionSupport.SUPPORTED;
    }

    @Override
    public void evaluateTriggers(RequiredActionContext context) {}

    /**
     * Presents the appropriate challenge to the user: email entry or OTP entry.
     */
    @Override
    public void requiredActionChallenge(RequiredActionContext context) {
        AuthenticationSessionModel authSession = context.getAuthenticationSession();
        String email = authSession.getAuthNote(NOTE_EMAIL_ADDRESS);
        if (email == null) {
            // Prompt for email address
            context.challenge(createEmailEntryForm(context, null));
        } else {
            // Prompt for OTP
            context.challenge(createOTPForm(context, null));
        }
    }

    /**
     * Handles form submissions for both email entry and OTP entry.
     * Reuses Utils.sendCode for sending the OTP email, and uses config for code length/ttl.
     */
    @Override
    public void processAction(RequiredActionContext context) {
        AuthenticationSessionModel authSession = context.getAuthenticationSession();
        KeycloakSession session = context.getSession();
        AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
        String email = authSession.getAuthNote(NOTE_EMAIL_ADDRESS);
        if (email == null) {
            // Handle email entry
            String enteredEmail = context.getHttpRequest().getDecodedFormParameters().getFirst("email");
            if (enteredEmail == null || !enteredEmail.contains("@")) {
                context.challenge(createEmailEntryForm(context, form -> form.setError("emailOtp.auth.invalidEmail")));
                return;
            }
            // Save email in session and send OTP using shared logic
            authSession.setAuthNote(NOTE_EMAIL_ADDRESS, enteredEmail);
            try {
                // Use Utils.sendCode to generate/store code and send email
                Utils.sendCode(
                    config,
                    session,
                    context.getUser(),
                    authSession,
                    Utils.MessageCourier.EMAIL,
                    /*deferredUser*/ false,
                    /*isOtl*/ false,
                    new String[0],
                    context
                );
            } catch (Exception e) {
                context.challenge(createEmailEntryForm(context, form -> form.setError("emailOtp.auth.sendError")));
                return;
            }
            context.challenge(createOTPForm(context, null));
            return;
        }
        // Handle OTP entry
        String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst("otp");
        String code = authSession.getAuthNote(Utils.CODE);
        String ttl = authSession.getAuthNote(Utils.CODE_TTL);
        if (enteredCode == null || code == null || ttl == null) {
            context.failure();
            return;
        }
        boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
        if (isValid) {
            if (Long.parseLong(ttl) < System.currentTimeMillis()) {
                context.challenge(createOTPForm(context, form -> form.setError("emailOtp.auth.codeExpired")));
                return;
            }
            // Save credential and update user email
            MessageOTPCredentialProvider credentialProvider = new MessageOTPCredentialProvider(session);
            credentialProvider.createCredential(
                context.getRealm(),
                context.getUser(),
                MessageOTPCredentialModel.create(/* isSetup= */ true)
            );
            context.getUser().setEmail(email);
            context.getUser().setEmailVerified(true);
            context.getUser().removeRequiredAction(PROVIDER_ID);
            context.getAuthenticationSession().removeRequiredAction(PROVIDER_ID);
            context.success();
        } else {
            context.challenge(createOTPForm(context, form -> form.setError("emailOtp.auth.codeInvalid")));
        }
    }

    @Override
    public void close() {}

    /**
     * Creates the form for entering the email address.
     */
    private Response createEmailEntryForm(RequiredActionContext context, Consumer<LoginFormsProvider> formConsumer) {
        LoginFormsProvider form = context.form();
        if (formConsumer != null) {
            formConsumer.accept(form);
        }
        return form.createForm(FTL_EMAIL_ENTRY);
    }

    /**
     * Creates the form for entering the OTP code.
     */
    private Response createOTPForm(
        RequiredActionContext context,
        Consumer<LoginFormsProvider> formConsumer
    ) {
        LoginFormsProvider form = context.form();
        form.setAttribute("email", context.getAuthenticationSession().getAuthNote(NOTE_EMAIL_ADDRESS));
        if (formConsumer != null) {
            formConsumer.accept(form);
        }
        return form.createForm(FTL_EMAIL_OTP);
    }

    /**
     * Generates and sends an OTP to the given email address, storing the code and expiry in the session.
     *
     * @deprecated Use Utils.sendCode instead for consistency.
     */
    @Deprecated
    private void sendEmailOTP(RequiredActionContext context, String email) throws Exception {
        // Deprecated: replaced by Utils.sendCode
        throw new UnsupportedOperationException("Use Utils.sendCode instead");
    }
}
