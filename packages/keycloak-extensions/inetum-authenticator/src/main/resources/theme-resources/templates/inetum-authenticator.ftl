<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false displayCard=false; section>
    <#if section = "head">
    <link href="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/css/dob-styles.css" rel="stylesheet" />
    <script src="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/js/jquery-3.7.1.min.js"></script>
    <style>
        dob-sdk-root {
            /* ICONS */
            --dob-icon-camera-circle: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/camera-circle.svg') no-repeat center;
            --dob-icon-card-back: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/card-back.svg') no-repeat center;
            --dob-icon-card-front: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/card-front.svg') no-repeat center;
            --dob-icon-check-circle: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/check-circle.svg') no-repeat center;
            --dob-icon-check: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/check.svg') no-repeat center;
            --dob-icon-chevron-right: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/chevron-right.svg') no-repeat center;
            --dob-icon-cloud-upload: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/cloud-upload.svg') no-repeat center;
            --dob-icon-exclamation-triangle: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/exclamation-triangle.svg') no-repeat center;
            --dob-icon-info-circle: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/info-circle.svg') no-repeat center;
            --dob-icon-question-circle: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/question-circle.svg') no-repeat center;
            --dob-icon-info: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/info.svg') no-repeat center;
            --dob-icon-passport: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/passport.svg') no-repeat center;
            --dob-icon-replay: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/replay.svg') no-repeat center;
            --dob-icon-sign-in: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/sign-in.svg') no-repeat center;
            --dob-icon-sign-out: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/sign-out.svg') no-repeat center;
            --dob-icon-leave: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/leave.svg') no-repeat center;
            --dob-icon-times: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/times.svg') no-repeat center;
            --dob-icon-user: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/user.svg') no-repeat center;
            --dob-icon-certificate: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/verified.svg') no-repeat center;
            --dob-icon-video: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/video.svg') no-repeat center;
            --dob-icon-recording: url('${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/images/icons/recording.svg') no-repeat center;
        }
    </style>
    <#outputformat "plainText">
        <div id="app" data-locale="${msg('locale')}"></div>
        <script>
            window.DOB_API_KEY = "${api_key}";
            window.DOB_APP_ID = "${app_id}";
            window.DOB_CLIENT_ID = "${client_id}";
            window.DOB_DOC_ID = '${doc_id ! ""}';
            window.DOB_DOC_ID_TYPE = "${doc_id_type}";
            window.DOB_DATA = {
                uid: "${user_id}",
                td: "${token_dob}"
            };
            window.DOB_ENV_CONFIG = `${env_config}`;
            window.ASSETS_URL = "${url.resourcesPath}/inetum-sdk-${sdk_version}/"
            window.KEYCLOAK_LOGIN_ACTION_URL = "${url.loginAction}";
            window.LOCALE = document.getElementById("app").dataset.locale?? "en";
        </script>
    </#outputformat>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/js/dob-models-1.1.20.esm.js"></script>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/js/dob-sdk-${sdk_version}.js"></script>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/js/main.js"></script>
    <#elseif section = "body">
    <div class="dob-sdk-container dob-sdk-root-container">
        <div class="dob-attach-messages">
            <h2 id="info_title"></h2>
            <h3 id="info_description"></h3>
        </div>
        <dob-sdk-root id="dob-sdk"></dob-sdk-root>
    </div>
    <form id="kc-inetum-success-form" class="${properties.kcFormClass!}" action="${url.registrationAction}" method="post"></form>
    </#if>
</@layout.registrationLayout>