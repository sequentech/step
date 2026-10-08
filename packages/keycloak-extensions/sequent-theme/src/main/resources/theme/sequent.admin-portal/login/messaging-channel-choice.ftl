<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  "Get your verification code by": the channels the voter's Post offers for codes
      (messagingChannels, set by deferred-registration-user-creation). Each channel's number,
      consent and notice choice only show while that channel is selected; the consent and the
      notice choice submit the channel they name, so ticking them for one channel never applies
      to another.  -->

<#macro render>
    <#if messagingChannels?? && messagingChannels?has_content>
        <#assign submittedChannel = (register.formData['sequent.otp-channel'])!''>
        <#if !submittedChannel?has_content && messagingChannels?size == 1>
            <#assign submittedChannel = messagingChannels?first>
        </#if>
        <fieldset id="messaging-channel-choice" class="${properties.kcFormGroupClass!}" data-messaging-channel-choice
            <#if messagesPerField.existsError('sequent.otp-channel')>aria-invalid="true" aria-describedby="input-error-otp-channel"</#if>>
            <legend class="${properties.kcLabelClass!}">${msg("messaging.channelChoice.label")} *</legend>
            <#list messagingChannels as channel>
                <div class="messaging-channel-option">
                    <input type="radio" id="otp-channel-${channel}" name="sequent.otp-channel" value="${channel}"
                        <#if submittedChannel == channel>checked</#if>/>
                    <label for="otp-channel-${channel}">${msg("messageChannel." + channel)}</label>
                </div>
            </#list>
            <#if messagesPerField.existsError('sequent.otp-channel')>
                <span id="input-error-otp-channel" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                    ${kcSanitize(messagesPerField.get('sequent.otp-channel'))?no_esc}
                </span>
            </#if>
        </fieldset>

        <#list messagingChannels as channel>
            <div class="${properties.kcFormGroupClass!} messaging-channel-details" data-messaging-for="${channel}"
                <#if submittedChannel != channel>hidden</#if>>
                <#if channel == 'WHATSAPP' || channel == 'VIBER'>
                    <#assign numberField = 'sequent.' + channel?lower_case + '-number'>
                    <div class="${properties.kcLabelWrapperClass!}">
                        <label for="${numberField}" class="${properties.kcLabelClass!}">${msg("messaging.number." + channel)}</label> *
                    </div>
                    <div class="${properties.kcInputWrapperClass!}">
                        <input type="tel" id="${numberField}" name="${numberField}" class="${properties.kcInputClass!}"
                            autocomplete="tel" value="${(register.formData[numberField])!''}"
                            aria-describedby="${numberField}-help"
                            <#if messagesPerField.existsError(numberField)>aria-invalid="true"</#if>/>
                        <div id="${numberField}-help" class="messaging-number-help">${msg("messaging.number.help", messagingOrganization)}</div>
                        <#if messagesPerField.existsError(numberField)>
                            <span class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                ${kcSanitize(messagesPerField.get(numberField))?no_esc}
                            </span>
                        </#if>
                    </div>
                </#if>
                <#if channel == 'MESSENGER' && messagingPage??>
                    <p class="messaging-messenger-help">${msg("messageOtp.messenger.connectHelp", messagingPage)}</p>
                </#if>
                <#if channel == 'WHATSAPP' || channel == 'VIBER' || channel == 'MESSENGER'>
                    <div class="${properties.kcInputWrapperClass!} messaging-consent">
                        <input type="checkbox" id="message-consent-${channel}" name="sequent.message-consent"
                            value="${messagingConsentVersion}:${channel}"
                            <#if ((register.formData['sequent.message-consent'])!'') == messagingConsentVersion + ':' + channel>checked</#if>
                            <#if messagesPerField.existsError('sequent.message-consent')>aria-invalid="true"</#if>/>
                        <label for="message-consent-${channel}">${msg("messaging.consent", messagingOrganization, msg("messageChannel." + channel))}</label>
                        <#if messagesPerField.existsError('sequent.message-consent')>
                            <span class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                ${msg("messaging.consent.required", msg("messageChannel." + channel))}
                            </span>
                        </#if>
                    </div>
                </#if>
                <div class="${properties.kcInputWrapperClass!} messaging-notice-channel">
                    <input type="checkbox" id="notice-channel-${channel}" name="sequent.notice-channel" value="${channel}"
                        <#if ((register.formData['sequent.notice-channel'])!'') == channel>checked</#if>/>
                    <label for="notice-channel-${channel}">${msg("messaging.noticeChannel", msg("messageChannel." + channel))}</label>
                </div>
            </div>
        </#list>
        <script type="module" src="${url.resourcesPath}/js/messaging-channel-choice.js"></script>
    </#if>
</#macro>
