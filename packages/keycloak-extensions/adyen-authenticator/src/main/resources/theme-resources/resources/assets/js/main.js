// Create an instance of AdyenCheckout using the configuration 
// object
const checkout = await AdyenCheckout(adyenConfig);

// Create an instance of Drop-in and mount it to the container you
// created
const dropinComponent = checkout
    .create('dropin')
    .mount('#dropin-container');
