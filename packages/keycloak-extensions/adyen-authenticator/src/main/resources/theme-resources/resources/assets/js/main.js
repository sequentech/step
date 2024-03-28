
const adyenConfig = {
    environment: adyen_vars.environment,
    
    // Public key used for client-side authentication: 
    // https://docs.adyen.com/development-resources/client-side-authentication
    clientKey: adyen_vars.clientKey,
    
    // Set to false to not send analytics data to Adyen.
    analytics: {
        enabled: false
    },
    
    session: {
        // Unique identifier for the payment session
        id: adyen_vars.sessionId,

        // The payment session data
        sessionData: adyen_vars.sessionData,
    },
    onPaymentCompleted: (result, component) => {
        console.log("success! payment completed");
        console.info(result, component);

        // Select the form by its ID
        var form = document.getElementById('kc-adyen-form');

        // Submit the form
        form.submit();
    },
    onError: (error, component) => {
        console.error("error during payment");
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
            data: {
                holderName: adyen_vars.holderName,
            },
            billingAddressRequired: true,
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
