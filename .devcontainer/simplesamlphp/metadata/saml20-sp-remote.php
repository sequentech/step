<!-- 
 SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

<?php
// saml20-sp-remote.php

$keycloakSpAcsUrl = 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-cd1397d3-d236-42b4-a019-49143b616e13/broker/simplesamlphp/endpoint/clients/vp-sso';
$keycloakSpCertData = 'MIIDOzCCAiMCBgGaFdFcAzANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LWNkMTM5N2QzLWQyMzYtNDJiNC1hMDE5LTQ5MTQzYjYxNmUxMzAeFw0yNTEwMjQxMDQyMTNaFw0zNTEwMjQxMDQzNTNaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtY2QxMzk3ZDMtZDIzNi00MmI0LWEwMTktNDkxNDNiNjE2ZTEzMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAkOwN1Qyksmq2qlGm0d1R3/ZPnUzZAeyyz5JnfQSfK+36fv5LEiH7IsfqKL1WFSJym/MucpjY1EgbxIqIVcmoE7Q08xasCYxGeoztnFW8Bt5BjBpJuIXCZ/e2UDWkAX9Sj5TGfZc4o3NEQVGJkun2njZFhK4Br7AThhPONzFrshcjAFIEEDzwdYRdorl50RFpeG1wfKeuDO894vEelPVxSGjuz4imxqIzzgzDXEx8Em/SHEKVkGhpFPc8aagFnnnXvMgn8QWpouXRcB5t1rKHrJU/ibPlW4x20lu8ddmsU45pEcsKutV9RewHRfLALTl51NHXAOMwLHE0+eRaY9j4JQIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQCB0GmlZY9/yHbawD1cATbPu0IFvCBDL+dNoxWpwAtxKjUiNpG7khqZOOTJ3lyYT5hwBV3Rv1XjUut6PYEaqTiAkyvpLIs5EbwrkWjBTsh3hHgeVtlRJVlaglmoM/nB1ocE3YtmnsmIetmcfgIlmHEghy2mugtA5g6x6KsGPbr4V0CaAzdCJ2t4vnE6O0qxqVv5AvQAhFMdp/r51xv8gGuhgzoHwtAKjdE0r4S2ZVajW9BXsqcCPCOIZYGUFWuQ4G39AUo1Lt6gGXUjZKveuJb0Nt7kSIxeHklLtkyH5c2A0wg5am7cfzBSrI8ANBt6ZgdIOYxh23qUHiqK8vj6OktL';

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

    'certData' => $keycloakSpCertData,

    'validate.authnrequest' => true, 
    'validate.logoutrequest' => true,
];
