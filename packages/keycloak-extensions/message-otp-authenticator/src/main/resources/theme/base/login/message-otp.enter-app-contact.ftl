<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=false; section>
    <#if section = "header">
        ${msg(i18nPrefix + ".auth.enterContactTitle")}
    <#elseif section = "form">
        <form
          id="kc-message-otp-app-contact-form"
          class="${properties.kcFormClass!}"
          action="${url.loginAction}"
          method="post"
        >
            <fieldset class="${properties.kcFormGroupClass!}">
                <legend class="${properties.kcLabelClass!}">${msg("messaging.channelChoice.label")}</legend>
                <#list messagingChannels as option>
                    <div>
                        <input
                          type="radio"
                          id="app-channel-${option}"
                          name="channel"
                          value="${option}"
                          required
                          <#if option?index == 0>checked</#if>
                        />
                        <label for="app-channel-${option}">${msg("messageChannel." + option)}</label>
                    </div>
                </#list>
            </fieldset>
            <#list messagingChannels as option>
                <div class="app-channel-fields ${properties.kcFormGroupClass!}" data-channel="${option}">
                    <#if option == "MESSENGER">
                        <#if messagingPage??>
                            <p>${msg("messageOtp.messenger.intro", messagingPage)}</p>
                        </#if>
                    <#else>
                        <label for="app-contact-${option}" class="${properties.kcLabelClass!}">
                            ${msg("messaging.number." + option)}
                        </label>
                        <input
                          id="app-contact-${option}"
                          name="contact"
                          type="tel"
                          inputmode="tel"
                          autocomplete="tel"
                          class="${properties.kcInputClass!}"
                          aria-describedby="app-contact-help-${option}"
                          required
                        />
                        <div id="app-contact-help-${option}" class="${properties.kcInputHelperTextAfterClass!}">
                            ${msg("messaging.number.help", messagingOrganization)}
                        </div>
                    </#if>
                    <div>
                        <input
                          type="checkbox"
                          id="app-consent-${option}"
                          name="sequent.message-consent"
                          value="${messagingConsentVersion}:${option}"
                          required
                        />
                        <label for="app-consent-${option}">
                            ${msg("messaging.consent", messagingOrganization, msg("messageChannel." + option))}
                        </label>
                    </div>
                    <div>
                        <input
                          type="checkbox"
                          id="app-notice-${option}"
                          name="sequent.notice-channel"
                          value="${option}"
                        />
                        <label for="app-notice-${option}">
                            ${msg("messaging.noticeChannel", msg("messageChannel." + option))}
                        </label>
                    </div>
                </div>
            </#list>
            <div class="${properties.kcFormGroupClass!}">
                <button
                  id="kc-form-submit"
                  class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit"
                >
                  ${msg(i18nPrefix + ".auth.sendCodeButton")}
                </button>
            </div>
        </form>
        <script>
            <#noparse>
            (function () {
                var form = document.getElementById("kc-message-otp-app-contact-form");
                var groups = form.querySelectorAll(".app-channel-fields");
                // Only the chosen app's fields are shown and submitted.
                function show() {
                    var chosen = form.querySelector("input[name='channel']:checked");
                    groups.forEach(function (group) {
                        var active = chosen !== null && group.dataset.channel === chosen.value;
                        group.hidden = !active;
                        group.querySelectorAll("input").forEach(function (input) {
                            input.disabled = !active;
                        });
                    });
                }
                form.querySelectorAll("input[name='channel']").forEach(function (radio) {
                    radio.addEventListener("change", show);
                });
                show();
            })();
            </#noparse>
        </script>
    </#if>
</@layout.registrationLayout>
