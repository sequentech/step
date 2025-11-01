<!--
 SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

<?php
/**
 * SAML 2.0 SP Remote Metadata Configuration
 *
 * This file defines the metadata for the remote Service Provider (Keycloak).
 * It uses the centralized configuration to build all URLs and settings
 * dynamically.
 *
 * The entity ID here is the Keycloak realm name, and all endpoints are
 * constructed based on the Keycloak deployment URL and realm configuration
 * provided by Sequent.
 */

// Load centralized configuration
$config = require __DIR__ . '/../config.php';

// Build URLs dynamically from configuration
$keycloakSpAcsUrl = sprintf(
    '%s/realms/%s/broker/%s/endpoint/clients/%s',
    rtrim($config['sp_base_url'], '/'),
    $config['sp_realm'],
    $config['sp_idp_alias'],
    $config['sp_client_id']
);

$keycloakSloUrl = sprintf(
    '%s/realms/%s/broker/%s/endpoint',
    rtrim($config['sp_base_url'], '/'),
    $config['sp_realm'],
    $config['sp_idp_alias']
);

// Use the SP entity ID (realm name) as the metadata array key
$metadata[$config['sp_realm']] = [
    'AssertionConsumerService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => $keycloakSpAcsUrl,
        ],
    ],
    'SingleLogoutService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => $keycloakSloUrl,
        ],
    ],

    // Keycloak's public certificate (provided by Sequent during setup)
    'certData' => $config['sp_cert_data'],

    // Validate signed requests from Keycloak
    'validate.authnrequest' => true,
    'validate.logoutrequest' => true,
];
