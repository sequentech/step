<?php

// =============================================================================
// CUSTOM APPLICATION CONFIGURATION
// =============================================================================
// These are custom keys specific to our application, not SimpleSAMLphp config

$configOverride = [
    // =========================================================================
    // DEPLOYMENT CONFIGURATION
    // =========================================================================

    /**
     * The base URL where SimpleSAMLphp is hosted.
     *
     * Examples:
     * - Development: 'http://localhost:8083/simplesaml'
     * - Production: 'https://idp.example.com/simplesaml'
     */
    'idp_base_url' => getenv('IDP_BASE_URL') ?: 'http://localhost:8083/simplesaml',

    /**
     * The hostname/domain of your SimpleSAMLphp IdP server.
     *
     * Examples:
     * - Development: 'localhost:8083'
     * - Production: 'idp.example.com'
     */
    'idp_hostname' => getenv('IDP_HOSTNAME') ?: 'localhost:8083',

    // =========================================================================
    // TARGET APPLICATION CONFIGURATION
    // =========================================================================

    /**
     * The tenant ID for the voting system.
     * You will receive this from Sequent.
     *
     * Example: 'abc12345-6789-0123-4567-890123456789'
     */
    'tenant_id' => getenv('TENANT_ID') ?: '90505c8a-23a9-4cdf-a26b-4e19f6a097d5',

    /**
     * The event ID for the voting event.
     * You will receive this from Sequent.
     *
     * Example: 'def67890-1234-5678-9012-345678901234'
     */
    'event_id' => getenv('EVENT_ID') ?: 'cd1397d3-d236-42b4-a019-49143b616e13',

    // =========================================================================
    // SERVICE PROVIDER (KEYCLOAK) CONFIGURATION
    // =========================================================================

    /**
     * The Keycloak Service Provider base URL.
     * This is the URL where Keycloak is deployed.
     *
     * Sequent uses subdomain pattern: login-{SUBDOMAIN}.sequent.vote
     *
     * Examples:
     * - Development: 'http://127.0.0.1:8090'
     * - Production: 'https://login-example.sequent.vote'
     */
    'sp_base_url' => getenv('SP_BASE_URL') ?: 'http://127.0.0.1:8090',

    /**
     * The Keycloak identity provider alias.
     * This is configured on the Keycloak side and identifies your IdP.
     *
     * Default: 'simplesamlphp' (but can be customized by Sequent)
     */
    'sp_idp_alias' => getenv('SP_IDP_ALIAS') ?: 'simplesamlphp',

    /**
     * The SAML client ID within Keycloak that will receive the final redirect.
     *
     * Default: 'vp-sso' (voting portal SSO)
     */
    'sp_client_id' => getenv('SP_CLIENT_ID') ?: 'vp-sso',

    /**
     * The Keycloak public certificate for validating signed requests.
     * This is provided by Sequent during setup.
     *
     * Format: Base64-encoded X.509 certificate (without BEGIN/END markers)
     */
    'sp_cert_data' => getenv('SP_CERT_DATA') ?: 'MIIDOzCCAiMCBgGaFdFcAzANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LWNkMTM5N2QzLWQyMzYtNDJiNC1hMDE5LTQ5MTQzYjYxNmUxMzAeFw0yNTEwMjQxMDQyMTNaFw0zNTEwMjQxMDQzNTNaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtY2QxMzk3ZDMtZDIzNi00MmI0LWEwMTktNDkxNDNiNjE2ZTEzMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAkOwN1Qyksmq2qlGm0d1R3/ZPnUzZAeyyz5JnfQSfK+36fv5LEiH7IsfqKL1WFSJym/MucpjY1EgbxIqIVcmoE7Q08xasCYxGeoztnFW8Bt5BjBpJuIXCZ/e2UDWkAX9Sj5TGfZc4o3NEQVGJkun2njZFhK4Br7AThhPONzFrshcjAFIEEDzwdYRdorl50RFpeG1wfKeuDO894vEelPVxSGjuz4imxqIzzgzDXEx8Em/SHEKVkGhpFPc8aagFnnnXvMgn8QWpouXRcB5t1rKHrJU/ibPlW4x20lu8ddmsU45pEcsKutV9RewHRfLALTl51NHXAOMwLHE0+eRaY9j4JQIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQCB0GmlZY9/yHbawD1cATbPu0IFvCBDL+dNoxWpwAtxKjUiNpG7khqZOOTJ3lyYT5hwBV3Rv1XjUut6PYEaqTiAkyvpLIs5EbwrkWjBTsh3hHgeVtlRJVlaglmoM/nB1ocE3YtmnsmIetmcfgIlmHEghy2mugtA5g6x6KsGPbr4V0CaAzdCJ2t4vnE6O0qxqVv5AvQAhFMdp/r51xv8gGuhgzoHwtAKjdE0r4S2ZVajW9BXsqcCPCOIZYGUFWuQ4G39AUo1Lt6gGXUjZKveuJb0Nt7kSIxeHklLtkyH5c2A0wg5am7cfzBSrI8ANBt6ZgdIOYxh23qUHiqK8vj6OktL',

    /**
     * The voting portal base URL.
     * This is where users will be redirected after successful authentication.
     *
     * Sequent uses subdomain pattern: voting-{SUBDOMAIN}.sequent.vote
     *
     * Examples:
     * - Development: 'http://127.0.0.1:3000'
     * - Production: 'https://voting-example.sequent.vote'
     */
    'voting_portal_url' => getenv('VOTING_PORTAL_URL') ?: 'http://127.0.0.1:3000',

    // =========================================================================
    // DERIVED CONFIGURATION (DO NOT OVERRIDE)
    // =========================================================================

    /**
     * The Keycloak realm identifier (automatically constructed).
     * Format: tenant-{TENANT_ID}-event-{EVENT_ID}
     *
     * This is derived from TENANT_ID and EVENT_ID above.
     * Do not set SP_REALM environment variable - it's computed automatically.
     */
    'sp_realm' => 'tenant-' . (getenv('TENANT_ID') ?: '90505c8a-23a9-4cdf-a26b-4e19f6a097d5') .
                  '-event-' . (getenv('EVENT_ID') ?: 'cd1397d3-d236-42b4-a019-49143b616e13'),

    // =========================================================================
    // SIMPLESAMLPHP CONFIG OVERRIDES
    // =========================================================================

    /**
     * baseurlpath - The base URL path for SimpleSAMLphp
     * The base directory of the SimpleSAMLphp installation
     */
    'baseurlpath' => getenv('SSP_BASE_URL_PATH') ?: getenv('SSP_APACHE_ALIAS'),

    /**
     * timezone - The timezone to use for SimpleSAMLphp
     * Set to 'UTC' for consistency across deployments
     */
    'timezone' => 'UTC',

    /**
     * auth.adminpassword - Administrator password for SimpleSAMLphp web interface
     * Used to access the SimpleSAMLphp configuration interface
     */
    'auth.adminpassword' => getenv('SSP_ADMIN_PASSWORD'),

    /**
     * logging.level - The logging level for SimpleSAMLphp
     * SimpleSAMLphp log levels: ERR=3, WARNING=4, NOTICE=5, INFO=6, DEBUG=7
     */
    'logging.level' => (int) getenv('SSP_LOG_LEVEL'),

    /**
     * logging.handler - The logging handler to use
     * Options: 'syslog', 'file', 'errorlog'
     */
    'logging.handler' => getenv('SSP_LOG_HANDLER'),

    /**
     * usenewui - Whether to use the new SimpleSAMLphp UI
     * Set to true to use the modern interface
     */
    'usenewui' => true,

    /**
     * secretsalt - Secret salt used for generating secure hashes
     * Used for security-sensitive operations like state parameters
     */
    'secretsalt' => getenv('SSP_SECRET_SALT') ?: 'defaultsecretsalt',

    /**
     * enable.saml20-idp - Enable SAML 2.0 Identity Provider functionality
     * Must be true to act as an IdP
     */
    'enable.saml20-idp' => true,

    /**
     * session.cookie.secure - Whether session cookies should be marked as secure
     * Set to false in development mode to allow HTTP, true in production for HTTPS
     */
    'session.cookie.secure' => getenv('DEVELOPMENT_MODE') !== 'true',

    /**
     * session.cookie.samesite - SameSite attribute for session cookies
     * Set to 'Lax' in development to prevent CORS issues
     * Options: 'None', 'Lax', 'Strict', null
     */
    'session.cookie.samesite' => getenv('DEVELOPMENT_MODE') === 'true' ? 'Lax' : null,

    /**
     * saml20.idp.allowunsolicited - Allow IdP-initiated SSO
     * Enables SAML IdP-initiated login flows
     */
    'saml20.idp.allowunsolicited' => true,

    /**
     * store.type - Session storage backend type
     * Using 'memcache' for distributed session storage
     */
    'store.type' => 'memcache',

    /**
     * session.cookie.path - Path for session cookies
     * Set to '/' to make cookies available across all paths
     */
    'session.cookie.path' => '/',
];

// =============================================================================
// MODULE ENABLEMENT
// =============================================================================
// Enable modules dynamically via SSP_ENABLED_MODULES environment variable
// Example: SSP_ENABLED_MODULES="exampleauth admin"
$modules = getenv('SSP_ENABLED_MODULES');
if ($modules !== false) {
    foreach (explode(' ', $modules) as $moduleName) {
        $configOverride['module.enable'][$moduleName] = true;
    }
}

// =============================================================================
// APPLY OVERRIDES TO CONFIG
// =============================================================================
// Merge $configOverride into $config array
$config = array_replace_recursive($config, $configOverride);
