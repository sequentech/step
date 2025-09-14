<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#macro registrationLayout bodyClass="" displayInfo=false displayMessage=true displayRequiredFields=false displayCard=true>
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
    <title>${msg("loginTitle",(realm.displayName!''))}</title>
    <link rel="icon" href="${url.resourcesPath}/img/favicon.ico" />

    <#nested "head">

    <style id="login-custom-css" type="text/css">
        <#outputformat "plainText">
            ${msg("loginCustomCss")}
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

<body class="${properties.kcBodyClass!}">
<main class="${properties.kcLoginClass!}">
    <div id="kc-header" class="${properties.kcHeaderClass!}">
        <div id="kc-header-wrapper"
             class="${properties.kcHeaderWrapperClass!}">
            <div class="logo"></div>
            <div class="version version-version">
                <span class="title">
                    ${msg("system.version")}
                </span>
                <span class="value">${properties.systemVersion}</span>
            </div>
            <div class="version version-hash">
                <span class="title">
                    ${msg("system.hash")}
                </span>
                <span class="value">${properties.systemHash}</span>
            </div>
            <#if realm.internationalizationEnabled  && locale.supported?size gt 1>
                <div class="${properties.kcLocaleMainClass!}" id="kc-locale">
                    <div id="kc-locale-wrapper" class="${properties.kcLocaleWrapperClass!}">
                        <div id="kc-locale-dropdown" class="menu-button-links ${properties.kcLocaleDropDownClass!}">
                            <button tabindex="1" id="kc-current-locale-link" aria-labelledby="profile-language-current" aria-haspopup="true" aria-expanded="false" aria-controls="language-switch1">
                            <img src= "data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJsYW5ndWFnZSIgY2xhc3M9InByZWZpeF9fc3ZnLWlubGluZS0tZmEgcHJlZml4X19mYS1sYW5ndWFnZSBwcmVmaXhfX2ZhLWxnIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA2NDAgNTEyIj48cGF0aCBmaWxsPSJjdXJyZW50Q29sb3IiIGQ9Ik0wIDEyOGMwLTM1LjMgMjguNy02NCA2NC02NGg1MTJjMzUuMyAwIDY0IDI4LjcgNjQgNjR2MjU2YzAgMzUuMy0yOC43IDY0LTY0IDY0SDY0Yy0zNS4zIDAtNjQtMjguNy02NC02NFYxMjh6bTMyMCAwdjI1NmgyNTZWMTI4SDMyMHptLTE0MS43IDQ3LjljLTMuMi03LjItMTAuNC0xMS45LTE4LjMtMTEuOXMtMTUuMSA0LjctMTguMyAxMS45bC02NCAxNDRjLTQuNSAxMC4xLjEgMjEuOSAxMC4yIDI2LjRzMjEuOS0uMSAyNi40LTEwLjJsOC45LTIwLjFoNzMuNmw4LjkgMjAuMWM0LjUgMTAuMSAxNi4zIDE0LjYgMjYuNCAxMC4yczE0LjYtMTYuMyAxMC4yLTI2LjRsLTY0LTE0NHpNMTYwIDIzMy4ybDE5IDQyLjhoLTM4bDE5LTQyLjh6TTQ0OCAxNjRjMTEgMCAyMCA5IDIwIDIwdjRoNjBjMTEgMCAyMCA5IDIwIDIwcy05IDIwLTIwIDIwaC0ybC0xLjYgNC41Yy04LjkgMjQuNC0yMi40IDQ2LjYtMzkuNiA2NS40LjkuNiAxLjggMS4xIDIuNyAxLjZsMTguOSAxMS4zYzkuNSA1LjcgMTIuNSAxOCA2LjkgMjcuNHMtMTggMTIuNS0yNy40IDYuOUw0NjcgMzMzLjhjLTQuNS0yLjctOC44LTUuNS0xMy4xLTguNS0xMC42IDcuNS0yMS45IDE0LTM0IDE5LjRsLTMuNiAxLjZjLTEwLjEgNC41LTIxLjktLjEtMjYuNC0xMC4ycy4xLTIxLjkgMTAuMi0yNi40bDMuNi0xLjZjNi40LTIuOSAxMi42LTYuMSAxOC41LTkuOEw0MTAgMjg2LjFjLTcuOC03LjgtNy44LTIwLjUgMC0yOC4zczIwLjUtNy44IDI4LjMgMGwxNC42IDE0LjYuNS41YzEyLjQtMTMuMSAyMi41LTI4LjMgMjkuOC00NUgzNzZjLTExIDAtMjAtOS0yMC0yMHM5LTIwIDIwLTIwaDUydi00YzAtMTEgOS0yMCAyMC0yMHoiLz48L3N2Zz4="/>
                            <span id="profile-language-current">
                            ${locale.current}
                            </span>
                            <img src= "data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJjYXJldC1kb3duIiBjbGFzcz0icHJlZml4X19zdmctaW5saW5lLS1mYSBwcmVmaXhfX2ZhLWNhcmV0LWRvd24gcHJlZml4X19mYS1sZyIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB2aWV3Qm94PSIwIDAgMzIwIDUxMiI+PHBhdGggZmlsbD0iY3VycmVudENvbG9yIiBkPSJNMTM3LjQgMzc0LjZjMTIuNSAxMi41IDMyLjggMTIuNSA0NS4zIDBsMTI4LTEyOGM5LjItOS4yIDExLjktMjIuOSA2LjktMzQuOVMzMDEgMTkxLjkgMjg4IDE5MS45TDMyIDE5MmMtMTIuOSAwLTI0LjYgNy44LTI5LjYgMTkuOHMtMi4yIDI1LjcgNi45IDM0LjlsMTI4IDEyOHoiLz48L3N2Zz4="/>
                            </button>
                            <ul role="menu" tabindex="-1" aria-labelledby="kc-current-locale-link" aria-activedescendant="" id="language-switch1" class="${properties.kcLocaleListClass!}">
                                <#assign i = 1>
                                <#list locale.supported as l>
                                    <li class="${properties.kcLocaleListItemClass!}" role="none">
                                        <a role="menuitem" id="language-${i}" class="${properties.kcLocaleItemClass!}" href="${l.url}">${l.label}</a>
                                    </li>
                                    <#assign i++>
                                </#list>
                            </ul>
                        </div>
                    </div>
                </div>
            </#if>
        </div>
    </div>
    <div id="kc-title" class="${properties.kcTitleClass!}">
        ${kcSanitize(msg("loginTitleHtml",(realm.displayNameHtml!'')))?no_esc}
    </div>

    <#nested "body">

    <#if displayCard>
    <div class="${properties.kcFormCardClass!}">
        <header class="${properties.kcFormHeaderClass!}">
        <#if !(auth?has_content && auth.showUsername() && !auth.showResetCredentials())>
            <#if displayRequiredFields>
                <div class="${properties.kcContentWrapperClass!}">
                    <div class="${properties.kcLabelWrapperClass!} subtitle">
                        <span class="subtitle"><span class="required">*</span> ${msg("requiredFields")}</span>
                    </div>
                    <div class="col-md-10">
                        <h1 id="kc-page-title"><#nested "header"></h1>
                    </div>
                </div>
            <#else>
                <h1 id="kc-page-title"><#nested "header"></h1>
            </#if>
        <#else>
            <#if displayRequiredFields>
                <div class="${properties.kcContentWrapperClass!}">
                    <div class="${properties.kcLabelWrapperClass!} subtitle">
                        <span class="subtitle"><span class="required">*</span> ${msg("requiredFields")}</span>
                    </div>
                    <div class="col-md-10">
                        <#nested "show-username">
                        <div id="kc-username" class="${properties.kcFormGroupClass!}">
                            <label id="kc-attempted-username">${address!(auth.attemptedUsername)}</label>
                            <a id="reset-login" href="${url.loginRestartFlowUrl}" aria-label="${msg("restartLoginTooltip")}">
                                <div class="kc-login-tooltip">
                                    <i class="${properties.kcResetFlowIcon!}"></i>
                                    <span class="kc-tooltip-text">${msg("restartLoginTooltip")}</span>
                                </div>
                            </a>
                        </div>
                    </div>
                </div>
            <#else>
                <#nested "show-username">
                <div id="kc-username" class="${properties.kcFormGroupClass!}">
                    <label id="kc-attempted-username">${address!(auth.attemptedUsername)}</label>
                    <a id="reset-login" href="${url.loginRestartFlowUrl}" aria-label="${msg("restartLoginTooltip")}">
                        <div class="kc-login-tooltip">
                            <i class="${properties.kcResetFlowIcon!}"></i>
                            <span class="kc-tooltip-text">${msg("restartLoginTooltip")}</span>
                        </div>
                    </a>
                </div>
            </#if>
        </#if>
      </header>
      <div id="kc-content">
        <div id="kc-content-wrapper">

          <#-- App-initiated actions should not see warning messages about the need to complete the action -->
          <#-- during login.                                                                               -->
          <#if displayMessage && message?has_content && (message.type != 'warning' || !isAppInitiatedAction??)>
              <div class="alert-${message.type} ${properties.kcAlertClass!} pf-m-<#if message.type = 'error'>danger<#else>${message.type}</#if>">
                  <div class="pf-c-alert__icon">
                      <#if message.type = 'success'><span class="${properties.kcFeedbackSuccessIcon!}"></span></#if>
                      <#if message.type = 'warning'><span class="${properties.kcFeedbackWarningIcon!}"></span></#if>
                      <#if message.type = 'error'><span class="${properties.kcFeedbackErrorIcon!}"></span></#if>
                      <#if message.type = 'info'><span class="${properties.kcFeedbackInfoIcon!}"></span></#if>
                  </div>
                      <span class="${properties.kcAlertTitleClass!}">${kcSanitize(message.summary)?no_esc}</span>
              </div>
          </#if>

          <#nested "form">

          <#if auth?has_content && auth.showTryAnotherWayLink()>
              <form id="kc-select-try-another-way-form" action="${url.loginAction}" method="post">
                  <div class="${properties.kcFormGroupClass!}">
                      <input type="hidden" name="tryAnotherWay" value="on"/>
                      <a href="#" id="try-another-way"
                         onclick="document.forms['kc-select-try-another-way-form'].submit();return false;">${msg("doTryAnotherWay")}</a>
                  </div>
              </form>
          </#if>

          <#nested "socialProviders">

          <#if displayInfo>
              <div id="kc-info" class="${properties.kcSignUpClass!}">
                  <div id="kc-info-wrapper" class="${properties.kcInfoAreaWrapperClass!}">
                      <#nested "info">
                  </div>
              </div>
          </#if>
        </div>
      </div>


    </div>
    </#if>
    <div class="footer">
        <p>${msg("loginFooter")}</p>
    </div>
  </main>
</body>
</html>
</#macro>
