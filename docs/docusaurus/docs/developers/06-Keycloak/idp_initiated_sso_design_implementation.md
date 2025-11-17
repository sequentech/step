---
id: idp_initiated_sso_design_implementation
title: IdP SSO Design & Implementation
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# IdP-Initiated SAML SSO - Complete Design & Implementation

## Document Purpose

This document provides comprehensive technical documentation for Sequent developers working on the IdP-initiated SSO feature. It covers:
- Architecture and design decisions
- Component responsibilities
- Implementation details
- Reference implementation patterns
- Development workflow

**Audience:** Sequent internal developers

**For delivery/operations:** See [Delivery Team Configuration Guide](../Tutorials/setup_idp_initiated_sso)

**For third-parties:** See [Integration Guide](../../integrations/idp_initiated_sso_integration_guide)

---

## 1. Overview

This implementation enables IdP-initiated Single Sign-On (SSO) where **Keycloak** acts as the Service Provider (SP) / Identity Broker, integrating with **any** external SAML 2.0 compliant **Identity Provider (IdP)**. The flow allows users to initiate login from the external IdP and be seamlessly logged into the Sequent voting platform.

**Key Design Principle:** Clear separation between internal (Sequent-controlled) and external (third-party) components.

### Architecture Separation

**Internal Components (Sequent Maintains):**
- Keycloak (Service Provider/Broker) - `packages/keycloak-extensions/`
- Custom redirect provider - `SamlRedirectProvider`
- Voting portal application
- Realm configuration and management

**External Components (Third-Party Implements):**
- Identity Provider (IdP)
- User authentication mechanisms
- SAML assertion generation
- Reference: `.devcontainer/simplesamlphp/` (for testing/demonstration)

## 2. Goals

* Enable users to initiate login from an external SAML 2.0 IdP.  
* Authenticate users against the external IdP's configured mechanisms.  
* Establish a session in Keycloak based on the successful authentication at the external IdP.  
* Map user attributes (defined by the SAML assertion) to the Keycloak user profile (e.g., email, username, groups).  
* Redirect the user to a predefined target application URL (`RelayState`) after successful brokered login via Keycloak.  
* Utilize specific SAML configurations (e.g., signing keys, endpoints) for the target `vp-sso` client within Keycloak.  
* Handle the final redirection using a custom Keycloak endpoint.

## 3. Non-Goals

* Implementation of SP-initiated SSO flow (where the user starts at the target application).  
* Detailed configuration steps for the **external IdP** (as this varies widely).  
* Complex attribute transformations beyond Keycloak's standard broker mappers.  
* High availability or clustering setup details for Keycloak or the external IdP.

## 4. Architecture and Components

1.  **User's Browser:** Initiates the request and handles redirects and POST/Redirect requests between the IdP and Keycloak.  
2.  **External SAML 2.0 IdP:**  
    * The primary **Identity Provider (IdP)** (e.g., ADFS, Okta, Shibboleth, another Keycloak instance, etc.).  
    * Handles user authentication according to its policies.  
    * Generates SAML 2.0 assertions containing user identity information.  
    * Signs SAML responses and/or assertions using its configured keypair.  
    * Must be configured to trust Keycloak as a Service Provider, knowing its Entity ID and Assertion Consumer Service (ACS) URL.  
    * Must support IdP-initiated SSO and allow sending unsolicited responses to Keycloak.  
3.  **Keycloak Instance:**  
    * Acts as a **Service Provider (SP) / Identity Broker** **for the external IdP**. It trusts the external IdP via a configured Identity Provider entry.  
    * Acts as an **Identity Provider (IdP)** **for the `vp-sso` client**.  
    * Receives and validates SAML assertions from the external IdP.  
    * Maps asserted user attributes to Keycloak user accounts (potentially creating users via the "First Broker Login" flow).  
    * Establishes a Keycloak user session.  
    * Contains the configuration for the target **`vp-sso` SAML Client**, defining its specific endpoints, signing keys, and IdP-initiated behaviour relative to Keycloak.  
    * Hosts the custom **`SamlRedirectProvider`** endpoint (`/redirect-provider/redirect`) for final redirection.  
4.  **Target Application (Represented by `vp-sso` client config):** The final destination service configured as a SAML client within Keycloak.

**Interaction Diagram:**

