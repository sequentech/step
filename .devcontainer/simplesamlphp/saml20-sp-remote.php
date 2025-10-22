<!-- 
 SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

<?php
// saml20-sp-remote.php

$keycloakSpAcsUrl = 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-37eb51a7-c6b9-456f-93b4-5bd05081b18f/broker/simplesamlphp/endpoint/clients/vp-sso';
$keycloakSpCertData = 'MIIDOzCCAiMCBgGZ3f8D1TANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LTM3ZWI1MWE3LWM2YjktNDU2Zi05M2I0LTViZDA1MDgxYjE4ZjAeFw0yNTEwMTMxNDMzMjFaFw0zNTEwMTMxNDM1MDFaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtMzdlYjUxYTctYzZiOS00NTZmLTkzYjQtNWJkMDUwODFiMThmMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1vdrir39ABcm6tIVuy9Y+G4sPtrz3Rg2KtCFYPlf+7cBBb8L75SCheZVEtPVZ7djv6g7GjksNeeUjMQiNfPNlI9PGCd1Eeei0WnZ7FGOvQoFWv7SWqeCzu8tZJtBRqeWnuK8zLeka/amgdoygZ0gR3bqA/hI3EpNlPExZQNGITsWDsYZ/SKEIIkq37kXV/yTsW8h6jnJMydqgkN0MESErFiVIjGwrAvC7kA+7HLj0sOCNOaHu2U6LhZznfJuJBipCLfMbtjConOsXZC5GmMsJD7txPpejXfb82kmSHJcvsq3GFqF616mrW3rh2iM/gso3ClLeHpzwUG0weaKFbWyhwIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQDUbmfepXwr3aWHs/8UpIANLZqGN95+BSfuYi82gI+x0fKaxT+a7Sy2Om0juh77E+01B5lzdR2R72/39r/+1PGTpLdoQwVP9kFaDMuMNdYCZ4XS0HAeETuMPTAZVAqxiUc09ey0uKOJbpdWA8X0SDN8igwpIJGW2PSMo9A7rbkmOPFEF71je793TguCMqNbVGdDHWiI0ySXZh3Pw/UPdYyRhoUgINNELMjBmS4Yv1+S4Lpqz9ZL39eCULN1VkD2GK7Fnh3rosrWNP6TTIWNkvUY2Fw6Ptc3sikouSJRAvBA4H2JFAT3LA5nD5kh2EQfbgxMlWNzan/KJIESqNyo5XxL';

$metadata['http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-37eb51a7-c6b9-456f-93b4-5bd05081b18f'] = [
    'AssertionConsumerService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-37eb51a7-c6b9-456f-93b4-5bd05081b18f/broker/simplesamlphp/endpoint/clients/vp-sso',
        ],
    ],
    'SingleLogoutService' => [
        [
            'Binding' => 'urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST',
            'Location' => 'http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-37eb51a7-c6b9-456f-93b4-5bd05081b18f/broker/simplesamlphp/endpoint/clients/vp-sso',
        ],
    ],
    // 'saml:idp:url' => 'https://your-keycloak-domain/realms/{realm}/protocol/saml/clients/{idp-sso-url-name}',
    // 'signing.certificate' => 'MIIDOzCCAiMCBgGZ3f8D1TANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LTM3ZWI1MWE3LWM2YjktNDU2Zi05M2I0LTViZDA1MDgxYjE4ZjAeFw0yNTEwMTMxNDMzMjFaFw0zNTEwMTMxNDM1MDFaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtMzdlYjUxYTctYzZiOS00NTZmLTkzYjQtNWJkMDUwODFiMThmMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1vdrir39ABcm6tIVuy9Y+G4sPtrz3Rg2KtCFYPlf+7cBBb8L75SCheZVEtPVZ7djv6g7GjksNeeUjMQiNfPNlI9PGCd1Eeei0WnZ7FGOvQoFWv7SWqeCzu8tZJtBRqeWnuK8zLeka/amgdoygZ0gR3bqA/hI3EpNlPExZQNGITsWDsYZ/SKEIIkq37kXV/yTsW8h6jnJMydqgkN0MESErFiVIjGwrAvC7kA+7HLj0sOCNOaHu2U6LhZznfJuJBipCLfMbtjConOsXZC5GmMsJD7txPpejXfb82kmSHJcvsq3GFqF616mrW3rh2iM/gso3ClLeHpzwUG0weaKFbWyhwIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQDUbmfepXwr3aWHs/8UpIANLZqGN95+BSfuYi82gI+x0fKaxT+a7Sy2Om0juh77E+01B5lzdR2R72/39r/+1PGTpLdoQwVP9kFaDMuMNdYCZ4XS0HAeETuMPTAZVAqxiUc09ey0uKOJbpdWA8X0SDN8igwpIJGW2PSMo9A7rbkmOPFEF71je793TguCMqNbVGdDHWiI0ySXZh3Pw/UPdYyRhoUgINNELMjBmS4Yv1+S4Lpqz9ZL39eCULN1VkD2GK7Fnh3rosrWNP6TTIWNkvUY2Fw6Ptc3sikouSJRAvBA4H2JFAT3LA5nD5kh2EQfbgxMlWNzan/KJIESqNyo5XxL',

    'certData' => $keycloakSpCertData,

    'validate.authnrequest' => true, 
    'validate.logoutrequest' => true,
];