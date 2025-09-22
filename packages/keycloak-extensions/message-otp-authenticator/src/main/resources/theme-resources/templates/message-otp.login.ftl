<#--
SPDX-FileCopyrightText: 2024-2025 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header" || section = "show-username">
        <h1>
            <#if isOtl>
                ${msg("messageOtp.otl.title")}
            <#else>
                ${msg("messageOtp.auth.title")}
            </#if>
        </h1>
        <#-- the following already declared at template.ftl for "show-username" case. --->
        <#if section = "header" && section != "show-username">
            <div id="kc-username" class="${properties.kcFormGroupClass!}">
                <label id="kc-attempted-username">${address}</label>
                <a id="reset-login" href="${url.loginRestartFlowUrl}" aria-label="${msg("restartRegistrationTooltip")}">
                    <div class="kc-login-tooltip">
                        <i class="${properties.kcResetFlowIcon!}"></i>
                        <span class="kc-tooltip-text">${msg("restartRegistrationTooltip")}</span>
                    </div>
                </a>
            </div>
        </#if>
	<#elseif section = "form">
		<form
			id="kc-message-code-login-form"
			class="${properties.kcFormClass!}"
			action="${url.loginAction}"
			method="POST"
		>
            <#if !isOtl>
                <div class="${properties.kcFormGroupClass!}">
                      <div class="otp-container" id="otp-inputs">
                    <#assign otpLength = codeLength?number> 
                    <#list 1..otpLength as i>
                        <input
                            autocomplete="off"
                            type="text"
                            inputmode="numeric"
                            pattern="\d"
                            id="otp-${i}"
                            name="otp${i}"
                            maxlength="1"
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
                    onclick="resendOtp(${(resendTimer)})"
                    >
                </button>
            </div>

            <script>
                <#if isOtl>
                    let resendTimerI18n = "${msg("messageOtp.otl.resend.timer")}";
                    let resendButtonI18n = "${msg("messageOtp.otl.resend.button")}";
                <#else>
                    let resendTimerI18n = "${msg("messageOtp.auth.resend.timer")}";
                    let resendButtonI18n = "${msg("messageOtp.auth.resend.button")}";
                </#if>
                let resendTimerTimeout = ${(resendTimer)};;
                let codeJustSent = "${(codeJustSent?string('true', 'false'))}";
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

                    otpInputs.forEach((input, index) => {
                        input.addEventListener('input', (e) => {
                            if (input.value.length === 1 && index < otpInputs.length - 1) {
                                otpInputs[index + 1].focus();
                                otpInputs[index + 1].select();
                            }
                            else if (index === otpInputs.length - 1) {
                                document.getElementById('kc-form-submit').focus();
                            }
                        });

                        input.addEventListener('keydown', (e) => {
                            if (e.key === 'Backspace' && input.value.length === 0 && index > 0) {
                                otpInputs[index - 1].focus();
                                otpInputs[index - 1].select();
                            } else if (e.key === 'Backspace' && input.value.length === 1 && index > 0) {
                                otpInputs[index].value = '';
                                otpInputs[index - 1].focus();
                                otpInputs[index - 1].select();
                            } else if (e.key === 'Backspace' && input.value.length === 1 && index === 0) {
                                otpInputs[index].value = '';
                            }
                            else if (e.key === 'ArrowLeft' && index > 0) {
                                otpInputs[index - 1].focus();
                            }
                            else if (e.key === 'ArrowRight' && index < otpInputs.length - 1) {
                                otpInputs[index + 1].focus();
                            }
                            else if (e.key === 'ArrowRight' && index === otpInputs.length - 1) {
                                document.getElementById('kc-form-submit').focus();
                            }
                        });

                        input.addEventListener('paste', (e) => {
                            const pasteDataTrim = e.clipboardData
                                .getData('text')
                                .trim();
                            const pasteData = pasteDataTrim
                                .substring(0, otpInputs.length);
                            pasteData.split('').forEach((char, i) => {
                                if (i < otpInputs.length) {
                                    otpInputs[i].value = char;
                                }
                            });
                            if (pasteDataTrim.length >= otpInputs.length) {
                                document.getElementById('kc-form-submit').focus();
                            } else {
                                otpInputs[pasteDataTrim.length + 1].focus();
                                otpInputs[pasteDataTrim.length + 1].select();
                            }
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
