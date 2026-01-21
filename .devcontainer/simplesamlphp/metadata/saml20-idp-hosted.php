<?php
/**
 * SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * SAML 2.0 IdP Hosted Metadata Configuration
 *
 * This file defines the metadata for this SimpleSAMLphp instance acting as
 * an Identity Provider (IdP). It uses the centralized configuration to build
 * the entity ID dynamically.
 *
 * See: https://simplesamlphp.org/docs/stable/simplesamlphp-reference-idp-hosted
 */

// Load centralized configuration
require __DIR__ . '/../config/config.php';

// Build entity ID dynamically from configuration
$entityId = $config['idp_base_url'] . '/saml2/idp/metadata.php';

$metadata[$entityId] = [
    /*
     * The hostname of the server (VHOST) that will use this SAML entity.
     *
     * Can be '__DEFAULT__', to use this entry by default.
     */
    'host' => '__DEFAULT__',

    // X.509 key and certificate. Relative to the cert directory.
    'privatekey' => 'server.pem',
    'certificate' => 'server.crt',

    /*
     * Authentication source to use. Must be one that is configured in
     * 'config/authsources.php'.
     */
    'auth' => 'example-userpass',
    'saml20.sendartifact' => false,
    'ArtifactResolutionService' => [
        [
            'Location' => 'https://localhost:8083/simplesaml/saml2/idp/ArtifactResolutionService.php',
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Artifact',
            'isDefault' => true,
        ],
    ],

    'SingleSignOnServiceBinding' => [
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Artifact',
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect',
    ],

    'SingleLogoutServiceBinding' => [
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Artifact',
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
        'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect',
    ],
    
    'sign.logout' => true,
    'saml20.sign.response' => true,
    'saml20.sign.assertion' => true,
    'https.certificate' => '/etc/ssl/certs/ssl-cert-snakeoil.pem',
    'errorURL' => 'https://idp.example.com/simplesaml/',
];
