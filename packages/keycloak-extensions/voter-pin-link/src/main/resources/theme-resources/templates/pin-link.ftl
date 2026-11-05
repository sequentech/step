<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#assign revealing = (pinLinkMode!'form') == 'reveal'>
<@layout.registrationLayout displayMessage=!revealing; section>
    <#if section = "head">
        <#-- The theme puts this before the realm's loginCustomCss, which so wins over these rules. -->
        <style id="pin-link-style">
            <#if revealing>
                <#-- Leaving this page would lose the only copy of the PIN. -->
                #kc-locale { display: none; }
                #pin-link-value { text-align: center; }
                #pin-link-value output { font-size: clamp(1.5rem, 8vw, 2rem); font-weight: 700; letter-spacing: .1em; user-select: all; }
                @media print { #pin-link-print { display: none; } }
            <#else>
                <#-- The CSP blocks the theme's menu script: keyboards and touch screens open it with this. -->
                #kc-locale-dropdown:focus-within ul { display: block; }
            </#if>
        </style>
    <#elseif section = "header">
        ${msg(revealing?then("pinLinkRevealTitle", "pinLinkTitle"))}
    <#elseif section = "form">
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if revealing>
                <p>${msg("pinLinkRevealIntro")}</p>
                <p id="pin-link-value" class="${properties.kcFormGroupClass!}"><output>${pinLinkValue}</output></p>
                <p><strong>${msg("pinLinkRevealWarning")}</strong></p>
                <div class="${properties.kcFormGroupClass!}">
                    <button type="button" id="pin-link-print" class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}">${msg("pinLinkPrint")}</button>
                </div>
                <script nonce="${pinLinkNonce}">
                    document.getElementById("pin-link-print").addEventListener("click", function () { window.print(); });
                </script>
            <#else>
                <p>${msg("pinLinkIntro")}</p>
                <form id="pin-link-form" method="post" autocomplete="off">
                    <div class="${properties.kcFormGroupClass!}">
                        <label for="username" class="${properties.kcLabelClass!}">${msg("pinLinkUsername")}</label>
                        <input id="username" name="username" class="${properties.kcInputClass!}" type="text" autofocus autocomplete="off" required maxlength="255" />
                    </div>
                    <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
                        <input class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit" value="${msg("pinLinkSubmit")}" />
                    </div>
                </form>
            </#if>
          </div>
        </div>
    </#if>
</@layout.registrationLayout>
