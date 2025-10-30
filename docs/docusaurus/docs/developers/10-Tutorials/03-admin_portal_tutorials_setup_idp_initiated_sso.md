---
id: admin_portal_tutorials_setup_idp_initiated_sso
title: Setup IDP Initiated SSO with SimpleSAMLphp as IDP and Keycloak as SP
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

This tutorial explains how to set up IdP-initiated Single Sign-On (SSO). In this scenario:

* **Identity Provider (IdP):** SimpleSAMLphp handles user authentication.
* **Service Provider (SP) / Broker:** Keycloak acts as an SP, trusting SimpleSAMLphp for authentication. It also acts as an Identity Broker.
* **Target Client (`vp-sso`):** A specific SAML client *within* Keycloak that itself acts as an SP, potentially relying on Keycloak (now acting as an IdP for it) after brokering the authentication from SimpleSAMLphp.
* **Flow:** The user starts at a special page hosted by SimpleSAMLphp, authenticates there, and is then redirected through Keycloak to a final target application (defined by the `vp-sso` client's configuration or RelayState).

## Prerequisites

1.  **SimpleSAMLphp Instance:** Running and accessible (e.g., via Docker at `http://localhost:8083/simplesaml/`). Ensure PHP dependencies are installed and required modules (`saml`, `exampleauth`, etc.) are enabled.
2.  **Keycloak Instance:** Running and accessible (e.g., at `http://127.0.0.1:8090/`). The realm should be configured as per your initial export.
3.  **Certificates:**
    * SimpleSAMLphp needs a private key (`server.pem`) and public certificate (`server.crt`) for signing SAML messages. Place these in the `cert/` directory configured in SimpleSAMLphp.
    * Keycloak needs its own realm keys (usually auto-generated).
    * You'll need the public certificates of each system to configure trust in the other.
4.  **Networking:** Ensure Keycloak and SimpleSAMLphp can reach each other's endpoints via HTTP/HTTPS as needed. We'll primarily use HTTP POST binding.
5.  **Keycloak Custom Provider (Optional but in your config):** The `SamlRedirectProvider` Java code needs to be compiled and deployed to Keycloak's `providers/` directory if you intend to use the custom `/redirect-provider/redirect` endpoint.

---

## Step 1: Configure SimpleSAMLphp (IdP)

Configure SimpleSAMLphp to act as the central IdP.

### 1.1. Enable IdP & Unsolicited SSO

Edit your SimpleSAMLphp configuration:

```php title="/var/simplesamlphp/config/config.php"
<?php

// *** Crucial SAML IdP Settings ***
enable.saml20-idp = true;       // Enable the SAML 2.0 IdP functionality
saml20.idp.allowunsolicited = true; // IMPORTANT: Allow IdP-initiated flow

// Session/State Storage (Memcache recommended for Docker, ensure memcached is running)
store.type = 'memcache';

// Enable required modules (for the example)
module.enable['exampleauth'] = true;

// Security: Use HTTPS in production
session.cookie.secure = true; // Enable if using HTTPS
```

### 1.2. Define Authentication Source

Configure how SimpleSAMLphp authenticates users. We'll use the example static user/password source:

```php title="/var/simplesamlphp/config/authsources.php"
<?php

$config = [
    'admin' => [
        'core:AdminPassword',
    ],

    'example-userpass' => [
        'exampleauth:UserPass',

        // Define test users and their attributes
        'student:studentpass' => [
            'uid' => ['student'],
            'eduPersonAffiliation' => ['member', 'student'],
            'email' => 'student@example.com', // **Keycloak will use this**
            'givenName' => 'Student',
            'sn' => 'User',
        ],
        'employee:employeepass' => [
            'uid' => ['employee'],
            'eduPersonAffiliation' => ['member', 'employee'],
            'email' => 'employee@example.com', // **Keycloak will use this**
            'givenName' => 'Employee',
            'sn' => 'User',
        ],
    ],
];
```

**Note:** Ensure attributes required by Keycloak (like email for the Principal Attribute) are defined for your test users.

### 1.3. Configure IdP Metadata (Hosted)

Define SimpleSAMLphp's own metadata as an IdP:

```php title="/var/simplesamlphp/metadata/saml20-idp-hosted.php"
<?php

$metadata['http://localhost:8083/simplesaml/saml2/idp/metadata.php'] = [
    // The 'host' identifies the virtual host for this IdP configuration.
    'host' => '__DEFAULT__', // Use for the default virtual host

    // Signing key and certificate (relative to the 'cert' directory)
    'privatekey' => 'server.pem',  // Your IdP's private signing key
    'certificate' => 'server.crt', // Your IdP's public certificate

    // The authentication source SimpleSAMLphp will use for users logging in via this IdP
    'auth' => 'example-userpass', // Must match an ID in authsources.php

    // --- Recommended Security Settings ---
    'sign.logout' => true,             // Sign logout requests/responses initiated by this IdP
    'saml20.sign.response' => true,    // Sign the entire SAML Response message
    'saml20.sign.assertion' => true,   // Sign the SAML Assertion within the Response

    // --- Optional Settings ---
    // 'SingleSignOnServiceBinding' => ['urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect', 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST'], // Specify supported bindings
    // 'SingleLogoutServiceBinding' => ['urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect', 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST'],
];
```

* The array key (`http://localhost:8083/...`) is the **Entity ID** of your SimpleSAMLphp IdP. Keycloak will need this.
* Make sure `privatekey` and `certificate` point to valid files in your `cert/` directory.

### 1.4. Configure SP Metadata (Remote)

Tell SimpleSAMLphp about the Keycloak instance (acting as SP/Broker) it trusts:

```php title="/var/simplesamlphp/metadata/saml20-sp-remote.php"
<?php

// Use the Keycloak Realm's SAML Entity ID as the key.
// Find this in Keycloak: Realm Settings -> General -> Endpoints -> SAML 2.0 Identity Provider Metadata -> entityID attribute
$metadata['tenant-e8062b49-532b-4f60-8548-0d3c14a25894-event-cd1397d3-d236-42b4-a019-49143b616e13'] = [
    'AssertionConsumerService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-cd1397d3-d236-42b4-a019-49143b616e13/broker/simplesamlphp/endpoint/clients/vp-sso',
        ],
    ],
    'SingleLogoutService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-cd1397d3-d236-42b4-a019-49143b616e13/broker/simplesamlphp/endpoint',
        ],
    ],

    // Keycloak's Public Certificate (for validating signatures FROM Keycloak, e.g., signed AuthnRequests)
    // Get this from Keycloak: Realm Settings -> Keys -> RS256 -> Public key -> Certificate (copy the content)
    'certData' => 'MIIDOzCCAiMCBgGZ3f8D1TANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LTM3ZWI1MWE3LWM2YjktNDU2Zi05M2I0LTViZDA1MDgxYjE4ZjAeFw0yNTEwMTMxNDMzMjFaFw0zNTEwMTMxNDM1MDFaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtMzdlYjUxYTctYzZiOS00NTZmLTkzYjQtNWJkMDUwODFiMThmMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1vdrir39ABcm6tIVuy9Y+G4sPtrz3Rg2KtCFYPlf+7cBBb8L75SCheZVEtPVZ7djv6g7GjksNeeUjMQiNfPNlI9PGCd1Eeei0WnZ7FGOvQoFWv7SWqeCzu8tZJtBRqeWnuK8zLeka/amgdoygZ0gR3bqA/hI3EpNlPExZQNGITsWDsYZ/SKEIIkq37kXV/yTsW8h6jnJMydqgkN0MESErFiVIjGwrAvC7kA+7HLj0sOCNOaHu2U6LhZznfJuJBipCLfMbtjConOsXZC5GmMsJD7txPpejXfb82kmSHJcvsq3GFqF616mrW3rh2iM/gso3ClLeHpzwUG0weaKFbWyhwIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQDUbmfepXwr3aWHs/8UpIANLZqGN95+BSfuYi82gI+x0fKaxT+a7Sy2Om0juh77E+01B5lzdR2R72/39r/+1PGTpLdoQwVP9kFaDMuMNdYCZ4XS0HAeETuMPTAZVAqxiUc09ey0uKOJbpdWA8X0SDN8igwpIJGW2PSMo9A7rbkmOPFEF71je793TguCMqNbVGdDHWiI0ySXZh3Pw/UPdYyRhoUgINNELMjBmS4Yv1+S4Lpqz9ZL39eCULN1VkD2GK7Fnh3rosrWNP6TTIWNkvUY2Fw6Ptc3sikouSJRAvBA4H2JFAT3LA5nD5kh2EQfbgxMlWNzan/KJIESqNyo5XxL',

    'validate.authnrequest' => true, // SimpleSAMLphp should validate signed AuthnRequests from Keycloak
    'validate.logoutrequest' => true, // SimpleSAMLphp should validate signed LogoutRequests from Keycloak
];
```

---

## Step 2: Configure Keycloak (SP/Broker & IdP for vp-sso)

### 2.1. Configure Keycloak to Trust SimpleSAMLphp (Identity Provider)

This configures Keycloak to act as an SP/Broker, using SimpleSAMLphp as an external IdP. Your initial realm export largely defines this.

1. **Navigate:** Go to **Identity Providers** in your Keycloak realm (tenant-...-event-...).
2. **Select/Verify simplesamlphp:** Create a provider with alias `simplesamlphp` (Provider ID `saml`)
   * **Import Config:** Alternatively, you could import SimpleSAMLphp's metadata (`http://localhost:8083/simplesaml/saml2/idp/metadata.php`).
   * **Key Settings:**
     * **Single Sign-On Service URL:** `http://localhost:8083/simplesaml/saml2/idp/SSOService.php`
     * **Single Logout Service URL:** `http://localhost:8083/simplesaml/saml2/idp/SingleLogoutService.php`
     * **NameID Policy Format:** Transient (matches SimpleSAMLphp config)
     * **Principal Type:** Attribute Name
     * **Principal Attribute:** email
     * **HTTP-POST Binding Response:** **ON**
     * **HTTP-POST Binding AuthnRequest:** **ON**
     * **Want AuthnRequests Signed:** **ON** (Keycloak will sign requests *to* SimpleSAMLphp)
     * **Signature Algorithm:** RSA_SHA256
     * **SAML Signature Key Name:** KEY_ID
     * **Want Assertions Signed:** **ON** (SimpleSAMLphp signs assertions)
     * **Validate Signatures:** **ON**
     * **Validating X509 Certificates:** Paste the content of SimpleSAMLphp's `server.crt` (public certificate), *without* the `-----BEGIN...` and `-----END...` lines.
3. **Mapper:**
   * Go to the **Mappers** tab for `simplesamlphp`.
   * Create a mapper that maps a SAML attribute to a Keycloak user attribute. For this example, we will create a mapper that maps the email:
     * **Name:** email-mapper
     * **Mapper type:** Attribute Importer
     * **Attribute Name:** email
     * **Friendly Name:** Email
     * **Name Format:** ATTRIBUTE_FORMAT_BASIC
     * **User Attribute Name:** email

### 2.2. Configure the vp-sso Client

This configures the specific `vp-sso` SAML client application.

1. **Navigate:** Go to **Clients**.
2. **Create Client:** Click **Create client**.
3. **General Settings:**
   * **Client type:** SAML
   * **Client ID:** vp-sso
   * Click **Next**.
4. **Capability config:** Keep defaults, click **Next**.
5. **Login settings:**
   * **Root URL:** `http://localhost:3000`
   * **Valid redirect URIs:** Add `http://localhost:3000/*` (or be more specific if possible). The redirect-provider URL goes into the fine-grain settings.
   * **IdP-initiated SSO URL Name:** vp-sso
   * **IdP-initiated SSO RelayState:** `http://localhost:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/cd1397d3-d236-42b4-a019-49143b616e13/login`
6. **Save.**
7. **Advanced Tab (Client Details):**
   * **Fine grain SAML endpoint configuration:**
     * **Assertion Consumer Service POST Binding URL:** `http://localhost:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-cd1397d3-d236-42b4-a019-49143b616e13/redirect-provider/redirect`
8. **SAML Capabilities:**
   * **Name ID format:** email
   * **Force POST binding:** **ON**
   * **Force Name ID format:** **OFF**
   * **Include AuthnStatement:** **ON**
   * **Sign Documents:** **ON**
   * **Sign Assertions:** **ON**
   * **Signature Algorithm:** RSA_SHA256
9. **Signature and Encryption:**
   * **Client signature required:** **ON**
   * **Signing Certificate:** Click **Import certificate**, Format: X.509 Certificate PEM, paste the external SP's public signing certificate.  

---

## Step 3: Create the IdP-Initiated SSO Trigger Page

Create a PHP page within SimpleSAMLphp's web-accessible directory to start the flow:

```php title="/var/simplesamlphp/public/idp-initiated-sso.php"
<?php

$simpleSamlBaseUrl = '/simplesaml'; // Path to simplesamlphp web root

// Keycloak's Entity ID (as configured in saml20-sp-remote.php)
$keycloakSpEntityId = 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-37eb51a7-c6b9-456f-93b4-5bd05081b18f';

// Final destination URL after successful authentication via Keycloak
$finalRedirectUrl = 'http://127.0.0.1:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/37eb51a7-c6b9-456f-93b4-5bd05081b18f/login'; // Your target application URL

// --- Construct the IdP-initiated SSO URL ---
$idpSsoUrl = "{$simpleSamlBaseUrl}/saml2/idp/SSOService.php";

$queryParams = [
    'spentityid' => $keycloakSpEntityId, // Tell IdP which SP to send assertion to
    'RelayState' => $finalRedirectUrl, // Tell SP where to redirect user finally
];

$loginUrl = $idpSsoUrl . '?' . http_build_query($queryParams);
?>

<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IdP Login Trigger</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style> body { font-family: 'Inter', sans-serif; } </style>
</head>
<body class="bg-gray-100 flex items-center justify-center h-screen">
    <div class="max-w-md w-full bg-white rounded-lg shadow-lg p-8 text-center">
        <div class="mb-6">
            <!-- Icon/Logo -->
            <div class="mx-auto flex items-center justify-center h-16 w-16 bg-blue-100 rounded-full">
                <svg class="h-8 w-8 text-blue-600" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" /></svg>
            </div>
            <h1 class="text-2xl font-bold text-gray-800 mt-4">IdP Login Page</h1>
            <p class="text-gray-600 mt-2">Start your Single Sign-On journey here.</p>
        </div>
        <div class="bg-gray-50 p-6 rounded-lg">
            <p class="text-gray-700 mb-4">Click below to log in via the central IdP.</p>
            <a href="<?php echo htmlspecialchars($loginUrl); ?>" class="w-full inline-block bg-blue-600 text-white font-bold py-3 px-4 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition duration-300 ease-in-out">
                Login via SimpleSAMLphp IdP
            </a>
        </div>
        <p class="text-xs text-gray-400 mt-6">You will be redirected to authenticate.</p>
    </div>
</body>
</html>
```

* Save this file in `/var/simplesamlphp/public/` (or your equivalent public web directory for SimpleSAMLphp).
* Access it via your browser, e.g., `http://localhost:8083/simplesaml/idp-initiated-sso.php`.

---

## Step 4: Test the Flow

1. **Open Browser:** Navigate to the trigger page: `http://localhost:8083/simplesaml/idp-initiated-sso.php`.
2. **Click Login Button:** Click the "Login via SimpleSAMLphp IdP" link.
3. **SimpleSAMLphp Authentication:** You should be redirected to the SimpleSAMLphp login page (e.g., `example-userpass` form). Enter valid credentials (e.g., `student`/`studentpass`).
4. **Redirection to Keycloak:** Upon successful authentication, SimpleSAMLphp generates a SAML assertion and POSTs it to Keycloak's broker endpoint (`.../broker/simplesamlphp/endpoint`), including the `RelayState`.
5. **Keycloak Processing:** Keycloak validates the assertion, finds or creates the user based on the email attribute, and establishes a Keycloak session.
6. **Final Redirection:** Keycloak should then redirect your browser to the URL specified in the `RelayState` (`http://127.0.0.1:3000/.../login`). If your custom `redirect-provider` is involved, Keycloak might redirect to that endpoint first, which then performs the final redirect based on the `RelayState` it receives.

You should now be logged into your target application, having authenticated only at SimpleSAMLphp.

---

## Step 5: Troubleshooting

* **Check Logs:** Examine both SimpleSAMLphp logs (location depends on `logging.handler`) and Keycloak logs (server log) for detailed error messages. Increase log levels if necessary.
* **SAML Tracer:** Use browser extensions (like SAML-tracer for Firefox, SAML Chrome Panel) to inspect the SAML messages being exchanged. Check for signature errors, incorrect destinations, missing attributes, or status code errors.
* **Metadata:** Ensure Entity IDs, ACS URLs, SLO URLs, and certificates match *exactly* between SimpleSAMLphp's SP metadata (`saml20-sp-remote.php`) and Keycloak's IdP configuration.
* **Certificates:** Verify that the correct public certificates are used for signature validation in both systems. Ensure private keys are correctly configured and accessible. Check for expiration.
* **Clock Skew:** Make sure the servers running SimpleSAMLphp and Keycloak have synchronized clocks (using NTP is recommended).
* **RelayState Issues:** If the final redirection fails:
  * Verify the `RelayState` parameter is correctly included in the initial URL and subsequent SAML responses (use a tracer).
  * Check Keycloak's broker settings and logs to see how it handles `RelayState`.
  * If using the custom `SamlRedirectProvider`, ensure it's deployed, check its specific logs, and verify the ACS URL in SimpleSAMLphp is pointing correctly if needed.
* **Session Storage:** If you see "state not found" errors, ensure SimpleSAMLphp's `store.type` is configured correctly (e.g., memcache is running and accessible if configured).
* **Attribute Mapping:** If login succeeds but user details are missing in the target app, verify that SimpleSAMLphp includes the necessary attributes in the assertion (`authsources.php`) and that Keycloak's IdP mapper (`email-mapper`) and potentially client mappers (`vp-sso` client) are correctly configured.