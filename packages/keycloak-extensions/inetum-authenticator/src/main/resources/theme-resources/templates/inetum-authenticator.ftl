<#import "template.ftl" as layout>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">
<head>
    <title>Inetum Authentication</title>
    <meta charset="UTF-8"/>
    <meta http-equiv="X-UA-Compatible" content="IE=edge"/>
    <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
    <link href="${url.resourcesPath}/inetum-sdk-3.9.2/assets/css/dob-styles.css" rel="stylesheet" />
    <link href="${url.resourcesPath}/inetum-sdk-3.9.2/assets/css/dob-colors.css" rel="stylesheet" />
    <script src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/jquery-3.7.1.min.js"></script>
    <#outputformat "plainText">
        <script>
            window.DOB_API_KEY = "${api_key}";
            window.DOB_APP_ID = "${app_id}";
            window.DOB_CLIENT_ID = "${client_id}";
            window.DOB_DATA = {
                uid: "${user_id}",
                td: "${token_dob}"
            };
        </script>
    </#outputformat>
</head>
<body>
    <header class="inetum-header color-header-bg">
        <img src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/images/inetum-ico.png" alt="Inetum"/>
    </header>
    <div class="dob-container dob-container-bg">
        <div class="dob-attach-messages">
            <h2 id="info_title"></h2>
            <h3 id="info_description"></h3>
        </div>
        <dob-sdk-root id="dob-sdk"></dob-sdk-root>
    </div>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/dob-models-1.1.19.esm.js"></script>
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/dob-sdk-3.9.2.js"></script>
    <script>
        <#include "resources/inetum-sdk-3.9.2/assets/js/example-esm.js">
    </script>
</body>
</html>
