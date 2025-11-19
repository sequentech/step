<?php
/**
 * SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * IdP-Initiated SSO Trigger Page
 *
 * This page initiates the IdP-initiated SAML SSO flow to authenticate users
 * and redirect them to the Sequent voting portal through Keycloak.
 */

namespace SimpleSAML;

require_once('_include.php');

$config = Configuration::getInstance();

// Extract configuration values
$simpleSamlBaseUrl = '/simplesaml';

// Build the Keycloak SP Entity ID from realm
$keycloakSpEntityId = $config->getString('sp_realm');

// --- Logic ---
$idpSsoUrl = "{$simpleSamlBaseUrl}/saml2/idp/SSOService.php";

$queryParams = [
    'spentityid' => $keycloakSpEntityId,
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
    <style>
        body { font-family: 'Inter', sans-serif; }
    </style>
</head>
<body class="bg-gray-100 flex items-center justify-center h-screen">

    <div class="max-w-md w-full bg-white rounded-lg shadow-lg p-8 text-center">
        
        <div class="mb-6">
            <div class="mx-auto flex items-center justify-center h-16 w-16 bg-blue-100 rounded-full">
                <svg class="h-8 w-8 text-blue-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
            </div>
            <h1 class="text-2xl font-bold text-gray-800 mt-4">IdP Login Page</h1>
            <p class="text-gray-600 mt-2">You are on a page that can initiate SSO.</p>
        </div>
        
        <div class="bg-gray-50 p-6 rounded-lg">
            <p class="text-gray-700 mb-4">Click the button to log in via our central SimpleSAMLphp IdP.</p>
            
            <!-- This is now a simple link (styled as a button) to the correct IdP endpoint -->
            <a href="<?php echo htmlspecialchars($loginUrl); ?>" class="w-full inline-block bg-blue-600 text-white font-bold py-3 px-4 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition duration-300 ease-in-out">
                Login to Service Provider (Keycloak)
            </a>
        </div>
        
        <p class="text-xs text-gray-400 mt-6">
            This will redirect you to the SimpleSAMLphp IdP to authenticate.
        </p>

    </div>

</body>
</html>
