<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#-- Template Layout for Keycloak Account Pages -->

<#macro accountLayout bodyClass="" displayInfo=false displayMessage=true displayRequiredFields=false>
<!DOCTYPE html>
<html class="${properties.kcHtmlClass!}"<#if realm.internationalizationEnabled> lang="${locale.currentLanguageTag}"</#if>>
<head>
    <meta charset="utf-8">
    <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
    <meta name="robots" content="noindex, nofollow">

    <#if properties.meta?has_content>
        <#list properties.meta?split(' ') as meta>
            <meta name="${meta?split('==')[0]}" content="${meta?split('==')[1]}"/>
        </#list>
    </#if>
    <title>${msg("accountPageTitle",(realm.displayName!''))}</title>
    <link rel="icon" href="${url.resourcesPath}/img/favicon.ico" />

    <style id="account-custom-css" type="text/css">
        <#outputformat "plainText">
            ${msg("accountCustomCss")}
        </#outputformat>
    </style>

    <#if properties.stylesCommon?has_content>
        <#list properties.stylesCommon?split(' ') as style>
            <link href="${url.resourcesCommonPath}/${style}" rel="stylesheet" />
        </#list>
    </#if>
    <#if properties.styles?has_content>
        <#list properties.styles?split(' ') as style>
            <link href="${url.resourcesPath}/${style}" rel="stylesheet" />
        </#list>
    </#if>
    <#if properties.scripts?has_content>
        <#list properties.scripts?split(' ') as script>    
            <script src="${url.resourcesPath}/${script}" type="text/javascript"></script>
        </#list>
    </#if>
    <script type="importmap">
            {
                "imports": {
                    "rfc4648": "${url.resourcesCommonPath}/node_modules/rfc4648/lib/rfc4648.js"
                }
            }
    </script>
    <script src="${url.resourcesPath}/js/menu-button-links.js" type="module"></script>
    <#if scripts??>
        <#list scripts as script>
            <script src="${script}" type="text/javascript"></script>
        </#list>
    </#if>
    <script type="module">
        import { checkCookiesAndSetTimer } from "${url.resourcesPath}/js/authChecker.js";

        checkCookiesAndSetTimer(
            "${url.ssoLoginInOtherTabsUrl?no_esc}"
        );
    </script>
