# Keycloak 2FA Authenticator

2FA Authenticator for Keycloak.

## Design

This project is a Keycloak [SPI] (Service Provider Interface) that supports
multiple features:

- [x] OTP sent via SMS.
- [ ] OTP sent via Email.
- [ ] Configurable user-attribute to use to get the user's email address or
  telephone number.
- [ ] Allow to send the same OTP via both SMS and Email if the user has both user
  attributes configured.
- [ ] Integrate with HOTP/TOTP, allowing the user to have multiple 2FA and choose if
  using either HOTP/TOTP or Email/SMS, using a configurable user-attribute that
  will be checked in a conditional flow.
- [ ] Integrate with "Reset OTP" Required Action, in which case it can be configured
  for a specific user that the OTP authenticators are disabled.
- Configurable templates for Email and SMS OTP.
- [ ] A REST endpoint that generates and returns a magic link for resetting the
  user. This allows the integration with third-party systems (like windmill).
- [ ] A REST endpoint that sends an OTP to the user. This allows the integration
  with third-party systems (like windmill).
- [ ] Email SMTP provider configurable using environment variables (SMTP API).
- [x] SMS sender provider configurable using environment variables (AWS SNS API only
  for now).

The user-attributes used for authentication should be read-only for the user, as
explained by Keycloak in [read-only user attributes]. Following that guide, use
something like following for read-only user attributes:

```bash
kc.[sh|bat] start \
  --spi-user-profile-declarative-user-profile-read-only-attributes=sequent.read-only.* \
  --spi-user-profile-declarative-user-profile-admin-read-only-attributes=sequent.read-only.*
```

## Contributions and acknowledgements

This project started as a fork of [Daniko's sms authenticator].

[SPI]: https://www.keycloak.org/docs/latest/server_development/index.html#_providers
[Daniko's sms authenticator]: https://github.com/dasniko/keycloak-2fa-sms-authenticator
[read-only user attributes]: https://www.keycloak.org/docs/22.0.5/server_admin/#_read_only_user_attributes