<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
	<#if section = "header">
<div>
    <div>
     <#if address??>
     <div>
		${msg("messageOtpAuthTitleAddress")}
     </div>
     <div>
        ${address}
     </div>
        <#else>
        ${msg("messageOtpAuthTitle")}
        </#if>
    </div>
    <#if ttl??>
    <div>
        <#assign ttlSeconds = ttl?number>
        <#assign ttlMinutes = ttlSeconds / 60>
        <#assign roundedMinutes = (ttlMinutes)?round>
            <span style="font-size: smaller;">
                ${msg("messageOtpAuthTTLTime",roundedMinutes)}
            </span>
    </div>
    </#if>
    </div>
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
            <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                <button
                    id="resend-otp-btn"
                    type="button" 
                    name="resend"
                    value="true"
                    class="${properties.kcButtonClass!} ${properties.kcButtonSecondaryClass!}"
                    onclick="resendOtp(${(resendTimer)!"60"})"
                    >
                    ${msg("resendOtp", "Resend OTP")}
                </button>
            </div>

<script>
let resendTimer = "${msg("resendOtpTimer")}"
let resendButtonText = "${msg("resendOtpButton")}"
function resendOtp(resendTimer) {
    let resendBtn = document.getElementById('resend-otp-btn');
    let form = document.getElementById('kc-message-code-login-form');
    localStorage.setItem('resendOtpEndTime', Date.now() + resendTimer * 1000);
    localStorage.setItem('resendOtpDisabled', true);

    let hiddenInput = document.createElement("input");
    hiddenInput.type = "hidden";
    hiddenInput.name = "resend";
    hiddenInput.value = "true";
    form.appendChild(hiddenInput);

    form.submit();
}

document.addEventListener('DOMContentLoaded', (event) => {
    updateButtonState();
});

function updateButtonState() {
    let resendBtn = document.getElementById('resend-otp-btn');
    let endTime = localStorage.getItem('resendOtpEndTime');
    let disabled = localStorage.getItem('resendOtpDisabled') === 'true';
    let countdown = Math.max(Math.ceil((endTime - Date.now()) / 1000), 0);

    if (disabled) {
        resendBtn.disabled = true;
        let interval = setInterval(() => {
        if (countdown > 0) {
            resendBtn.innerText = resendTimer.replace("{0}", countdown);
            countdown--;
        } else {
            clearInterval(interval);
            resendBtn.disabled = false;
            resendBtn.innerText = resendButtonText;
        }
    }, 1000);
    } else {
        resendBtn.disabled = false;
        resendBtn.innerText = resendButtonText;
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
