<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#-- One channel per voter: the page names it and its masked destination (NAMED), or nothing about
     the account at all (NEUTRAL, the reset flow). There is no choice of channel and no other way to
     get the code. Five wrong codes finish the step. -->
<#assign exhausted = (attemptsExhausted!false)>
<@layout.registrationLayout displayInfo=!exhausted; section>
    <#if section = "header">
            <#if exhausted>
                ${msg("messageOtp.auth.title")}
            <#elseif (codeDisclosure!'') == 'NEUTRAL'>
                <#-- The reset page: nothing about the account, the same for an unknown one. -->
                <span id="otp-neutral">${msg("messageOtp.auth.neutralSent")}</span>
            <#elseif isOtl>
                ${msg("messageOtp.otl.address", address)}
            <#elseif channel?? && (deliveryState!'') == 'UNKNOWN'>
                <span role="status">${msg("messageOtp.delivery.unknown")}</span>
            <#elseif channel?? && (deliveryState!'') == 'FAILED'>
                <span role="alert">${msg("messageOtp.delivery.failed", msg("messageChannel." + channel))}</span>
            <#elseif channel?? && channel == 'MESSENGER'>
                ${msg("messageOtp.auth.title")}
            <#elseif channel??>
                <#if address?has_content>
                    ${msg("messageOtp.auth.sentTo", msg("messageChannel." + channel), address)}
                <#else>
                    ${msg("messageOtp.auth.sentToChannel", msg("messageChannel." + channel))}
                </#if>
            <#else>
                ${msg("messageOtp.auth.address", address)}
            </#if>
	<#elseif section = "form" && exhausted>
		<div id="kc-otp-restart" class="${properties.kcFormGroupClass!}">
			<a id="kc-otp-restart-link" href="${url.loginRestartFlowUrl}"
				class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">${msg("messageOtp.auth.restart")}</a>
		</div>
	<#elseif section = "form">
		<#-- A neutral page (reset) shows the Messenger box to everyone when the event offers
		     Messenger for codes (messengerConnect): the link is live for a Messenger account and
		     binds to nothing otherwise, and no link state is shown. -->
		<#assign neutralMessenger = (messengerConnect!false)>
		<#if (channel?? && channel == 'MESSENGER') || neutralMessenger>
			<div id="kc-messenger-connect" class="${properties.kcFormGroupClass!}">
				<#if neutralMessenger>
					<h2>${msg("messageOtp.messenger.ifMessenger")}</h2>
				<#else>
					<h2>${msg("messageOtp.messenger.title")}</h2>
				</#if>
				<#if messengerPage?? && messagingOrganization??>
					<p>${msg("messageOtp.messenger.introPage", messagingOrganization, messengerPage)}</p>
				<#elseif messengerPage??>
					<p>${msg("messageOtp.messenger.intro", messengerPage)}</p>
				</#if>
				<#if (messengerReconnect!false)>
					<p class="kc-messenger-reconnect">${msg("messageOtp.messenger.reconnect")}</p>
				</#if>
				<#-- On a phone: open the chat here. On a computer: scan the QR code with the phone. -->
				<ol class="kc-messenger-steps-phone">
					<li>${msg("messageOtp.messenger.step1")}</li>
					<li>${msg("messageOtp.messenger.step2")}</li>
					<li>${msg("messageOtp.messenger.step3")}</li>
				</ol>
				<#if messengerQr??>
					<div class="kc-messenger-desktop">
						<img id="kc-messenger-qr" src="${messengerQr}" width="200" height="200" alt="${msg("messageOtp.messenger.scan")}"/>
						<ol>
							<li>${msg("messageOtp.messenger.scanStep1")}</li>
							<li>${msg("messageOtp.messenger.scanStep2")}</li>
							<li>${msg("messageOtp.messenger.scanStep3")}</li>
						</ol>
					</div>
				</#if>
				<#if messengerLink??>
					<a id="kc-messenger-link" href="${messengerLink}" target="_blank" rel="noopener noreferrer"
						class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">${msg("messageOtp.messenger.open")}</a>
				</#if>
				<p class="kc-messenger-get-started">${msg("messageOtp.messenger.getStarted")}</p>
				<#if (messengerState!'') == 'CODE_SENT'>
					<p role="status">${msg("messageOtp.messenger.codeSent")}</p>
				<#elseif (messengerState!'') == 'EXPIRED' || (messengerState!'') == 'REPLACED'>
					<p role="alert">${msg("messageOtp.messenger.expired")}</p>
				<#elseif messengerState??>
					<p role="status">${msg("messageOtp.messenger.pending")}</p>
				</#if>
			</div>
			<style>
				.kc-messenger-desktop { display: none; }
				@media (min-width: 768px) and (hover: hover) {
					.kc-messenger-desktop { display: block; text-align: center; }
					.kc-messenger-steps-phone { display: none; }
				}
			</style>
		</#if>
		<form
			id="kc-message-code-login-form"
			class="${properties.kcFormClass!}"
			action="${url.loginAction}"
			method="POST"
		>
            <#if !isOtl>
                <div class="${properties.kcFormGroupClass!}">
                      <div class="otp-container" id="otp-inputs" role="group"
                           aria-label="${msg('otpCodeLabel')}" aria-describedby="otp-instructions">
                    <#assign otpLength = (codeLength!"6")?number> 
                    <#list 1..otpLength as i>
                        <input
                            autocomplete="<#if i == 1>one-time-code<#else>off</#if>"
                            type="text"
                            inputmode="numeric"
                            pattern="\d"
                            id="otp-${i}"
                            name="otp${i}"
                            maxlength="${otpLength?c}"
                            aria-label="${msg('otpDigit', i?c, otpLength?c)}"
                            class="otp-input"
                            <#if i == 1> autofocus="autofocus" </#if> />
                    </#list>
                </div>
                </div>
                <input type="hidden" id="code" name="code" />
                <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                    <div
                        id="kc-form-buttons"
                        class="${properties.kcFormButtonsClass!}"
                    >
                        <input
                            id="kc-form-submit"
                            class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                            type="submit"
                            value="${msg("doSubmit")}"
                            onclick="handleOtpInput()"
                        />
                    </div>
                </div>
            </#if>
            <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                <button
                    id="resend-otp-btn"
                    type="button" 
                    name="resend"
                    value="true"
                    class="${properties.kcButtonClass!} ${properties.kcButtonSecondaryClass!}"
                    onclick="resendOtp(${(resendTimer)!60})"
                    >
                </button>
            </div>
            <#if channel?? && channel == 'MESSENGER'>
                <div class="${properties.kcFormGroupClass!}">
                    <button type="submit" name="messengerStatus" value="true" formnovalidate
                        class="${properties.kcButtonClass!} ${properties.kcButtonSecondaryClass!}">${msg("messageOtp.messenger.check")}</button>
                </div>
            </#if>

            <script>
                <#if isOtl>
                    let resendTimerI18n = "${msg("messageOtp.otl.resend.timer")?no_esc}";
                    let resendButtonI18n = "${msg("messageOtp.otl.resend.button")?no_esc}";
                <#else>
                    let resendTimerI18n = "${msg("messageOtp.auth.resend.timer")?no_esc}";
                    let resendButtonI18n = "${msg("messageOtp.auth.resend.button")?no_esc}";
                </#if>
                let resendTimerTimeout = ${(resendTimer)!60};
                // Every send starts the countdown, including one that failed.
                let codeJustSent = "${((resendTimerStarted!codeJustSent!false)?string('true', 'false'))}";
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

                    const otpInputs = document.querySelectorAll('.otp-input');

                    function focusOtp(index) {
                        if (index >= otpInputs.length) {
                            document.getElementById('kc-form-submit').focus();
                        } else {
                            otpInputs[index].focus();
                            otpInputs[index].select();
                        }
                    }

                    function syncOtpCode() {
                        const code = document.getElementById('code');
                        if (code) code.value = Array.from(otpInputs, input => input.value).join('');
                    }

                    function fillOtp(value) {
                        const digits = value.replace(/\D/g, '').slice(0, otpInputs.length);
                        if (!digits) return;
                        otpInputs.forEach((input, index) => input.value = digits[index] || '');
                        syncOtpCode();
                        focusOtp(digits.length);
                    }

                    otpInputs.forEach((input, index) => {
                        input.addEventListener('input', (e) => {
                            // SMS autofill/password managers deliver the complete code at once.
                            if (input.value.length > 1) {
                                fillOtp(input.value);
                                return;
                            }
                            input.value = input.value.replace(/\D/g, '');
                            syncOtpCode();
                            if (input.value) {
                                focusOtp(index + 1);
                            }
                        });

                        input.addEventListener('keydown', (e) => {
                            if (e.key === 'Backspace') {
                                e.preventDefault();
                                input.value = '';
                                syncOtpCode();
                                focusOtp(Math.max(0, index - 1));
                            }
                            else if (e.key === 'ArrowLeft' && index > 0) {
                                e.preventDefault();
                                focusOtp(index - 1);
                            }
                            else if (e.key === 'ArrowRight') {
                                e.preventDefault();
                                focusOtp(index + 1);
                            }
                        });

                        input.addEventListener('paste', (e) => {
                            e.preventDefault();
                            fillOtp(e.clipboardData.getData('text'));
                        });
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
                            console.log(`updateButtonState: CODE/LINK JUST SENT endTime=${endTime}, disabled=${disabled}`);
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

                    function handleOtpInput() {
                    const form = document.getElementById('kc-message-code-login-form');
                    const code = document.getElementById('code');
                    const otpInputs = document.querySelectorAll('.otp-input');
                        let otp = '';
                        otpInputs.forEach((input) => {
                            otp += input.value;
                        });
                        code.value = otp;
                        form.submit();
                    }

                
                </#noparse>
            </script>

            <style>
    .otp-container {
        display: flex;
        justify-content: center;
        margin: 20px 0;
    }

    .otp-input {
        width: 40px;
        height: 50px;
        font-size: 18px;
        text-align: center;
        border: 1px solid #ccc;
        border-radius: 8px;
        margin: 0 8px;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
    }

    .otp-input:focus {
        border-color: #007bff;
        outline: none;
        box-shadow: 0 2px 5px rgba(0, 123, 255, 0.5);
    }

</style>
		</form>
		<#if channel?? && channel == 'MESSENGER'>
			<#include "messenger-status-poll.ftl">
		</#if>
	<#elseif section = "info">
        <p id="otp-instructions" class="kc-message-otl-instructions">
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
                    <#elseif channel?? && senderLabel?? && ['WHATSAPP', 'VIBER']?seq_contains(channel) && !['UNKNOWN', 'FAILED']?seq_contains(deliveryState!'')>
                        ${msg("messageOtp.auth.openApp", msg("messageChannel." + channel), senderLabel)}
                    <#elseif channel?? && channel = "EMAIL">
                        ${msg("messageOtp.auth.instructionEmail")}
                    <#elseif channel?? && channel = "SMS">
                        ${msg("messageOtp.auth.instructionSms")}
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
