<!-- 
 SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

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
            'username' => 'user1',
            'externalId' => 'X1',
        ),
        'user2:password' => array(
            'email' => 'user2@example.com',
            'username' => 'user2',
            'externalId' => 'Y2',
        ),
        'xalsina:1234' => array(
            'email' => 'XAVIER.ALSINA@sequentech.io',
            'username' => 'XALSINA',
            'externalId' => '39380103D',
        ),
        'eduardo:1234' => array(
            'email' => 'edulix@nvotes.com',
            'username' => 'edulix',
            'externalId' => '44055908Y',
        ),
    ),

);
