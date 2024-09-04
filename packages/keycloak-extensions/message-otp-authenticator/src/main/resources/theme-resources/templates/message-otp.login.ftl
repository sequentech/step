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
                        <#if isOtl>
                            ${msg("messageOtp.otl.address")}
                        <#else>
                            ${msg("messageOtp.auth.address")}
                        </#if>
                    </div>
                    <div>
                        ${address}
                    </div>
                <#else>
                    <#if isOtl>
                        ${msg("messageOtp.otl.title", realm.displayName)}
                    <#else>
                        ${msg("messageOtp.auth.title", realm.displayName)}
                    </#if>
                </#if>
            </div>
        </div>
    <#elseif section = "show-username">
        <h1>
            <#if isOtl>
                ${msg("messageOtp.otl.title", realm.displayName)}
            <#else>
                ${msg("messageOtp.auth.title", realm.displayName)}
            </#if>
        </h1>
	<#elseif section = "form">
        <#if !isOtl>
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
						${msg("messageOtp.auth.label")}
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
                    onclick="resendOtp(${(resendTimer)})"
                    >
                </button>
            </div>

<script>
    let resendTimerI18n = "${msg("messageOtp.auth.resend.timer")}"
    let resendTimerTimeout = ${(resendTimer)};
    let resendButtonI18n = "${msg("messageOtp.auth.resend.button")}"
    let codeJustSent = "${(codeJustSent?string('true', 'false'))}"
    <#noparse>
    function resendOtp(resendTimerTimeout) {
        let resendBtn = document.getElementById('resend-otp-btn');
        let form = document.getElementById('kc-message-code-login-form');
        localStorage.setItem('resendOtpEndTime', Date.now() + resendTimerTimeout * 1000);
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
        console.log("updateButtonState");
        let resendBtn = document.getElementById('resend-otp-btn');
        var endTime = localStorage.getItem('resendOtpEndTime');
        var disabled = localStorage.getItem('resendOtpDisabled') === 'true';
        var now = Date.now();
        console.log(`updateButtonState: endTime=${endTime}, disabled=${disabled}`);
        if (codeJustSent === "true") {
            endTime = now + resendTimerTimeout * 1000;
            localStorage.setItem('resendOtpEndTime', endTime);
            localStorage.setItem('resendOtpDisabled', true);
            console.log(`updateButtonState: CODE JUST SENT endTime=${endTime}, disabled=${disabled}`);
        }
        let countdown = Math.max(Math.ceil((endTime - now) / 1000), 0);
        console.log(`updateButtonState: countdown=${countdown}`);

        if (disabled) {
            console.log(`updateButtonState: yes, disabled`);
            resendBtn.disabled = true;
            let interval = setInterval(() => {
                console.log(`updateButtonState: setInterval, countdown=${countdown}`);
                if (countdown > 0) {
                    resendBtn.innerText = resendTimerI18n.replace("{0}", countdown);
                    countdown--;
                } else {
                    clearInterval(interval);
                    resendBtn.disabled = false;
                    resendBtn.innerText = resendButtonI18n;
                }
            }, 1000);
        } else {
            console.log(`updateButtonState: not disabled`);
            resendBtn.disabled = false;
            resendBtn.innerText = resendButtonI18n;
        }
    }
    </#noparse>
</script>

		</form>
        </#if>
	<#elseif section = "info">
        <p class="kc-message-otl-instructions">
            <#if isOtl>
                    <#if courier??>
                        <#if courier = "SMS">
                            ${msg("messageOtp.otl.instructionSms")}
                        <#elseif courier = "EMAIL" >
                            ${msg("messageOtp.otl.instructionEmail")}
                        <#elseif courier = "BOTH" >
                            ${msg("messageOtp.otl.instructionBoth")}
                        </#if>
                    <#else>
                        ${msg("messageOtp.otl.instructionBoth")}
                    </#if>
                </p>
            <#else>
                <#if courier??>
                    <#if courier = "SMS">
                        ${msg("messageOtp.auth.instructionSms")}
                    <#elseif courier = "EMAIL" >
                        ${msg("messageOtp.auth.instructionEmail")}
                    <#elseif courier = "BOTH" >
                        ${msg("messageOtp.auth.instructionBoth")}
                    </#if>
                <#else>
                    ${msg("messageOtp.auth.instructionBoth")}
                </#if>
			</#if>
        </p>
        <#if ttl??>
            <div>
                <#assign ttlSeconds = ttl?number>
                <#assign ttlMinutes = ttlSeconds / 60>
                <#assign roundedMinutes = (ttlMinutes)?round>
                    <span>
                        <#if isOtl>
                            ${msg("messageOtp.otl.ttlTime",roundedMinutes)}
                        <#else>
                            ${msg("messageOtp.auth.ttlTime",roundedMinutes)}
                        </#if>
                    </span>
            </div>
        </#if>
	</#if>
</@layout.registrationLayout>
