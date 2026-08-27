<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#macro registrationLayout bodyClass="" displayInfo=false displayMessage=true displayRequiredFields=false displayCard=true displaySocialProviders=false>
<!DOCTYPE html>
<html class="${properties.kcHtmlClass!}"<#if realm.internationalizationEnabled> lang="${locale.currentLanguageTag}"</#if> translate="no">

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
                "rfc4648": "${url.resourcesCommonPath}/vendor/rfc4648/rfc4648.js"
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
        import { startSessionPolling } from "${url.resourcesPath}/js/authChecker.js";

        startSessionPolling(
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
                            <img alt="" aria-hidden="true" src= "data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJsYW5ndWFnZSIgY2xhc3M9InByZWZpeF9fc3ZnLWlubGluZS0tZmEgcHJlZml4X19mYS1sYW5ndWFnZSBwcmVmaXhfX2ZhLWxnIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA2NDAgNTEyIj48cGF0aCBmaWxsPSJjdXJyZW50Q29sb3IiIGQ9Ik0wIDEyOGMwLTM1LjMgMjguNy02NCA2NC02NGg1MTJjMzUuMyAwIDY0IDI4LjcgNjQgNjR2MjU2YzAgMzUuMy0yOC43IDY0LTY0IDY0SDY0Yy0zNS4zIDAtNjQtMjguNy02NC02NFYxMjh6bTMyMCAwdjI1NmgyNTZWMTI4SDMyMHptLTE0MS43IDQ3LjljLTMuMi03LjItMTAuNC0xMS45LTE4LjMtMTEuOXMtMTUuMSA0LjctMTguMyAxMS45bC02NCAxNDRjLTQuNSAxMC4xLjEgMjEuOSAxMC4yIDI2LjRzMjEuOS0uMSAyNi40LTEwLjJsOC45LTIwLjFoNzMuNmw4LjkgMjAuMWM0LjUgMTAuMSAxNi4zIDE0LjYgMjYuNCAxMC4yczE0LjYtMTYuMyAxMC4yLTI2LjRsLTY0LTE0NHpNMTYwIDIzMy4ybDE5IDQyLjhoLTM4bDE5LTQyLjh6TTQ0OCAxNjRjMTEgMCAyMCA5IDIwIDIwdjRoNjBjMTEgMCAyMCA5IDIwIDIwcy05IDIwLTIwIDIwaC0ybC0xLjYgNC41Yy04LjkgMjQuNC0yMi40IDQ2LjYtMzkuNiA2NS40LjkuNiAxLjggMS4xIDIuNyAxLjZsMTguOSAxMS4zYzkuNSA1LjcgMTIuNSAxOCA2LjkgMjcuNHMtMTggMTIuNS0yNy40IDYuOUw0NjcgMzMzLjhjLTQuNS0yLjctOC44LTUuNS0xMy4xLTguNS0xMC42IDcuNS0yMS45IDE0LTM0IDE5LjRsLTMuNiAxLjZjLTEwLjEgNC41LTIxLjktLjEtMjYuNC0xMC4ycy4xLTIxLjkgMTAuMi0yNi40bDMuNi0xLjZjNi40LTIuOSAxMi42LTYuMSAxOC41LTkuOEw0MTAgMjg2LjFjLTcuOC03LjgtNy44LTIwLjUgMC0yOC4zczIwLjUtNy44IDI4LjMgMGwxNC42IDE0LjYuNS41YzEyLjQtMTMuMSAyMi41LTI4LjMgMjkuOC00NUgzNzZjLTExIDAtMjAtOS0yMC0yMHM5LTIwIDIwLTIwaDUydi00YzAtMTEgOS0yMCAyMC0yMHoiLz48L3N2Zz4="/>
                            <span id="profile-language-current">
                            ${locale.current}
                            </span>
                            <img alt="" aria-hidden="true" src= "data:image/svg+xml;base64,PHN2ZyBhcmlhLWhpZGRlbj0idHJ1ZSIgZGF0YS1wcmVmaXg9ImZhcyIgZGF0YS1pY29uPSJjYXJldC1kb3duIiBjbGFzcz0icHJlZml4X19zdmctaW5saW5lLS1mYSBwcmVmaXhfX2ZhLWNhcmV0LWRvd24gcHJlZml4X19mYS1sZyIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB2aWV3Qm94PSIwIDAgMzIwIDUxMiI+PHBhdGggZmlsbD0iY3VycmVudENvbG9yIiBkPSJNMTM3LjQgMzc0LjZjMTIuNSAxMi41IDMyLjggMTIuNSA0NS4zIDBsMTI4LTEyOGM5LjItOS4yIDExLjktMjIuOSA2LjktMzQuOVMzMDEgMTkxLjkgMjg4IDE5MS45TDMyIDE5MmMtMTIuOSAwLTI0LjYgNy44LTI5LjYgMTkuOHMtMi4yIDI1LjcgNi45IDM0LjlsMTI4IDEyOHoiLz48L3N2Zz4="/>
                            </button>
                            <ul role="menu" tabindex="-1" aria-labelledby="kc-current-locale-link" aria-activedescendant="" id="language-switch1" class="${properties.kcLocaleListClass!}">
                                <#assign i = 1>
                                <#list locale.supported as l>
                                    <li class="${properties.kcLocaleListItemClass!}" role="none">
                                        <a role="menuitem" id="language-${i}" class="${properties.kcLocaleItemClass!}" href="${l.url}" data-lang="${l.languageTag!}">${l.label}</a>
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
                            <h1 id="kc-page-title"><#nested "header"></h1>
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
                    <h1 id="kc-page-title"><#nested "header"></h1>
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


          <#if displaySocialProviders>
           <#nested "socialProviders">
          </#if>

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
        <p>${kcSanitize(msg("loginFooter"))?no_esc}</p>
    </div>
  </main>
<script>
    (function () {
        function getCookie(name) {
            var cookies = document.cookie ? document.cookie.split("; ") : [];
            for (var i = 0; i < cookies.length; i++) {
                var parts = cookies[i].split("=");
                if (parts[0] === name) {
                    return decodeURIComponent(parts.slice(1).join("="));
                }
            }
            return null;
        }

        function isIpAddress(hostname) {
            return /^\d{1,3}(\.\d{1,3}){3}$/.test(hostname) || hostname.indexOf(":") !== -1;
        }

        function buildCookie(name, value, domain, expires) {
            var cookie =
                name + "=" + encodeURIComponent(value) +
                "; Path=/" +
                "; SameSite=Lax";

            if (domain) {
                cookie += "; Domain=" + domain;
            }

            if (expires) {
                cookie += "; Expires=" + expires;
            }

            if (window.location.protocol === "https:") {
                cookie += "; Secure";
            }

            return cookie;
        }

        function clearCookie(name, domain) {
            document.cookie = buildCookie(name, "", domain, "Thu, 01 Jan 1970 00:00:00 GMT");
        }

        function canSetCookieOnDomain(domain) {
            var probeName =
                "__sequent_cookie_domain_probe__" + Math.random().toString(36).slice(2);
            var probeValue = Math.random().toString(36).slice(2);

            document.cookie = buildCookie(probeName, probeValue, domain);
            var accepted = getCookie(probeName) === probeValue;

            clearCookie(probeName, domain);
            clearCookie(probeName, null);

            return accepted;
        }

        function getCookieDomain(hostname) {
            if (!hostname || hostname === "localhost" || hostname.indexOf(".") === -1) {
                return null;
            }

            if (isIpAddress(hostname)) {
                return null;
            }

            var parts = hostname.split(".");
            for (var i = 2; i <= parts.length; i++) {
                var candidate = parts.slice(-i).join(".");
                if (canSetCookieOnDomain(candidate)) {
                    return candidate;
                }
            }

            return null;
        }

        var domain = getCookieDomain(window.location.hostname);

        function toInternalLang(lang) {
            if (!lang) {
                return null;
            }
            var primary = lang.toLowerCase().split("-")[0];
            // Keep this fallback tiny: this template runs outside the app and
            // cannot use the shared sequent-core WASM helpers.
            return primary === "ca" ? "cat" : primary;
        }

        function toKeycloakLocale(lang) {
            var internalLang = toInternalLang(lang);
            if (!internalLang) {
                return null;
            }
            return internalLang === "cat" ? "ca" : internalLang;
        }

        function setSessionLangCookie(lang) {
            if (!lang) {
                return;
            }
            document.cookie = buildCookie("USER_LANGUAGE", lang, domain);
        }

        function getLangFromHref(href) {
            var u = new URL(href, window.location.origin);
            return u.searchParams.get("kc_locale") || u.searchParams.get("locale");
        }

        function getSupportedLocalesWithUrls() {
            return [
                <#list locale.supported as l>
                    {
                        "languageTag": "${l.languageTag}",
                        "url": "${l.url?js_string}"
                    }<#if l_has_next>,</#if>
                </#list>
            ];
        }

        <#if (realm.attributes["language_detection_policy"]!"") == "force-default">
        (function enforceDefaultLocale() {
            var kcLocale = "${realm.defaultLocale!}";
            var internalLocale = "${(realm.attributes["forced_language_code"])!realm.defaultLocale!}";
            var cookieLang = toInternalLang(getCookie("USER_LANGUAGE"));
            var selectedInternalLocale = cookieLang || toInternalLang(internalLocale);
            var selectedKcLocale = toKeycloakLocale(selectedInternalLocale) || kcLocale;
            if (!selectedKcLocale) {
                return;
            }
            var supportedLocales = getSupportedLocalesWithUrls();
            var targetLocale = supportedLocales.find(function (l) { return l.languageTag === selectedKcLocale; });
            if (!targetLocale) {
                var fallbackUrl = new URL(window.location.href);
                fallbackUrl.searchParams.set("kc_locale", selectedKcLocale);
                if (fallbackUrl.toString() !== window.location.href) {
                    window.location.replace(fallbackUrl.toString());
                    return;
                }
                setSessionLangCookie(selectedInternalLocale);
                return;
            }
            if ("${locale.currentLanguageTag!}" !== targetLocale.languageTag) {
                window.location.replace(targetLocale.url);
                return;
            }
            setSessionLangCookie(selectedInternalLocale);
        })();
        </#if>

        document.querySelectorAll("#language-switch1 a[href]").forEach(function (link) {
            link.addEventListener("click", function () {
                var lang = toInternalLang(getLangFromHref(link.href));
                if (lang) {
                    setSessionLangCookie(lang);
                }
            });
        });
    })();
</script>
</body>
</html>
</#macro>
