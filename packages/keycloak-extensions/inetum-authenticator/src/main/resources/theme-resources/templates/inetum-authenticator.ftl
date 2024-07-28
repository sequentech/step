<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false displayCard=false; section>
    <#if section = "head">
    <link href="${url.resourcesPath}/inetum-sdk-3.9.2/assets/css/dob-styles.css" rel="stylesheet" />
    <link href="${url.resourcesPath}/inetum-sdk-3.9.2/assets/css/dob-colors.css" rel="stylesheet" />
    <script src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/jquery-3.7.1.min.js"></script>
    <#outputformat "plainText">
        <script>
            window.DOB_API_KEY = "${api_key}";
            window.DOB_APP_ID = "${app_id}";
            window.DOB_CLIENT_ID = "${client_id}";
            window.DOB_DOC_ID = "${doc_id}";
            window.DOB_DOC_ID_TYPE = "${doc_id_type}";
            window.DOB_DATA = {
                uid: "${user_id}",
                td: "${token_dob}"
            };
            window.DOB_ENV_CONFIG = `${env_config}`;
            window.KEYCLOAK_LOGIN_ACTION_URL = "${url.loginAction}";
        </script>
    </#outputformat>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/dob-models-1.1.19.esm.js"></script>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/dob-sdk-3.9.2.js"></script>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/main.js"></script>
    <#elseif section = "body">
    <div class="dob-container dob-container-bg">
        <div class="dob-attach-messages">
            <h2 id="info_title"></h2>
            <h3 id="info_description"></h3>
        </div>
        <dob-sdk-root id="dob-sdk"></dob-sdk-root>
    </div>
    <form id="kc-inetum-success-form" class="${properties.kcFormClass!}" action="${url.registrationAction}" method="post"></form>
    </#if>
</@layout.registrationLayout>