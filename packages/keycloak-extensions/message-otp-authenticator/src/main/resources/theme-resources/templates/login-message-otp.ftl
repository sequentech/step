<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
	<#if section = "header">
		${msg("messageOtpAuthTitle", realm.displayName)}
	<#elseif section = "show-username">
		<h1>${msg("messageOtpAuthTitle", realm.displayName)}</h1>
	<#elseif section = "form">
		<form
			id="kc-message-code-login-form"
			class="${properties.kcFormClass!}"
			action="${url.loginAction}"
			method="POST"
		>
			<div class="${properties.kcFormGroupClass!}">
				<div class="${properties.kcLabelWrapperClass!}">
					<label
						for="code"
						class="${properties.kcLabelClass!}"
					>
						${msg("messageOtpAuthLabel")}
					</label>
				</div>
				<div class="${properties.kcInputWrapperClass!}">
					<input 
						type="text"
						id="code"
						name="code"
						class="${properties.kcInputClass!}"
						autofocus
					/>
				</div>
			</div>
			<div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
				<div
					id="kc-form-buttons"
					class="${properties.kcFormButtonsClass!}"
				>
					<input
						class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
						type="submit"
						value="${msg("doSubmit")}"
					/>
				</div>
			</div>
<div class="${properties.kcFormGroupClass!}">
    <button
        id="resend-otp-btn"
        type="button" 
        name="resend"
        value="true"
        class="${properties.kcButtonClass!} ${properties.kcButtonSecondaryClass!}"
        onclick="resendOtp()"
    >
        ${msg("resendOtp", "Resend OTP")}
    </button>
</div>

<script>
function resendOtp() {
    let resendBtn = document.getElementById('resend-otp-btn');
    let form = document.getElementById('kc-message-code-login-form');
    let countdown = 60;

    resendBtn.disabled = true;
    localStorage.setItem('resendOtpEndTime', Date.now() + countdown * 1000);
    localStorage.setItem('resendOtpDisabled', true);

    // Update button text to show countdown
    let interval = setInterval(() => {
        if (countdown > 0) {
            resendBtn.innerText = 'Resend OTP in ' + countdown + ' seconds';
            countdown--;
        } else {
            clearInterval(interval);
            resendBtn.disabled = false;
            resendBtn.innerText = 'Resend OTP';
        }
    }, 1000);

    // Add hidden input to signal resend
    let hiddenInput = document.createElement("input");
    hiddenInput.type = "hidden";
    hiddenInput.name = "resend";
    hiddenInput.value = "true";
    form.appendChild(hiddenInput);

    // Submit the form programmatically
    form.submit();
}

// Initialize button state on page load
document.addEventListener('DOMContentLoaded', (event) => {
    updateButtonState();
});

function updateButtonState() {
    let resendBtn = document.getElementById('resend-otp-btn');
    let endTime = localStorage.getItem('resendOtpEndTime');
    let disabled = localStorage.getItem('resendOtpDisabled') === 'true';
    let countdown = Math.max(Math.ceil((endTime - Date.now()) / 1000), 0);

    if (disabled) {
        if (countdown > 0) {
            resendBtn.disabled = true;
            resendBtn.innerText = 'Resend OTP in ' + countdown + ' seconds';
            setTimeout(updateButtonState, 1000);
        } else {
            resendBtn.disabled = false;
            resendBtn.innerText = 'Resend OTP';
            localStorage.removeItem('resendOtpEndTime');
            localStorage.removeItem('resendOtpDisabled');
        }
    } else {
        resendBtn.disabled = false;
        resendBtn.innerText = 'Resend OTP';
    }
}
</script>
		</form>
	<#elseif section = "info" >
		<#if courier??>
			<#if courier = "SMS">
				${msg("messageOtpAuthInstructionSms")}
			<#elseif courier = "EMAIL" >
				${msg("messageOtpAuthInstructionEmail")}
			<#elseif courier = "BOTH" >
				${msg("messageOtpAuthInstruction")}
			</#if>
		</#if>
	</#if>
</@layout.registrationLayout>
