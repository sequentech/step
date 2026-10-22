---
id: reports_and_templates
title: Reports and Templates
description: "To generate a report, you need to select the Election Event in the Admin Portal and add an entry for a specific report type."
---

## Overview

To generate a report, you need to select the Election Event in the Admin Portal and add an entry for a specific report type.

## Creating a Report

1. Go to the **REPORTS** tab and click the **Create Report** or **ADD** button.
2. Select the type (see special notes for specific types below).
3. **Optional:** Select Election for reporting only a specific election.
4. **Optional:** Choose a template. You must have created one earlier (see [more about creating templates](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md)). If none is selected, the system's default will be used.
5. Click **Save**.

## Generating Reports

After creating a report entry, you will see it in the list. Click on the actions menu (3 dots on the right side) and choose:

- **Generate:** Create a report based on real data. Only **Activity Logs** and **Participation Report** have this action.
- **Preview:** Show the template with example data. The preview does not contain real data.

## Report Types

| Report type | Content | How you get the real document | Share outside the election organization |
| --- | --- | --- | --- |
| **Initialization Report** | For each election: the voting period, the registered voters, the ballots counted and the votes of each candidate. It is made before voting, so the counts are zero. | **Generate Initialization Report** on the **Tally** tab. See [Initialization report](11-admin_portal_tutorials_initialization-report.md). | Yes |
| **Electoral Results** | For each contest, area and channel: the participation, the valid, blank and invalid votes, and the votes and winning position of each candidate. | The tally ceremony makes it. See [Get the Results](../03-procedures/07-results.md). | Yes |
| **Participation Report** | The number of voters, the number of voters that voted, the votes and the participation, for the election event or one election. | **Generate** on the **Reports** tab. | Yes |
| **Activity Logs** | The electoral log of the election event: each record with its date, kind, user id and full message. | **Generate** on the **Reports** tab, or the export button on the **Logs** tab. See [Election logs](19-admin_portal_tutorials_election-logs.md). | No. The records of the votes contain the identifier, the IP address and the country of each voter. |
| **Ballot Images** | One image of each decrypted ballot, for each contest and area. | **Generate Ballot Images** in the results of a tally, only if your platform has the ballot images export. | No. In an area with few voters, a person can find how a voter voted. |
| **Ballot Receipt** | The ballot ID of one voter and a link to find the ballot. It contains no vote. | The voter downloads it after casting the vote. | Only the voter |
| **Manual Verification** | A sign-in link for one voter. | An action on the voter. | No |
| **Credentials** | The voter information letter, with the username and the password of one voter. | An action on the voter. | No |

The **Report hash** on a report identifies the moment when the platform made the report. It does
not cover the content of the report.

To select the documents for candidates and observers after an election, see
[Prepare the Audit Pack](../03-procedures/08-audit-pack.md).

### Voters Turnout

**Note:** To display male and female percentages, the sex attribute must have been added. Check out [how to add the sex attribute](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

## Voter Fields in Reports

Reports that render one identified voter at a time, such as voter information letters and manual
verification reports, can print any of the voter's fields. Use `user.<attribute>` for a user
attribute, for example `{{user.ward}}` or `{{user.school_support_choice}}`, and `user.first_name`,
`user.last_name`, `user.username` or `user.email` for the standard fields. Aggregate reports do
not receive voter fields.

Voters often leave some fields empty. The `concat` helper joins the fields that are present and
skips the rest, `format_pattern` formats values such as postal codes, and `upper` uppercases them:

```handlebars
<p>{{concat user.first_name user.middle_names user.last_name}}</p>
<p>{{concat user.corr_unit user.corr_street_number user.corr_street}}</p>
<p>{{concat user.corr_city user.corr_province (format_pattern user.corr_postal_code "### ###")}}</p>
<p>{{upper user.corr_country_name}}</p>
```

See [Voter variables](../02-reference/02-election-event/13-02-election_management_election-event_templates.md#voter-variables)
for every variable and helper, including the fixed fields of the voter information letter.

## Secret Voter Fields in Reports

Secret voter fields are available only to reports that render one identified voter at a time.
Current examples are voter information letters and manual verification reports. Aggregate reports
such as turnout, election activity, tally, and results do not receive a voter object and cannot use
secret fields.

The assigned template must both reference and declare every secret field it uses. For example:

```json
{
  "secret_attribute_names": ["customerReference"],
  "document": "<p>Reference: {{user.customerReference}}</p>"
}
```

At generation time, Step validates the declaration against the election event's current Keycloak
User Profile and decrypts only the declared names for the report's voter. The operator needs
`voter-secret-attribute-read` in addition to the normal report permission. An undeclared, removed,
or unreadable secret causes generation to fail rather than exposing its stored representation.

See [Secret Voter Variables](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables)
for variable shapes and declaration rules.

## Downloading Secret-Bearing or Encrypted Reports

Secret-bearing files use private storage. Downloading one requires
`voter-secret-attribute-read` as well as the ordinary document permissions, even when the file
was generated by another administrator.

Secret classification and password encryption are separate: declaring a secret does not force
every report to be encrypted. Reports retain their configured encryption policy; voter information
letters retain their existing password-protected PDF behavior.

For a newly generated password-encrypted report, download it from **Reports** or the completed
entry in **Tasks**. The Admin Portal opens the existing report-password dialog; retrieving the
password requires `document-password-read` and, for a secret-bearing document,
`voter-secret-attribute-read`. The password belongs to that generated document, so changing the
report configuration later does not change an earlier file's password. Older encrypted files
without a stored document password still show decryption instructions; use the password supplied
when they were generated.

Email and SMS are delivery messages, not password-encrypted report files. Their bodies remain
readable by the recipient, including declared secret fields. See
[test-only Console delivery](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#test-only-console-delivery)
before testing a template containing credentials.
