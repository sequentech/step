<#--
    SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header" || section = "show-username">
        ${msg("registerFinishManualTitle")?no_esc}
    <#elseif section = "form">
        <div class="last-page-text">
            <#if rejectReason??>
                <p>${msg(rejectReason)}</p>
            </#if>
            <#-- Without a voter in the registry, no field was compared: there's nothing to list. -->
            <#if mismatchedFields?? && (rejectReason!"") != "NO_VOTER">
                <p>${msg("rejectReasonListItems")?no_esc}</p>
                <ul>
                <#list mismatchedFields?keys as key>
                    <#if mismatchedFields[key]??>
                        <li>${key}: <strong>${mismatchedFields[key]}</strong></li>
                    <#else>
                        <li>${key}: <i>${msg("empty")}</i></li>
                    </#if>
                </#list>
                </ul>
            </#if>
            <p>${msg("registerFinishManualMessage")?no_esc}</p>
            <#--  The election officer's reply-by time in the Post's zone (lookup-and-update-user's
                  reply-by-hours), named by timezones.name.<zone>, else the CLDR long name.  -->
            <#if enrollmentReplyBy??>
                <#assign replyByZoneKey = "timezones.name." + enrollmentReplyBy.zone>
                <#assign replyByZoneName = msg(replyByZoneKey)>
                <#if replyByZoneName == replyByZoneKey>
                    <#assign replyByZoneName = enrollmentReplyBy.zoneName>
                </#if>
                <#assign replyByCombinedKey = enrollmentTimezoneMessageKey!"timezones.voterDateTimeZone">
                <#assign replyByProbe = msg(replyByCombinedKey, "__dateTime__", "__zoneName__")>
                <#if !replyByProbe?contains("__dateTime__") || !replyByProbe?contains("__zoneName__")>
                    <#assign replyByCombinedKey = "timezones.defaultVoterDateTimeZone">
                    <script>console.warn("Invalid timezone text: using the enrollment default.");</script>
                </#if>
                <p id="enrollment-reply-by" class="enrollment-reply-by">${msg("enrollment.replyBy", msg(replyByCombinedKey, enrollmentReplyBy.dateTime, replyByZoneName))}</p>
            </#if>
            <#if noticeChannel??>
                <p id="notice-channel">${msg("messageOtp.pending.channel", msg("messageChannel." + noticeChannel))}</p>
            </#if>
            <p id="instruction1" class="instruction">
                ${msg("pageExpiredMsg2")} <a id="loginContinueLink" href="${url.loginRestartFlowUrl}">${msg("doClickHere")}</a> .
            </p>
        </div>
    </#if>
</@layout.registrationLayout>