</head>
<body class="${properties.kcBodyClass!} ${bodyClass}">
        <div class="${properties.kcLoginClass!}">
            <div id="kc-header" class="${properties.kcHeaderClass!}">
                <div id="kc-header-wrapper" class="${properties.kcHeaderWrapperClass!}">
                    <#if realm.internationalizationEnabled && locale.supported?size gt 1>
                        <div class="${properties.kcLocaleMainClass!}" id="kc-locale">
                            <div id="kc-locale-wrapper" class="${properties.kcLocaleWrapperClass!}">
                                <div id="kc-locale-dropdown" class="menu-button-links ${properties.kcLocaleDropDownClass!}">
                                    <button tabindex="1" id="kc-current-locale-link" aria-label="${msg("languages")}" aria-haspopup="true" aria-expanded="false" aria-controls="language-switch1">
                                        <img src="data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJsYW5ndWFnZSIgY2xhc3M9InByZWZpeF9fc3ZnLWlubGluZS0tZmEgcHJlZml4X19mYS1sYW5ndWFnZSBwcmVmaXhfX2ZhLWxnIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA2NDAgNTEyIj48cGF0aCBmaWxsPSJjdXJyZW50Q29sb3IiIGQ9Ik0wIDEyOGMwLTM1LjMgMjguNy02NCA2NC02NGg1MTJjMzUuMyAwIDY0IDI4LjcgNjQgNjR2MjU2YzAgMzUuMy0yOC43IDY0LTY0IDY0SDY0Yy0zNS4zIDAtNjQtMjguNy02NC02NFYxMjh6bTMyMCAwdjI1NmgyNTZWMTI4SDMyMHptLTE0MS43IDQ3LjljLTMuMi03LjItMTAuNC0xMS45LTE4LjMtMTEuOXMtMTUuMSA0LjctMTguMyAxMS45bC02NCAxNDRjLTQuNSAxMC4xLjEgMjEuOSAxMC4yIDI2LjRzMjEuOS0uMSAyNi40LTEwLjJsOC45LTIwLjFoNzMuNmw4LjkgMjAuMWM0LjUgMTAuMSAxNi4zIDE0LjYgMjYuNCAxMC4yczE0LjYtMTYuMyAxMC4yLTI2LjRsLTY0LTE0NHpNMTYwIDIzMy4ybDE5IDQyLjhoLTM4bDE5LTQyLjh6TTQ0OCAxNjRjMTEgMCAyMCA5IDIwIDIwdjRoNjBjMTEgMCAyMCA5IDIwIDIwcy05IDIwLTIwIDIwaC0ybC0xLjYgNC41Yy04LjkgMjQuNC0yMi40IDQ2LjYtMzkuNiA2NS40LjkuNiAxLjggMS4xIDIuNyAxLjZsMTguOSAxMS4zYzkuNSA1LjcgMTIuNSAxOCA2LjkgMjcuNHMtMTggMTIuNS0yNy40IDYuOUw0NjcgMzMzLjhjLTQuNS0yLjctOC44LTUuNS0xMy4xLTguNS0xMC42IDcuNS0yMS45IDE0LTM0IDE5LjRsLTMuNiAxLjZjLTEwLjEgNC41LTIxLjktLjEtMjYuNC0xMC4ycy4xLTIxLjkgMTAuMi0yNi40bDMuNi0xLjZjNi40LTIuOSAxMi42LTYuMSAxOC41LTkuOEw0MTAgMjg2LjFjLTcuOC03LjgtNy44LTIwLjUgMC0yOC4zczIwLjUtNy44IDI4LjMgMGwxNC42IDE0LjYuNS41YzEyLjQtMTMuMSAyMi41LTI4LjMgMjkuOC00NUgzNzZjLTExIDAtMjAtOS0yMC0yMHM5LTIwIDIwLTIwaDUydi00YzAtMTEgOS0yMCAyMC0yMHoiLz48L3N2Zz4="/>
                                        <span>${locale.current}</span>
                                        <img src="data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJjYXJldC1kb3duIiBjbGFzcz0icHJlZml4X19zdmctaW5saW5lLS1mYSBwcmVmaXhfX2ZhLWNhcmV0LWRvd24gcHJlZml4X19mYS1sZyIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB2aWV3Qm94PSIwIDAgMzIwIDUxMiI+PHBhdGggZmlsbD0iY3VycmVudENvbG9yIiBkPSJNMTM3LjQgMzc0LjZjMTIuNSAxMi41IDMyLjggMTIuNSA0NS4zIDBsMTI4LTEyOGM5LjItOS4yIDExLjktMjIuOSA2LjktMzQuOVMzMDEgMTkxLjkgMjg4IDE5MS45TDMyIDE5MmMtMTIuOSAwLTI0LjYgNy44LTI5LjYgMTkuOHMtMi4yIDI1LjcgNi45IDM0LjlsMTI4IDEyOHoiLz48L3N2Zz4="/>
                                    </button>
                                    <ul role="menu" tabindex="-1" aria-labelledby="kc-current-locale-link" aria-activedescendant="" id="language-switch1">
                                        <#list locale.supported as l>
                                            <li role="none" id="kc-locale-${l.languageTag}" class="${properties.kcLocaleClass!}">
                                                <a role="menuitem" href="${url.changeLocaleUrl}?locale=${l.languageTag}">
                                                    ${l.languageTag}
                                                </a>
                                            </li>
                                        </#list>
                                    </ul>
                                </div>
                            </div>
                        </div>
                    </#if>
                    <div class="container">
                        <div id="kc-logo">
                            <a href="${url.loginUrl}">
                                <img id="kc-logo" src="${url.resourcesCommonPath}/img/keycloak-logo.svg" alt="Keycloak"/>
                            </a>
                        </div>
                    </div>
                </div>
            </div>
            <div class="${properties.kcMainWrapperClass!}">
                <div id="kc-main-content" class="${properties.kcContentClass!}">
                    <div class="${properties.kcLoginMainClass!}">
                        <div id="kc-page-title" class="${properties.kcPageTitleClass!}">
                            <#if displayInfo>
                                <h1>${msg("accountPageTitle")}</h1>
                            </#if>
                        </div>
                        <#if displayMessage>
                            <#include "/${properties.kcMessagesPath}" />
                        </#if>
                        <#if displayRequiredFields>
                            <div class="alert alert-info" id="kc-required-action-message">
                                <strong>${msg("requiredAction")}</strong>
                            </div>
                        </#if>
                        <div id="kc-form-wrapper" class="${properties.kcFormWrapperClass!}">
                            ${body}
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="${properties.kcFooterClass!}">
            <div id="kc-footer">
                <#if properties.socialProviders! && properties.socialProviders?size gt 0>
                    <div id="kc-social-providers">
                        <ul>
                            <#list properties.socialProviders as provider>
                                <li class="kc-social-provider">
                                    <a href="${url.baseUri}/realms/${realm.name}/login-actions/authenticate?client_id=${client.clientId}&tab_id=${tabId}&auth_session_id=${authSessionId}&execution=${provider.alias}">
                                        <img src="${provider.image}" alt="${provider.displayName}"/>
                                    </a>
                                </li>
                            </#list>
                        </ul>
                    </div>
                </#if>
            </div>
        </div>
</body>
</html>
</#macro>
