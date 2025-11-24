<!--
 SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

<?php

// Parse SSP_EXAMPLE_USERS environment variable
// Format: "username1:password1:email1,username2:password2:email2,..."
// Example: "user1:password:user1@example.com,user2:password:user2@example.com"
$exampleUsersEnv = getenv('SSP_EXAMPLE_USERS') ?: 'user1:password:user1@example.com,user2:password:user2@example.com';
$exampleUsers = [];

foreach (explode(',', $exampleUsersEnv) as $userSpec) {
    $parts = explode(':', trim($userSpec), 3);
    if (count($parts) === 3) {
        $username = $parts[0];
        $password = $parts[1];
        $email = $parts[2];
        $exampleUsers["$username:$password"] = [
            'email' => $email,
        ];
    }
}

$config = array(
    'admin' => array(
        'core:AdminPassword',
    ),

    'example-userpass' => array_merge(
        ['exampleauth:UserPass'],
        $exampleUsers
    ),

);
