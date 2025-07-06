<#--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<#include "intl-tel-input.ftl">
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header">
        <h2 style="text-align:center;">
            ${msg(i18nPrefix + ".auth.enterContactTitle")}
        </h2>
    <#elseif section = "form">
        <form
          id="kc-message-otp-contact-form"
          class="${properties.kcFormClass!}"
          action="${url.loginAction}"
          method="post"
        >
            <div class="${properties.kcFormGroupClass!}">
                <#if i18nPrefix == "emailOtp">
                  <div class="${properties.kcLabelWrapperClass!}">
                    <label
                      for="email"
                      class="${properties.kcLabelClass!}"
                    >
                      ${msg("emailOtp.auth.enterEmailLabel")}
                    </label>
                  </div>
                  <div class="${properties.kcInputWrapperClass!}">
                    <input
                      id="email"
                      name="contact"
                      type="email"
                      class="${properties.kcInputClass!}"
                      value="${contact!}"
                      required
                      autofocus
                    />
                    <div
                      class="help-message ${properties.kcInputHelperTextAfterClass!}"
                      style="margin: 8px 0 16px 0; color: #555;"
                    >
                      ${msg("emailOtp.auth.enterEmailHelp")}
                    </div>
                </div>
                <#elseif i18nPrefix == "mobileOtp">
                    <div class="${properties.kcLabelWrapperClass!}">
                        <label
                          for="contact"
                          class="${properties.kcLabelClass!}"
                        >
                          ${msg("mobileOtp.auth.enterMobileLabel")}
                        </label>
                    </div>
                    <div class="${properties.kcInputWrapperClass!}">
                      <@renderIntlTelInput id="contact" name="mobile-num" value=contact />
                      <div
                        class="help-message ${properties.kcInputHelperTextAfterClass!}"
                        style="margin: 8px 0 16px 0; color: #555;"
                      >
                        ${msg("mobileOtp.auth.enterMobileHelp")}
                      </div>
                    </div>
                <#else>
                    <#-- Fallback for contact info, if neither email nor mobile is provided -->
                </#if>
            </div>
            <#if error??>
                <div class="error ${properties.kcFormGroupClass!}">
                  ${msg(i18nPrefix + ".auth." + error)}
                </div>
            </#if>
            <div class="${properties.kcFormGroupClass!}">
                <button
                  id="kc-form-submit"
                  class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit"
                >
                  ${msg(i18nPrefix + ".auth.sendCodeButton")}
                </button>
            </div>
        </form>
    </#if>
</@layout.registrationLayout>
