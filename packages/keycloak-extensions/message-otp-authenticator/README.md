<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Keycloak Message OTP Authenticator

A powerful Message OTP Authenticator for Keycloak.

## Design

This project is a Keycloak [SPI] (Service Provider Interface) that supports
multiple features:

- [x] OTP sent via SMS.
- [x] OTP sent via Email.
- [x] OTP shown in Keycloak log output if Provider is in simulation mode.
- [x] Configurable user-attribute to use to get the user's email address or
  telephone number.
- [x] Allow to send the same OTP via both SMS and Email if the user has both
  user attributes configured.
- [x] Integrate with "Reset OTP" Required Action, in which case it can be
  configured for a specific user that the OTP authenticators are disabled.
- [ ] A REST endpoint that generates and returns a smart link for resetting the
  user. This allows the integration with third-party systems (like windmill).
- [x] Email SMTP provider configurable using environment variables (SMTP API).
- [x] SMS sender provider configurable using environment variables (AWS SNS API
  only for now).

The user-attributes used for authentication should be read-only for the user, as
explained by Keycloak in [read-only user attributes]. Following that guide, use
something like following for read-only user attributes:

```bash
kc.[sh|bat] start \
  --spi-user-profile-declarative-user-profile-read-only-attributes=sequent.read-only.* \
  --spi-user-profile-declarative-user-profile-admin-read-only-attributes=sequent.read-only.*
```

## Provider deployment configuration

You can configure the AWS SNS credentials using the following environment
variables in your Keycloak docker image:
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`

These AWS variables are used by the [DefaultCredentialsProvider] which is used
by the [`SnsClient.create()`] function call in the `AwsSnsService` class.

You can configure the SMTP credentials for sending emails with the following
environment variables in your keycloak docker image:
- `SMTP_HOST`
- `SMTP_PORT`
- `SMTP_USER`
- `SMTP_PASSWORD`

## Additional resources

Another more complex and featureful provider for SMS authentication to look at
and get some inspiration is 
[https://github.com/cooperlyt/keycloak-phone-provider].

## Contributions and acknowledgements

This project started as a fork of [Dasniko's sms authenticator].

[SPI]: https://www.keycloak.org/docs/latest/server_development/index.html#_providers
[Dasniko's sms authenticator]: https://github.com/dasniko/keycloak-2fa-sms-authenticator
[read-only user attributes]: https://www.keycloak.org/docs/22.0.5/server_admin/#_read_only_user_attributes
[DefaultCredentialsProvider]: https://sdk.amazonaws.com/java/api/latest/software/amazon/awssdk/auth/credentials/DefaultCredentialsProvider.html
[`SnsClient.create()`]: https://sdk.amazonaws.com/java/api/latest/software/amazon/awssdk/services/sns/SnsClient.html#create()
