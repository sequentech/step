<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  "Get codes and updates by": the channels the selected Post offers, in the ticket's order
      (WhatsApp, Viber, Messenger, SMS, Email). One channel per voter. deferred-registration-user-
      creation sets messagingChannels (the submitted Post's), messagingChannelsByPost (each Post's,
      as messagingChannelsByPostJson for the script), messagingPostAttribute (the Post field) and
      messagingAllChannels; the script shows only the selected Post's channels and refreshes them
      when the Post changes. Each channel's number and consent show while it is selected; the
      consent names the organization and the channel, and every channel needs it. The server checks
      the submitted Post and channel again.  -->

<#macro render>
    <#if messagingChannels?? && messagingChannels?has_content>
        <#assign submittedChannel = (register.formData['sequent.otp-channel'])!''>
        <#if !submittedChannel?has_content && messagingChannels?size == 1>
            <#assign submittedChannel = messagingChannels?first>
        </#if>
        <fieldset id="messaging-channel-choice" class="${properties.kcFormGroupClass!}" data-messaging-channel-choice
            data-post-attribute="${messagingPostAttribute!''}"
            data-channels-by-post="${messagingChannelsByPostJson!'{}'}"
            <#if messagesPerField.existsError('sequent.otp-channel')>aria-invalid="true" aria-describedby="input-error-otp-channel"</#if>>
            <legend class="${properties.kcLabelClass!}">${msg("messaging.channelChoice.label")} *</legend>
            <p id="messaging-post-help" class="messaging-post-help" data-post-help-template="${msg("messaging.channelChoice.help", "__post__")}" hidden></p>
            <#list (messagingAllChannels!messagingChannels) as channel>
                <div class="messaging-channel-option" data-messaging-option="${channel}"
                    <#if !messagingChannels?seq_contains(channel)>hidden</#if>>
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

        <#list (messagingAllChannels!messagingChannels) as channel>
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
                    <p class="messaging-messenger-help"><strong>${msg("messageOtp.messenger.connect")}</strong> · ${msg("messageOtp.messenger.connectHelp", messagingPage)}</p>
                </#if>
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
            </div>
        </#list>
        <script type="module" src="${url.resourcesPath}/js/messaging-channel-choice.js"></script>
    </#if>
</#macro>
