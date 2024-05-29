<!--
SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Hasura Authentication and Keycloak

## Introduction

Hasura authentication is handled using `HASURA_GRAPHQL_JWT_SECRET` environment
variable for the `graphql-engine` hasura backend service. We use Keycloak to
handle the [JWT authentication tokens for Hasura endpoints]. We have one 
Keycloak realm per Election Event.

## JWK_URL configuration

Keycloak provides a `JWK_URL` per realm. For example, the `master` realm has the
following JWK URL in the devcontainer docker compose:
[http://keycloak:8090/realms/admin/protocol/openid-connect/certs]. We could
easily just configure HASURA_GRAPHQL_JWT_SECRET to thus be:

```json
{"jwk_url": "http://keycloak:8090/realms/electoral-process/protocol/openid-connect/certs"}
```

The content of that url would be something like what follows:

<details>
  <summary>Click to show JWK</summary>

```json
{
    "keys": [
        {
            "kid": "lEYINSzzCNmODNWD6j-rtIpb_jAPeklCCXp0Pdr8s38",
            "kty": "RSA",
            "alg": "RSA-OAEP",
            "use": "enc",
            "n": "pH0OAklVfiVdC7q3Ycdr71rV8xVmdWrm1N78C6ZZMm-cWPSsbVMzWsX25gYqgQEz1_y840ZrOb_U5y3jxMZrQ2e_PhaIdUW9ypYtgPoXLNzXaXHqGuOgVgypMgZ3f6ZSP0QtgkjtQ7seTwVRm78SdxOiGNfXudF_ssDgpeGb4qKBUxLst_6XpfJzTmhZinXxyYWIvwhQvvRNZr7tYWAkkmRK3Ssp7BjTvJENyUn7gpnEcZ4Dn_ur4g1oq-UXRuM9jJQKBtU2UMXVtCn11_mhpO6oRl5MqIYtJNvt-H7GIa2Vt2EVryQekMtubSyz7dA2DYshvgJWw7xKtlVyR6DRcQ",
            "e": "AQAB",
            "x5c": [
                "MIICsTCCAZkCBgGJ6hDOuDANBgkqhkiG9w0BAQsFADAcMRowGAYDVQQDDBFlbGVjdG9yYWwtcHJvY2VzczAeFw0yMzA4MTIxNDAzMzdaFw0zMzA4MTIxNDA1MTdaMBwxGjAYBgNVBAMMEWVsZWN0b3JhbC1wcm9jZXNzMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApH0OAklVfiVdC7q3Ycdr71rV8xVmdWrm1N78C6ZZMm+cWPSsbVMzWsX25gYqgQEz1/y840ZrOb/U5y3jxMZrQ2e/PhaIdUW9ypYtgPoXLNzXaXHqGuOgVgypMgZ3f6ZSP0QtgkjtQ7seTwVRm78SdxOiGNfXudF/ssDgpeGb4qKBUxLst/6XpfJzTmhZinXxyYWIvwhQvvRNZr7tYWAkkmRK3Ssp7BjTvJENyUn7gpnEcZ4Dn/ur4g1oq+UXRuM9jJQKBtU2UMXVtCn11/mhpO6oRl5MqIYtJNvt+H7GIa2Vt2EVryQekMtubSyz7dA2DYshvgJWw7xKtlVyR6DRcQIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQAxTFwTI62E2gyDifL57k+GR+7Jp+EvQncdLs1cnG9e3RJWaKnGyets1VwpBkoHUmFZT5ebY3cmHuYX5IdAOKmTioGOO60PQZ45ToFxVLszcYPG1jSLFuIvVPKahb3hHD6KEFX8ZGcuvVDVsVNDU2WUVwu0Do19HDRAMZQmxJjiDk3LSKLcLdO8+z2iVj3p9y7TRPnSsJQKB2tqv0kEdwpjOJiXZGEnBeKFT4tGRIiphazUZf51p8HYhRuX1pRDxZ1lsEYC13NV5YuPYDkNrBf6tyaMSQVa+5MXqv+Td0JqKqNr2jpYNbD+6OQ6MY8311F+y9kk17Tkx2cmZCQ7MCgl"
            ],
            "x5t": "pnt6atnAcd0jIwoyym4J_KFA1Co",
            "x5t#S256": "yyGTb83ZEdirYf6TJ6bZ3DKtTFWJg0zwWuS3LahnrhM"
        },
        {
            "kid": "IatIEaMwM8DA2TaVkvDBIqn71CjeF8KxLyUOLnpcEyI",
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": "q8-wv_ypwU2h2PjANU2_NNQT5NNVzI8au8Nzh7SD7Va2GJ41oef0zxa5SOIipBqcnbAcX9hCSaBqjg5CbSWy6XMWky54QzjDWdGhq9sqXIA6TAf2vKsrbL5daNQsRwBLwguB9QQKw52cL7T2NcIPS-y3H1oxPTfNUA92p2K8Eg7S3UN-kzxSyoytSJsO75HW20qrOcLls9u3_2wkMxy4Dn9ves4rcAEOR6IoMZVtOyZ8UBxiF552VhF1bC9yU6WSnDWaKdhACMf4mW2nJk5IvPF16iQ_cw2YJ5ro1KSVVra5WzPCZTIPuIu1IRWnZDgASnYJdrAu_pbNLO-JlwqGuQ",
            "e": "AQAB",
            "x5c": [
                "MIICsTCCAZkCBgGJ6hDOLTANBgkqhkiG9w0BAQsFADAcMRowGAYDVQQDDBFlbGVjdG9yYWwtcHJvY2VzczAeFw0yMzA4MTIxNDAzMzdaFw0zMzA4MTIxNDA1MTdaMBwxGjAYBgNVBAMMEWVsZWN0b3JhbC1wcm9jZXNzMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAq8+wv/ypwU2h2PjANU2/NNQT5NNVzI8au8Nzh7SD7Va2GJ41oef0zxa5SOIipBqcnbAcX9hCSaBqjg5CbSWy6XMWky54QzjDWdGhq9sqXIA6TAf2vKsrbL5daNQsRwBLwguB9QQKw52cL7T2NcIPS+y3H1oxPTfNUA92p2K8Eg7S3UN+kzxSyoytSJsO75HW20qrOcLls9u3/2wkMxy4Dn9ves4rcAEOR6IoMZVtOyZ8UBxiF552VhF1bC9yU6WSnDWaKdhACMf4mW2nJk5IvPF16iQ/cw2YJ5ro1KSVVra5WzPCZTIPuIu1IRWnZDgASnYJdrAu/pbNLO+JlwqGuQIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQCrSOi+z/AenFHHcFT+/j8NO7vepzCPAlwFB7oZuEJHJQ7P05j0Sz8S2V8zPXd8SXrU2iQIYwZH3sYoRB1bYxFAp+5AuXzzR1UKOUgme0T2cpYkmcPBY1QeWJ3URQJ3T3OsOlXGBaSRpNQg97UgsnlKfIDG8p28LgaOZLu991XttyToqMzIuix/4lyque7PZqHJ4IaLmARNgHRiTQTxW5zMw8Fi5DkCQdOStmggMgFegNeObNsbYO3TCFJN8FWQ7gQ9cSvGWXboIFUOAAT31rqZRHqv/2TTW8qq4mPZ8TH7a7PB2eKqk1cV8wpNk/tDsopWEgvj4DFj0hQUrU9D3u4l"
            ],
            "x5t": "SVki4OkikWg1vwV70KfwL03qGhg",
            "x5t#S256": "YHoF4l2HirDdO_CS1NBesL2t4O8C4q1pAoE8DJS0qQ8"
        }
    ]
}
```
</details>

Each tenant has its own Keycloak realm, called `tenant/<tenant-id>` and each
election event has its own realm, called
`tenant/<tenant-id>/event/<election-event-id>`.

## Multiple JWT Secrets with Harvest Endpoint

Related issue (not yet implemented): https://github.com/sequentech/meta/issues/199
Similar approach by other people: https://adhikary.net/2021/09/27/hasura-jwks-jwt-w-multiple-auth0-tenants

Since each realm has its own JWK URL and Hasura Open source only allows a single
JWK_URL to be configured per Hasura instance (see [Multiple JWT Secrets]), we
have created our own JWK URL endpoint at Hasura, that collects all the JWK keys
from all available Keycloak realms and returns them as a list. This is the
endpoint configured at `HASURA_GRAPHQL_JWT_SECRET`.

Harvest serves the JWK URL. In turn, harvest retrieves the information from
Hashicorp Vault and thus quickly returns the array of JWT Secrets in a single
JWK URL. Each time a new tenant or a new election event is
created/updated/removed, the Hashicorp Vault key storing the JWT Secrets array
is updated.

## JWT Claims configuration

Keycloak realms are configured to insert in the JWT claims the claims
`x-hasura-tenant-id` and `x-hasura-event-id`. Hasura, in turn, is configured to
use these claims in its permission rules to allow access only to the appropriate
tenant and election events related to the given claims. Other configured claims
are

Only Keycloak administrators of the `master` realm can configure the mappings of
these claims.

## Keycloak Claims Configuration

Additionally to `x-hasura-tenant-id` and `x-hasura-event-id`, there are other
claims configured in Keycloak for usage by Hasura. In particular:
- `x-hasura-default-role`
- `x-hasura-allowed-roles`

The Hasura claims configuration can be accessed in [Keycloak Administration UI]:
1. Select the relevant Realm. The `master` realm is selected by default, but you
   can choose a Tenant or Election Event realm.
2. Click in `Clients` in the left sidebar.
3. Click in `admin-portal` inside the `Client list` tab. This will launch the
   `admin-portal` client details. 
4. Click in the `Client scopes` tab on the top tab bar.
5. Click in the `admin-portal-dedicated` client scope.
6. This will show the list of dedicated Mappers, showing by default the
   `Mappers` tab and listing different mappers such as:
   - `x-hasura-tenant-id`
   - `x-hasura-event-id`
   - `x-hasura-default-role`
   - `x-hasura-allowed-roles`

Please note that there are other clients with potentially different claims
mapper configurations. The above is related to the claims mapper configuration
related to the Admin Portal client. For example, the configuration for the
Voting Portal would be in the corresponding `voting-portal` client list (step
3).

[JWT authentication tokens for Hasura endpoints]: https://hasura.io/docs/latest/auth/authentication/jwt/
[Multiple JWT Secrets]: https://hasura.io/docs/latest/auth/authentication/multiple-jwt-secrets/
[Keycloak Administration UI]: http://127.0.0.1:8090/admin/master/console