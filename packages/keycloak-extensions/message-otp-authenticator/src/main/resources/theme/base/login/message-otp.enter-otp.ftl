<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header">
        <h2 style="text-align:center;">${msg(i18nPrefix + ".auth.enterOtpTitle")}</h2>
    <#elseif section = "form">
        <#assign viaMessenger = (channel!'') == 'MESSENGER'>
        <#if viaMessenger>
            <div id="kc-messenger-connect" class="${properties.kcFormGroupClass!}">
                <#if messengerPage??><p>${msg("messageOtp.messenger.intro", messengerPage)}</p></#if>
                <ol>
                    <li>${msg("messageOtp.messenger.step1")}</li>
                    <li>${msg("messageOtp.messenger.step2")}</li>
                    <li>${msg("messageOtp.messenger.step3")}</li>
                </ol>
                <#if messengerLink??>
                    <a id="kc-messenger-link" href="${messengerLink}" target="_blank" rel="noopener noreferrer"
                        class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">${msg("messageOtp.messenger.connect")}</a>
                </#if>
                <#if messengerWord?? && messengerPage??>
                    <p>${msg("messageOtp.messenger.word", messengerWord, messengerPage)}</p>
                </#if>
                <#if (messengerState!'') == 'CODE_SENT'>
                    <p role="status">${msg("messageOtp.messenger.codeSent")}</p>
                <#elseif (messengerState!'') == 'EXPIRED' || (messengerState!'') == 'REPLACED'>
                    <p role="alert">${msg("messageOtp.messenger.expired")}</p>
                <#elseif messengerState??>
                    <p role="status">${msg("messageOtp.messenger.pending")}</p>
                </#if>
            </div>
        </#if>
        <form id="kc-message-otp-form" class="${properties.kcFormClass!}" action="${url.loginAction}" method="post">
            <div class="${properties.kcFormGroupClass!} otp-sent-row">
                <span class="otp-sent-label">
                    <#if viaMessenger>
                        ${msg("messageOtp.messenger.title")}
                    <#elseif channel??>
                        ${msg("messageOtp.auth.sentTo", msg("messageChannel." + channel), contact)}
                    <#else>
                        ${msg(i18nPrefix + ".auth.sentToContact", contact)?no_esc}
                    </#if>
                </span>
                <b class="otp-sent-value">${email!}${mobile!}</b>
                <button type="submit" name="changeValue" value="true" class="change-link">
                    ${msg(i18nPrefix + ".auth.changeContact")}
                </button>
            </div>
            <div class="${properties.kcFormGroupClass!} otp-input-row">
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
                            <#if i == 1> autofocus="autofocus" </#if>
                        />
                    </#list>
                </div>
            </div>
            <input type="hidden" id="code" name="code" />
            <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}" style="text-align:center;">
                <div id="kc-form-buttons" class="${properties.kcFormButtonsClass!}">
                    <input
                        id="kc-form-submit"
                        class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit"
                        value="${msg(i18nPrefix + ".auth.verifyButton")}"
                        onclick="handleOtpInput()"
                    />
                </div>
            </div>
        </form>
        <#if viaMessenger>
            <form id="kc-messenger-check-form" class="${properties.kcFormClass!}" action="${url.loginAction}" method="post" style="text-align:center;">
                <button type="submit" name="messengerStatus" value="true"
                    class="${properties.kcButtonClass!} ${properties.kcButtonSecondaryClass!}">${msg("messageOtp.messenger.check")}</button>
            </form>
            <#include "messenger-status-poll.ftl">
        </#if>
        <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!} resend-row">
            <span class="resend-prefix">
                ${msg(i18nPrefix + ".auth.resendTextPrefix.question")}
                <span id="resend-action" class="resend-action"></span>
            </span>
        </div>
        <#if ttl??>
            <div class="ttl-row ${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                <#assign ttlSeconds = ttl?number>
                <#assign ttlMinutes = ttlSeconds / 60>
                <#assign roundedMinutes = (ttlMinutes)?round>
                    <span>${msg(i18nPrefix + ".auth.ttlTime",roundedMinutes)}</span>
            </div>
        </#if>
        <script>
            let resendTimerI18n = "${msg(i18nPrefix + ".auth.resend.timer")}";
            let resendButtonI18n = "${msg(i18nPrefix + ".auth.resend.button.link")}";
            let resendTimerTimeout = ${(resendTimer)};
            let codeJustSent = "${(codeJustSent?string('true', 'false'))}";
            let sendFailed = "${((deliveryState!'') == 'FAILED')?string('true', 'false')}";
            <#noparse>
            // A code that was not sent does not make the voter wait to ask again.
            if (sendFailed === "true") {
                localStorage.setItem('resendOtpEndTime', Date.now());
                localStorage.setItem('resendOtpDisabled', false);
            }
            function resendOtp(resendTimerTimeout) {
                let form = document.getElementById('kc-message-otp-form');
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
                let resendAction = document.getElementById('resend-action');
                var endTime = localStorage.getItem('resendOtpEndTime');
                var disabled = localStorage.getItem('resendOtpDisabled') === 'true';
                var now = Date.now();
                if (!endTime || codeJustSent === "true") {
                    endTime = now + resendTimerTimeout * 1000;
                    localStorage.setItem('resendOtpEndTime', endTime);
                    localStorage.setItem('resendOtpDisabled', true);
                    disabled = true;
                }
                let countdown = Math.max(Math.ceil((endTime - now) / 1000), 0);
                if (disabled) {
                    resendAction.innerHTML = `<span class='resend-timer'>${resendTimerI18n.replace('{0}', countdown)}</span>`;
                    let interval = setInterval(() => {
                        if (countdown > 0) {
                            resendAction.innerHTML = `<span class='resend-timer'>${resendTimerI18n.replace('{0}', countdown)}</span>`;
                            countdown--;
                        } else {
                            clearInterval(interval);
                            resendAction.innerHTML = `<button type='button' class='resend-link' onclick='resendOtp(${resendTimerTimeout})'>${resendButtonI18n}</button>`;
                            localStorage.setItem('resendOtpDisabled', false);
                        }
                    }, 1000);
                } else {
                    resendAction.innerHTML = `<button type='button' class='resend-link' onclick='resendOtp(${resendTimerTimeout})'>${resendButtonI18n}</button>`;
                }
            }
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
                    const pasteDataTrim = e.clipboardData.getData('text').trim();
                    const pasteData = pasteDataTrim.substring(0, otpInputs.length);
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
            function handleOtpInput() {
                const form = document.getElementById('kc-message-otp-form');
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
            .alert-error {
                margin-bottom: 2em;
            }
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
            .otp-sent-row {
                display: flex;
                align-items: center;
                justify-content: center;
                gap: 0.5em;
                margin-bottom: 1em;
            }
            .otp-sent-label {
                color: #555;
            }
            .otp-sent-value {
                font-weight: 600;
            }
            .otp-error {
                text-align: center;
            }
            .otp-input-row {
                text-align: center;
            }
            .resend-row {
                display: flex;
                align-items: center;
                justify-content: center;
                margin-top: 1em;
                text-align: center;
            }
            .resend-prefix {
                display: inline;
                width: 100%;
            }
            .resend-timer {
                margin-left: 0.5em;
            }
            .resend-link, .change-link {
                background: none;
                border: none;
                color: #007bff;
                text-decoration: underline;
                cursor: pointer;
                font-weight: 500;
                padding: 0 0.5em;
                display: inline;
            }
            .resend-link:hover, .change-link:hover {
                color: #0056b3;
                text-decoration: none;
            }
            .ttl-row {
                width: 100% !important;
                justify-content: center !important;
                margin-top: 1em;
            }
        </style>
    </#if>
</@layout.registrationLayout>