```mermaid  
sequenceDiagram  
    participant UserBrowser as User Browser  
    participant ExternalIdP as External SAML 2.0 IdP  
    participant KeycloakBrokerSP as Keycloak (Broker/SP Role)  
    participant CustomRedirect as Keycloak Custom Redirect Endpoint
    participant TargetApp as Target Application (vp-sso)

    UserBrowser->>ExternalIdP: Accesses IdP's IdP-Initiated SSO endpoint (URL specific to IdP, includes target SP Entity ID & RelayState)  
    alt Not Authenticated at IdP  
        ExternalIdP-->>UserBrowser: Redirect to IdP Login Form  
        UserBrowser->>ExternalIdP: Submits Credentials  
        ExternalIdP->>ExternalIdP: Authenticate User  
    end  
    ExternalIdP->>ExternalIdP: Generate SAML Assertion (incl. attributes)  
    ExternalIdP-->>UserBrowser: Auto-Submit HTML Form (POST SAMLResponse to Keycloak ACS, includes RelayState)  
    UserBrowser->>KeycloakBrokerSP: POST SAMLResponse + RelayState to /broker/\{idp-alias\}/endpoint  
    KeycloakBrokerSP->>KeycloakBrokerSP: Validate Signature (using External IdP cert)  
    KeycloakBrokerSP->>KeycloakBrokerSP: Process Assertion, Map User Attributes, Create/Link User  
    KeycloakBrokerSP->>KeycloakBrokerSP: Establish Keycloak Session  
    alt Standard RelayState Handling  
        KeycloakBrokerSP-->>UserBrowser: Redirect based on RelayState (to TargetApp URL)  
    else Custom Redirect Endpoint Handling  
        KeycloakBrokerSP-->>UserBrowser: Redirect or Internal Forward to CustomRedirect Endpoint (Potentially carrying RelayState)  
        UserBrowser->>CustomRedirect: Accesses /redirect-provider/redirect  
        CustomRedirect->>CustomRedirect: Reads RelayState, validates against vp-sso config  
        CustomRedirect-->>UserBrowser: Redirect based on RelayState (to TargetApp URL)  
    end  
    UserBrowser->>TargetApp: Accesses Final URL (already has Keycloak session cookie)  
    TargetApp-->>UserBrowser: Application Interface (logged in)
```

## 5. Flow Description (IdP-Initiated)

1. **Initiation:** The user accesses a specific URL provided by the **external IdP** designed for IdP-initiated SSO. This URL typically includes parameters identifying the target SP (Keycloak's Entity ID) and the desired final RelayState (target application URL).  
2. **Authentication at IdP:** The external IdP checks if the user has an active session.  
   * If not, it prompts the user to authenticate using its configured methods (e.g., username/password, MFA).  
   * The user authenticates.  
3. **Assertion Generation:** The external IdP generates a SAML 2.0 Response containing an Assertion with user identity attributes, audience restriction set to Keycloak's Entity ID, and signs it according to its configuration.  
4. **POST to Keycloak:** The IdP sends an auto-submitting HTML form to the user's browser, instructing it to HTTP POST the SAML Response and the RelayState value to Keycloak's Assertion Consumer Service (ACS) URL specific to the configured broker entry (e.g., `/realms/{realm}/broker/{idp-alias}/` endpoint).  
5. **Keycloak Processing (Broker Role):**  
   * Keycloak receives the POST request at its broker endpoint.  
   * It validates the SAML Response signature using the external IdP's public certificate (configured in Keycloak's Identity Provider settings).  
   * It parses the Assertion and extracts user attributes based on the configured Mappers.  
   * It processes the "First Broker Login" flow to find, create, or link the user in the Keycloak realm.  
   * It establishes an authenticated session for the user within Keycloak.  
