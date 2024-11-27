<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Keycloak: Send OTP SMS to US

We have added support to sending SMS OTP via Twilio Verify. To use it, the
deployment should change like it follows:

1. Add the appropriate env vars for keycloak:

```bash
TWILIO_ACCOUNT_SID="ACaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
TWILIO_SERVICE_SID="VAaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
TWILIO_AUTH_TOKEN="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
```

2. Configur the `twilio-verify` sms-sender in keycloak:

```
--spi-sms-sender-provider=twilio-verify
--spi-sms-sender-twilio-verify-enabled=true
--spi-sms-sender-aws-enabled=false
```

## ✨ Admin Portal > Publish and Results changes on `election_dates` field

The `election_dates` for publications, for electoral results and for templates
have been updated to include more information and a different internal
structure. On migrations, this requires:
1. Publishing a new publication for the ballot to work well
2. Update all reports that use these dates in S3

## ✨ Admin Portal > Reports > Audit Logs: Improve 

Windmill now requires `APP_VERSION` and `APP_HASH` for reports.

## ✨ Ask for Admin password for sensitive actions

This feature changes the behavior of some sensitive actions like starting an
election voting period or publishing a new publication of the ballot styles.

The way it works is by requiring gold level of authentication and for that the
user needs to re-authenticate.

### Keycloak: Migration to add `gold` level of authentication support

In the Admin Portal Realm:
1. Click `Realm Settings` in the sidebar
2. In the `General` tab, click `Add ACR to LoA Mapping`
3. Add two key-values pairs:
    - key: `silver`
      value: `1`
    - key: `gold`
      value: `2`
4. Click `Authentication` in the sidebar
5. Click `sequent browser blow` and ensure that:
   1. All the normal authentication flow is under a `normal / silver`
      connditional subflow with a required condition of type
   `Condition - Level of Authentication` and value `1`.
   2. it has a new conditional subflow
   called `advanced / gold condition` with a required condition of type
   `Condition - Level of Authentication` and value `2` and a Required
   `Password Form` step.

## ✨ Admin Portal: Reports: Prepare templates from Annex A

We have added new reports to be generated:

SBEI:
- Initializaton Report
- Status Report
- Ballot receipt
- Election Returns of National Positions
- Transmission Reports
- Audit Logs
- OVCS Information
- Overseas Voters' Turnout
- List of Overseas Voters
- Transmission Report

OFOV:
- Overseas Voting Monitoring - OVCS Events
- Overseas Voting Monitoring - OVCS Statistics
- Overseas Voters’ Turnout - per Aboard Status and Sex
- Overseas Voters’ Turnout - per Aboard Status, Sex and with Percentage
- List of OV who Pre-enrolled (Approved)
- List of OV who Pre-enrolled but subject for Manual Validation
- List of OV who Pre-enrolled but Disapproved
- List of OV who have not yet Pre-enrolled
- List of Overseas Voters who Voted
- List of Overseas Voters with Voting Status
- No. of OV who have not yet Pre-enrolled

### S3: New files to be uploaded

For existing environments the following files need to be uploaded to S3:

- .devcontainer/minio/public-assets/audit_logs.json
- .devcontainer/minio/public-assets/audit_logs_system.hbs
- .devcontainer/minio/public-assets/election_returns_for_national_positions.json
- .devcontainer/minio/public-assets/election_returns_for_national_positions_system.hbs
- .devcontainer/minio/public-assets/initialization.json
- .devcontainer/minio/public-assets/initialization_system.hbs
- .devcontainer/minio/public-assets/ov_users.json
- .devcontainer/minio/public-assets/ov_users_system.hbs
- .devcontainer/minio/public-assets/ov_users_who_voted.json
- .devcontainer/minio/public-assets/ov_users_who_voted_system.hbs
- .devcontainer/minio/public-assets/ovcs_events.json
- .devcontainer/minio/public-assets/ovcs_events_system.hbs
- .devcontainer/minio/public-assets/ovcs_information.json
- .devcontainer/minio/public-assets/ovcs_information_system.hbs
- .devcontainer/minio/public-assets/ovcs_statistics.json
- .devcontainer/minio/public-assets/ovcs_statistics_system.hbs
- .devcontainer/minio/public-assets/overseas_voters.json
- .devcontainer/minio/public-assets/overseas_voters_system.hbs
- .devcontainer/minio/public-assets/pre_enrolled_ov_but_disapproved.json
- .devcontainer/minio/public-assets/pre_enrolled_ov_but_disapproved_system.hbs
- .devcontainer/minio/public-assets/pre_enrolled_ov_subject_to_manual_validation.json
- .devcontainer/minio/public-assets/pre_enrolled_ov_subject_to_manual_validation_system.hbs
- .devcontainer/minio/public-assets/pre_enrolled_users.json
- .devcontainer/minio/public-assets/pre_enrolled_users_system.hbs
- .devcontainer/minio/public-assets/statistical_report.json
- .devcontainer/minio/public-assets/statistical_report_system.hbs
- .devcontainer/minio/public-assets/status.json
- .devcontainer/minio/public-assets/status_system.hbs
- .devcontainer/minio/public-assets/transmission.json
- .devcontainer/minio/public-assets/transmission_system.hbs

## Environment Variables - devcontainer:
Add `PUBLIC_ASSETS_PATH: ${PUBLIC_ASSETS_PATH}` to the harvest container in the following files:

- .devcontainer/docker-compose-airgap-preparation.yml
- .devcontainer/docker-compose.yml

## ✨ Manual voter application approval flow

There's a new tab `Approvals` in the Election Event.

### Migration to add permissions to keycloak realm

It requires to add a couple of permissions In order use Election event
`Approvals` tab:
1. Go to realm roles, click on `Create role`
2. Add the following roles to the admin group: `application-read`, `application-write`, `election-event-approvals-tab`, `election-approvals-tab`
3. Add the following roles to the sbei group: `application-read`, `application-write`, `election-approvals-tab`


## ✨ Add Permissions for Buttons and Tabs

### Added new permissions for Election Event Localization

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
election-event-localization-selector
localization-create
localization-read
localization-write
localization-delete
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

### Added new permissions for Election Event Areas

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
area-create
area-delete
area-export
area-import
area-upsert
election-event-areas-columns
election-event-areas-filters
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

### Added new permissions for Election Event Tasks

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
election-event-tasks-back-button
election-event-tasks-columns
election-event-tasks-filters
task-export
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups