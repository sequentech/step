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
            window.DOB_API_KEY = "fd2280f6-4fb8-3ba3-b55f-5a425b64f07f";
            window.DOB_APP_ID = "7349cfcc-b3a1-4dd7-b399-d135be0a2f43";
            window.DOB_CLIENT_ID = "demosequentech";
            // AJAX call to the backend, this should ideally be handled by the server-side
            $.ajax({
                url: 'https://des.digitalonboarding.es/dob-api/transaction/new',
                type: 'post',
                data: JSON.stringify({
                    "wFtype_Facial": true,
                    "wFtype_OCR": true,
                    "wFtype_Video": true,
                    "wFtype_Anti_Spoofing": false,
                    "wFtype_Sign": false,
                    "wFtype_VerifAvan": false,
                    "wFtype_UECertificate": false,
                    "docID": "",
                    "name": "",
                    "lastname1": "",
                    "lastname2": "",
                    "country": "",
                    "mobilePhone": "",
                    "eMail": "",
                    "priority": 3,
                    "maxRetries": 3,
                    "maxProcessTime": 30,
                    "application": "OnBoardingWebDemo",
                    "clienteID": window.DOB_CLIENT_ID
                }),
                headers: {
                    "Content-Type": "application/json",
                    //For object property name, use quoted notation shown in second
                    "Authorization": "Bearer " + window.DOB_API_KEY
                },
                dataType: 'json',
                async: false,
                success: function(data) {
                    // Handling the success response
                    console.info("userID = " + data.response.userID);
                    console.info("tokenDob = " + data.response.tokenDob);
                    window.DOB_DATA = {
                        uid: data.response.userID,
                        td: data.response.tokenDob
                    };
                }
            });
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
    <script type="module" src="${url.resourcesPath}/inetum-sdk-3.9.2/assets/js/example-esm.js"></script>
</body>
</html>