6. **Final Redirection:**  
   * Keycloak handles the RelayState.  
   * **Scenario A (Standard):** Keycloak redirects the user's browser directly to the URL provided in the RelayState.  
   * **Scenario B (Custom Endpoint):** If Keycloak is configured (e.g., via the vp-sso client's ACS URL pointing internally, or post-login flows) to use the custom `/redirect-provider/redirect` endpoint, that endpoint handles validating the RelayState against the vp-sso client's allowed redirects and issues the final browser redirect.  
7. **Target Application Access:** The browser is redirected to the final RelayState URL. The user now has a Keycloak session cookie. If the target application (vp-sso) relies on Keycloak for authentication (either via SAML where Keycloak is IdP, or OIDC), it will recognize the session and grant access.

## 6. Key Configuration Details

### 6.1. External SAML 2.0 IdP

* **User Authentication:** Configured according to organizational requirements.  
* **SAML 2.0 Support:** Must support SAML 2.0 protocol.  
* **IdP-Initiated SSO:** Must have a mechanism/endpoint to initiate SSO, allowing specification of the target SP Entity ID and RelayState.  
* **Trust Configuration (SP Metadata):** Must be configured to trust Keycloak as an SP. This involves providing Keycloak's:  
  * Entity ID (e.g., http://keycloak-host/realms/\{realm\}).  
  * Assertion Consumer Service (ACS) URL (e.g., http://keycloak-host/realms/\{realm\}/broker/\{idp-alias\}/endpoint).  
  * Signing Certificate (Keycloak's realm certificate, if the IdP validates signed AuthnRequests).  
  * Required NameID format and attributes.  
* **Signing:** Must sign SAML Responses and/or Assertions using its private key.

### 6.2. Keycloak - Identity Provider (Broker Config)

* **Provider:** Add a new SAML v2.0 Identity Provider.
* **Alias:** A unique name for this IdP configuration (e.g., external-idp).
* **Endpoints:** Single Sign-On Service URL, Single Logout Service URL obtained from the external IdP's metadata.
* **Entity ID:** The external IdP's SAML Entity ID.
* **Signatures:** Want AuthnRequests Signed (optional, depends on IdP requirement), Validate Signatures=ON, external IdP's public signing certificate added to Validating X509 Certificates.
* **Principal Identification:** Configure NameID Policy Format, Principal Type, and Principal Attribute based on what the IdP sends and how users should be identified in Keycloak.
* **Mappers:** Create mappers (e.g., Attribute Importer, Username Template Importer) to map attributes from the IdP's assertion (SAML Attribute Name) to Keycloak user attributes (User Attribute Name).
* **Flows:** Configure First Broker Login Flow.
* **Auto-Redirect (Optional):** Configure the Identity Provider Redirector in the browser authentication flow with the IdP alias as default provider to automatically redirect users to the external IdP, bypassing the Keycloak login page.

### 6.3. Keycloak - Client (vp-sso)

* **Client ID:** vp-sso (Type: SAML).  
* **Role:** Acts as an SP *relative to Keycloak*.  
* **Endpoints:** Assertion Consumer Service POST Binding URL configured (potentially the custom redirect provider /redirect-provider/redirect). Root URL set.  
* **IdP-Initiated (Keycloak as IdP):** Enabled via IdP-initiated SSO URL Name (vp-sso) and IdP-initiated SSO RelayState (target application URL). *Note: This specific URL is for initiating login at Keycloak directly to access vp-sso, separate from the main external IdP-initiated flow.*  
* **Signatures:** Configured based on the target application's requirements (e.g., Client signature required, Sign Documents).  
* **Keys:** Can use realm keys or import client-specific signing keys.  
* **Client Scopes / Mappers:** Configured to include necessary information (like roles via role_list) in the SAML assertion Keycloak generates *for* the vp-sso application.

### 6.4. Custom Redirect Provider (SamlRedirectProvider)

* **Purpose:** Provides an explicit endpoint within Keycloak to handle the RelayState redirection, potentially adding validation logic tied to specific client configurations.  
* **Deployment:** Requires building a JAR and deploying to Keycloak.  
* **Integration:** Needs to be correctly invoked after the broker login flow, possibly by setting it as the ACS URL in the external IdP's configuration for Keycloak, or through custom authentication flows in Keycloak.

## 7. Security Considerations

* **Transport Security:** Use HTTPS for all endpoints (IdP, Keycloak, Target Application).  
* **Message Signing & Validation:** Ensure SAML messages (Assertions, Responses, Requests) are appropriately signed and validated by both the IdP and Keycloak broker using exchanged public certificates.  
* **Encryption:** Consider SAML assertion encryption if sensitive attributes are transmitted over untrusted networks.  
* **RelayState Validation:** Implement strict validation of the RelayState URL against an allowlist in Keycloak (either standard broker validation or within the custom provider) to prevent open redirect vulnerabilities.  
* **Certificate Management:** Securely manage private keys and monitor certificate validity periods.  
* **Audience Restriction:** Ensure the IdP correctly sets the Audience element in the SAML assertion to Keycloak's Entity ID, and Keycloak validates it.

## 8. Reference Implementation (SimpleSAMLphp)

### Purpose

The SimpleSAMLphp setup in `.devcontainer/simplesamlphp/` serves as:
1. **Development/Testing Tool** - Allows Sequent developers to test Keycloak SP without external IdP
2. **Reference Pattern** - Demonstrates best practices for third-party integrators

### Key Design Principles

#### Centralized Configuration

**File:** `.devcontainer/simplesamlphp/config.php`

All deployment-specific values centralized in one location:

```php
return [
    'idp_base_url' => getenv('IDP_BASE_URL') ?: 'http://localhost:8083/simplesaml',
    'sp_base_url' => getenv('SP_BASE_URL') ?: 'http://127.0.0.1:8090',
    'sp_realm' => 'tenant-' . (getenv('TENANT_ID') ?: '...') . '-event-' . (getenv('EVENT_ID') ?: '...'),
    'tenant_id' => getenv('TENANT_ID') ?: '...',
    'event_id' => getenv('EVENT_ID') ?: '...',
    // ... etc
];
```

**Benefits:**
- Single source of truth
- Environment variable support (dev, staging, production)
- No hardcoded tenant/event IDs
- Easy to adapt to third-party IdP platforms

#### Dynamic Metadata Generation

**Files:**
- `.devcontainer/simplesamlphp/metadata/saml20-idp-hosted.php` - IdP metadata
- `.devcontainer/simplesamlphp/metadata/saml20-sp-remote.php` - SP (Keycloak) metadata

Example from SP remote metadata:

```php
$config = require __DIR__ . '/../config.php';

$keycloakSpAcsUrl = sprintf(
    '%s/realms/%s/broker/%s/endpoint/clients/%s',
    $config['sp_base_url'],
    $config['sp_realm'],
    $config['sp_idp_alias'],
    $config['sp_client_id']
);

$metadata[$config['sp_realm']] = [
    'AssertionConsumerService' => [
        ['Location' => $keycloakSpAcsUrl],
    ],
    // ...
];
```

**Benefits:**
- Eliminates copy-paste errors
- Consistency across all metadata
- Easy environment switching
- Shows third-parties correct URL construction

#### Environment-Based Configuration

**File:** `.devcontainer/.env.development`

Development environment variables:

```bash
# IdP Configuration (SimpleSAMLphp)
IDP_BASE_URL=http://localhost:8083/simplesaml

# SP Configuration (Keycloak)
SP_BASE_URL=http://127.0.0.1:8090
SP_REALM=tenant-...-event-...
SP_IDP_ALIAS=simplesamlphp
SP_CLIENT_ID=vp-sso
SP_CERT_DATA=MII...

# Target Application
TENANT_ID=...
EVENT_ID=...
VOTING_PORTAL_URL=http://localhost:3000

# Example users for testing (SimpleSAMLphp only)
SSP_EXAMPLE_USERS=user1:password:user1@example.com,user2:password:user2@example.com
```

**Third-party template:** `.devcontainer/simplesamlphp/.env.example`

**Benefits of environment-based user configuration:**
- Easy customization for different testing scenarios
- No need to modify PHP code for different test users
- Clear separation between code and configuration
- Third-parties can easily understand how to configure their own IdP users

### Development Workflow

**Local Development:**
```
IdP: http://localhost:8083 (SimpleSAMLphp)
SP: http://127.0.0.1:8090 (Keycloak)
Voting Portal: http://localhost:3000
```

**Third-Party Development (against Sequent staging):**
```
IdP: http://localhost:8083 (third-party's local IdP)
SP: https://auth-staging.sequentech.io (Sequent's staging Keycloak)
Voting Portal: https://voting-staging.sequentech.io
```

**Production:**
```
IdP: https://idp.thirdparty.com (third-party's production IdP)
SP: https://auth.sequentech.io (Sequent's production Keycloak)
Voting Portal: https://voting.sequentech.io
```

### Testing with Reference Implementation

1. Start dev container (SimpleSAMLphp runs automatically)
2. Access trigger page: `http://localhost:8083/simplesaml/idp-initiated-sso.php`
3. Login with test credentials (configured via `SSP_EXAMPLE_USERS` env var):
   - Default: `user1`/`password` (email: user1@example.com)
   - Or: `user2`/`password` (email: user2@example.com)
   - Customize by editing `SSP_EXAMPLE_USERS` in `.devcontainer/.env`
4. Verify redirect to voting portal

---

## 9. Configuration Flow & Third-Party Integration

### For Third-Party Integration

**Sequent provides to third-party:**
- Keycloak SP base URL
- Realm identifier (tenant-event combination)
- IdP alias (what Sequent names their IdP in Keycloak)
- Keycloak public certificate
- Tenant ID and Event ID
- Voting portal URL

**Third-party provides to Sequent:**
- IdP Entity ID
- IdP SSO service URL
- IdP public certificate
- IdP metadata URL (if available)

**Configuration occurs:**
1. Third-party configures their IdP to trust Sequent Keycloak as SP
2. Sequent configures Keycloak to trust third-party IdP
3. Both parties test integration in staging environments
4. Go-live coordination

### Example URLs

**Keycloak Metadata URL:**
```
{SP_BASE_URL}/realms/tenant-{TENANT_ID}-event-{EVENT_ID}/broker/{IDP_ALIAS}/endpoint/descriptor
```

**ACS URL:**
```
{SP_BASE_URL}/realms/{REALM}/broker/{IDP_ALIAS}/endpoint/clients/{CLIENT_ID}
```

**Voting Portal Login:**
```
{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{EVENT_ID}/login
```

---

## 10. Security Considerations

### For Sequent Developers

- **Certificate validation:** Always validate SAML signatures from external IdPs
- **Audience restriction:** Ensure SAML assertions specify correct realm as audience
- **RelayState validation:** Validate RelayState URLs against allowlist (prevent open redirects)
- **HTTPS enforcement:** All production endpoints must use HTTPS
- **Certificate rotation:** Monitor certificate expiration for all third-party IdPs
- **Logging:** Log all authentication attempts for security audit

### For Third-Party Integrators (documented in integration guide)

- **SAML signing:** Always sign SAML responses and/or assertions
- **Certificate management:** Protect private keys, rotate regularly
- **HTTPS requirement:** Mandatory for production
- **Attribute validation:** Ensure required attributes (email) are included
- **Clock synchronization:** Use NTP to prevent timing issues

---

## 11. Troubleshooting for Developers

### Common Development Issues

**Issue:** SimpleSAMLphp metadata shows wrong Entity ID

**Cause:** `IDP_BASE_URL` environment variable not set or incorrect

**Solution:** Check `.devcontainer/.env.development` and restart container

---

**Issue:** Keycloak can't find SP remote metadata

**Cause:** `SP_REALM` value mismatch between config and metadata array key

**Solution:** Ensure `$metadata[$config['sp_realm']]` uses exact realm identifier

---

**Issue:** RelayState not preserved through flow

**Cause:** Third-party IdP not passing RelayState in SAML Response POST

**Solution:** Verify IdP includes RelayState parameter in POST to Keycloak ACS

---

## 12. Related Documentation

- **Delivery Team Guide:** [Setup IdP-Initiated SSO](../Tutorials/setup_idp_initiated_sso)
- **Third-Party Integration Guide:** [IdP-Initiated SSO Integration Guide](../../integrations/idp_initiated_sso_integration_guide)
- **Reference Implementation README:** `.devcontainer/simplesamlphp/README.md`

---

## 13. Future Considerations

### Potential Enhancements

- **Multiple IdP support:** Allow multiple external IdPs per realm
- **Enhanced attribute mapping:** Support more complex attribute transformations
- **Metadata refresh:** Automatic periodic metadata refresh from external IdPs
- **Monitoring/metrics:** Track authentication success rates, errors by IdP

### Known Limitations

- **Single tenant-event per integration:** Current design assumes one IdP per tenant-event
- **Manual configuration:** Third-party IdP setup requires manual coordination
- **Certificate rotation:** Requires coordinated updates on both sides

---

## 14. Assumptions and Dependencies

* The external IdP is SAML 2.0 compliant and supports IdP-initiated SSO
* Metadata (or configuration parameters) can be exchanged between the external IdP and Keycloak administrators
* Network connectivity allows communication between all involved components
* If the custom redirect provider is used, it is correctly built and deployed
* SimpleSAMLphp reference implementation available for testing