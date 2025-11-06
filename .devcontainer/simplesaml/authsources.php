<!-- 
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

<?php

$config = array(
    'admin' => array(
        'core:AdminPassword',
    ),

    'example-userpass' => array(
        'exampleauth:UserPass',
        'user1:password' => array(
            'email' => 'user1@example.com',
        ),
        'user2:password' => array(
            'email' => 'user2@example.com',
        ),
    ),

);
