<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#-- Shown for an instant while a SAML message is posted on. It sits between an
     external identity provider and the voting portal, so it carries no header,
     logo, version or footer of its own: only a neutral wait indicator, which
     the realm's loginCustomCss can restyle. -->
<!DOCTYPE html>
<html<#if realm.internationalizationEnabled> lang="${locale.currentLanguageTag}"</#if> translate="no">

<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex, nofollow">
    <title>${msg("saml.post-form.title")}</title>
    <style id="saml-post-form-css" type="text/css">
        html, body {
            height: 100%;
            margin: 0;
        }
        body {
            display: flex;
            align-items: center;
            justify-content: center;
            background: #fff;
            color: #767676;
            font-family: system-ui, sans-serif;
            text-align: center;
        }
        #kc-saml-post-form {
            padding: 16px;
        }
        #kc-saml-post-progress {
            width: 40px;
            height: 40px;
            margin: 0 auto;
            box-sizing: border-box;
            border: 4px solid currentColor;
            border-right-color: transparent;
            border-radius: 50%;
            animation: kc-saml-post-spin 1s linear infinite;
        }
        #kc-page-title, #kc-saml-post-message {
            position: absolute;
            width: 1px;
            height: 1px;
            overflow: hidden;
            clip-path: inset(50%);
            white-space: nowrap;
        }
        @keyframes kc-saml-post-spin {
            to {
                transform: rotate(360deg);
            }
        }
        @media (prefers-reduced-motion: reduce) {
            #kc-saml-post-progress {
                animation-duration: 6s;
            }
        }
    </style>
    <style id="login-custom-css" type="text/css">
        <#outputformat "plainText">
            ${msg("loginCustomCss")}
        </#outputformat>
    </style>
</head>

<body class="saml-post-form-page">
<main id="kc-saml-post-form" role="status">
    <div id="kc-saml-post-progress" aria-hidden="true"></div>
    <h1 id="kc-page-title">${msg("saml.post-form.title")}</h1>
    <p id="kc-saml-post-message">${msg("saml.post-form.message")}</p>
    <form name="saml-post-binding" method="post" action="${samlPost.url}">
        <#if samlPost.SAMLRequest??>
            <input type="hidden" name="SAMLRequest" value="${samlPost.SAMLRequest}"/>
        </#if>
        <#if samlPost.SAMLResponse??>
            <input type="hidden" name="SAMLResponse" value="${samlPost.SAMLResponse}"/>
        </#if>
        <#if samlPost.relayState??>
            <input type="hidden" name="RelayState" value="${samlPost.relayState}"/>
        </#if>
        <noscript>
            <p>${msg("saml.post-form.js-disabled")}</p>
            <input type="submit" value="${msg("doContinue")}"/>
        </noscript>
    </form>
</main>
<script>window.onload = function() {document.forms[0].submit()};</script>
</body>
</html>
