<#import "adyen-template.ftl" as layout>
<#import "user-profile-commons.ftl" as userProfileCommons>
<#import "register-commons.ftl" as registerCommons>
<@layout.registrationLayout ; section>
    <#if section = "html-extra-headers">
        <script
            src="https://checkoutshopper-${adyen_environment}.adyen.com/checkoutshopper/sdk/5.60.0/adyen.js"
            integrity="sha384-v6S0qEV99owe4JAJcIFjJS+fo18AFEjuJGA7cntolG3nJV5260/6LbYX9/qwP/sV"
            crossorigin="anonymous"
        >
        </script>

        <link
            rel="stylesheet"
            href="https://checkoutshopper-${adyen_environment}.adyen.com/checkoutshopper/sdk/5.60.0/adyen.css"
            integrity="sha384-zgFNrGzbwuX5qJLys75cOUIGru/BoEzhGMyC07I3OSdHqXuhUfoDPVG03G+61oF4"
            crossorigin="anonymous" />
        <script>
            const adyen_vars = {
                // Environment to use (live or test)
                environment: '${adyen_environment}',
                // Public key used for client-side authentication: 
                // https://docs.adyen.com/development-resources/client-side-authentication
                clientKey: '${adyen_client_key}',
                // Unique identifier for the payment session
                sessionId: '${adyen_session_id}',
                // The payment session data
                sessionData: '${adyen_session_data}'
            };
        </script>
        <script
            type="module"
            src="${url.resourcesPath}/assets/js/main.js"
        ></script>
        <link
            rel="stylesheet"
            href="${url.resourcesPath}/assets/css/main.css"
            crossorigin="anonymous" />
    <#elseif section = "form">
        <form
            id="kc-adyen-form"
            class="${properties.kcFormClass!}"
            action="${url.loginAction}"
            method="post"
        >
            <div class="card-pf">
                <span class="card-details">${msg("adyen.form.cardDetails")}</span>
                <div id="dropin-container"></div>
            </div>
        </form>
    </#if>
</@layout.registrationLayout>
