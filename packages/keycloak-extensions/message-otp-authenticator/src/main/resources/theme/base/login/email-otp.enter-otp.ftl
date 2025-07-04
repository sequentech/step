<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header">
        <h2>${msg("emailOtp.auth.enterOtpTitle")}</h2>
    <#elseif section = "form">
        <form id="kc-email-otp-form" method="post">
            <div>${msg("emailOtp.auth.otpSentTo")}: <b>${email}</b></div>
            <#if error??>
                <div class="error">${error}</div>
            </#if>
            <div class="otp-container" id="otp-inputs">
                <#assign otpLength = codeLength?number>
                <#list 1..otpLength as i>
                    <input autocomplete="off" type="text" inputmode="numeric" pattern="\\d" id="otp-${i}" name="otp${i}" maxlength="1" class="otp-input" <#if i == 1> autofocus="autofocus" </#if> />
                </#list>
            </div>
            <input type="hidden" id="code" name="code" />
            <div class="form-actions">
                <button id="kc-form-submit" type="submit" class="btn btn-primary" onclick="handleOtpInput()">${msg("emailOtp.auth.verifyButton")}</button>
            </div>
        </form>
        <form method="post" style="margin-top:1em;">
            <input type="hidden" name="changeEmail" value="true" />
            <button type="submit">${msg("emailOtp.auth.changeEmailButton")}</button>
        </form>
        <div class="form-actions" style="margin-top:1em;">
            <button id="resend-otp-btn" type="button" name="resend" value="true" class="btn btn-secondary" onclick="resendOtp(${(resendTimer)})"></button>
        </div>
        <script>
            let resendTimerI18n = "${msg("emailOtp.auth.resend.timer")}";
            let resendButtonI18n = "${msg("emailOtp.auth.resend.button")}";
            let resendTimerTimeout = ${(resendTimer)};
            let codeJustSent = "${(codeJustSent?string('true', 'false'))}";
            <#noparse>
            function resendOtp(resendTimerTimeout) {
                let resendBtn = document.getElementById('resend-otp-btn');
                let form = document.getElementById('kc-email-otp-form');
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
            function updateButtonState() {
                let resendBtn = document.getElementById('resend-otp-btn');
                var endTime = localStorage.getItem('resendOtpEndTime');
                var disabled = localStorage.getItem('resendOtpDisabled') === 'true';
                var now = Date.now();
                if (codeJustSent === "true") {
                    endTime = now + resendTimerTimeout * 1000;
                    localStorage.setItem('resendOtpEndTime', endTime);
                    localStorage.setItem('resendOtpDisabled', true);
                }
                let countdown = Math.max(Math.ceil((endTime - now) / 1000), 0);
                if (disabled) {
                    resendBtn.disabled = true;
                    let interval = setInterval(() => {
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
                    resendBtn.disabled = false;
                    resendBtn.innerText = resendButtonI18n;
                }
            }
            function handleOtpInput() {
                const form = document.getElementById('kc-email-otp-form');
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
    </#if>
</@layout.registrationLayout>
