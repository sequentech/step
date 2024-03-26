<#import "adyen-template.ftl" as layout>
<#assign scripts ["${}"]>
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
            const adyenConfig = {
                environment: '${adyen_environment}',
                
                // Public key used for client-side authentication: 
                // https://docs.adyen.com/development-resources/client-side-authentication
                clientKey: '${adyen_client_key}',
                
                // Set to false to not send analytics data to Adyen.
                analytics: {
                    enabled: false
                },
                
                session: {
                    // Unique identifier for the payment session
                    id: '${adyen_session_id}',

                    // The payment session data
                    sessionData: '${adyen_session_data}'
                },
                onPaymentCompleted: (result, component) => {
                    console.info(result, component);
                },
                onError: (error, component) => {
                    console.error(error.name, error.message, error.stack, component);
                },
                // Any payment method specific configuration. Find the 
                // configuration specific to each payment method:
                // https://docs.adyen.com/payment-methods
                //
                // For example, this is 3D Secure configuration for cards:
                paymentMethodsConfiguration: {
                    card: {
                        hasHolderName: true,
                        holderNameRequired: true,
                        billingAddressRequired: true
                    }
                }
            };
            // Create an instance of AdyenCheckout using the configuration 
            // object
            const checkout = await AdyenCheckout(adyenConfig);

            // Create an instance of Drop-in and mount it to the container you
            // created
            const dropinComponent = checkout
                .create('dropin')
                .mount('#dropin-container');
        </script>
    </#if>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <div id="dropin-container"></div>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
