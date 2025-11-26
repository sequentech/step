<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Load Testing Tool

This tool is a Python-based load testing utility that performs various actions for load testing your application. It can:

-   Generate voters into a CSV file.
-   Duplicate vote records in a cast_votes table.
-   Generate applications in different states.
-   Generate activity logs.

All configuration is managed via a `config.json` file placed in your working directory.

----------

## Features

-   **generate-voters:** Creates a CSV file with random voter records.
-   **duplicate-votes:** Duplicates vote entries in a PostgreSQL database.
-   **generate-applications:** Generates applications with configurable status and verification type.
-   **generate-activity-logs:** (Stub) Generates activity logs.

----------

## Installation

### 1. Clone the Repository

Clone or download the repository to your local machine.

### 2. Install Python Dependencies

Ensure you have Python 3.7+ installed, then run:

`pip3 install -r requirements.txt` 

Your `requirements.txt` should include packages like:

`Faker==13.3.4
psycopg2==2.9.10
openpyxl==3.1.5
pyzipper==0.3.6
python-dotenv==1.0.1
pybars3==0.9.7` 

----------

## Configuration

Create a `config.json` file in your working directory (the directory you pass to the tool with the `--working-directory` option). Below is an example configuration:

```json
{
  "election_event_json_file": "export_election_event-012597c9-3940-4888-8a8e-cc6bb82e2edf.json",
  "realm_name": "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-012597c9-3940-4888-8a8e-cc6bb82e2edf",
  "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
  "election_event_id": "012597c9-3940-4888-8a8e-cc6bb82e2edf",
  "generate_voters": {
    "csv_file_name": "generated_users.csv",
    "fields": [
      "username",
      "last_name",
      "first_name",
      "middleName",
      "dateOfBirth",
      "sex",
      "country",
      "embassy",
      "clusteredPrecinct",
      "overseasReferences",
      "area_name",
      "authorized-election-ids",
      "password",
      "email",
      "password_salt",
      "hashed_password"
    ],
    "excluded_columns": ["password", "password_salt", "hashed_password"],
    "email_prefix": "testsequent2025",
    "domain": "mailinator.com",
    "sequence_email_number": true,
    "sequence_start_number": 0,
    "voter_password": "Qwerty1234!",
    "password_salt": "sppXH6/iePtmIgcXfTHmjPS2QpLfILVMfmmVOLPKlic=",
    "hashed_password": "V0rb8+HmTneV64qto5f0G2+OY09x2RwPeqtK605EUz0=",
    "overseas_reference": "B",
    "min_age": 18,
    "max_age": 90
  },
  "duplicate_votes": {
    "row_id_to_clone": "a2d4b909-ea96-4de8-b1c1-57a392d1d5b3"
  },
  "generate_applications": {
    "applicant_data": {
      "email": "",
      "country": "",
      "embassy": "",
      "lastName": "",
      "username": "",
      "firstName": "",
      "dateOfBirth": "",
      "termsOfService": "termsOfServiceText",
      "emailAndOrMobile": "sequent.read-only.mobile-number;email",
      "sequent.read-only.id-card-type": "driversLicense",
      "sequent.read-only.mobile-number": "",
      "sequent.read-only.id-card-number": "",
      "sequent.read-only.id-card-number-validated": "VERIFIED"
    },
    "annotations": {
      "mismatches": 5,
      "session_id": null,
      "credentials": [
        {
          "id": null,
          "type": "password",
          "userLabel": null,
          "secretData": "{\"value\":\"huEJOcUD36x8KrK/g2PH2Fx4GaRFo2msPn9xiAP3pi4=\",\"salt\":\"e0CPyCoyeBBS972HhuUppQ==\",\"additionalParameters\":{}}",
          "createdDate": null,
          "credentialData": "{\"hashIterations\":600000,\"algorithm\":\"pbkdf2-sha256\",\"additionalParameters\":{}}",
          "passwordSecretData": {
            "salt": "e0CPyCoyeBBS972HhuUppQ==",
            "value": "huEJOcUD36x8KrK/g2PH2Fx4GaRFo2msPn9xiAP3pi4=",
            "additionalParameters": {}
          },
          "passwordCredentialData": {
            "algorithm": "pbkdf2-sha256",
            "hashIterations": 600000,
            "additionalParameters": {}
          }
        },
        {
          "id": null,
          "type": "message-otp",
          "userLabel": null,
          "secretData": "{\"nothing\":\"nothing\"}",
          "createdDate": 1739168363012,
          "otpsecretData": {
            "nothing": "nothing"
          },
          "credentialData": "{\"setup\":true}",
          "otpcredentialData": {
            "setup": true
          }
        }
      ],
      "verified_by": null,
      "verified_by_role": null,
      "fields_match": {
        "embassy": false,
        "lastName": false,
        "firstName": false,
        "middleName": false,
        "dateOfBirth": false
      },
      "rejection_reason": "",
      "unset-attributes": "email,sequent.read-only.mobile-number,sequent.read-only.id-card-number-validated",
      "rejection_message": null,
      "search-attributes": "firstName,middleName,lastName,embassy,dateOfBirth",
      "update-attributes": "sequent.read-only.id-card-number-validated,email,sequent.read-only.mobile-number,emailAndOrMobile,sequent.read-only.id-card-number,sequent.read-only.id-card-type",
      "manual_verify_reason": null
    }
  }
}
``` 

Customize the configuration as needed for your environment.

----------

## Usage

The tool is run via the command line and supports several subcommands. A global argument, **`--working-directory`**, specifies the directory containing the `config.json` file and any required files.

### Global Argument

-   **`--working-directory`**: Path to the directory with `config.json` and any required files.

### Subcommands

1.  **generate-voters**  
    Generates a CSV file containing fake voter data.  
    **Usage:**
    
    `python load_tool.py generate-voters --working-directory "/path/to/dir" --num-users 1000` 
    
    -   **`--num-users`**: Number of voter records to generate.
2.  **duplicate-votes**  
    Duplicates vote entries in the database.  
    **Usage:**

    `python load_tool.py duplicate-votes --working-directory "/path/to/dir" --num-votes 50000` 
    
    -   **`--num-votes`**: Number of vote records to duplicate.
3.  **generate-applications**  
    Generates applications with configurable status and verification type.  
    **Usage:**
    
    `python load_tool.py generate-applications --working-directory "/path/to/dir" --num-applications 100000 --status REJECTED --type MANUAL` 
    
    -   **`--num-applications`**: Number of applications to generate.
    -   **`--status`**: Application status. Choices: `"PENDING"`, `"REJECTED"`, or `"ACCEPTED"`. Defaults to `"PENDING"`.
    -   **`--type`**: Application verification type. Choices: `"AUTOMATIC"` or `"MANUAL"`. If not provided, a default is chosen based on status.
4.  **generate-activity-logs**  
    Stub for generating activity logs.  
    **Usage:**
    
    `python load_tool.py generate-activity-logs --working-directory "/path/to/dir"` 
    
_Note:_ If your directory path contains spaces, enclose it in quotes.

----------

## Environment Variables

For the **duplicate-votes** action (and any other actions that require database connections), ensure that the following environment variables are set:

-   `KC_DB`
-   `KC_DB_USERNAME`
-   `KC_DB_PASSWORD`
-   `KC_DB_URL_HOST`
-   `KC_DB_URL_PORT`
-   `HASURA_PG_DBNAME`
-   `HASURA_PG_USER`
-   `HASURA_PG_PASSWORD`
-   `HASURA_PG_HOST`
-   `HASURA_PG_PORT`

Set these in your shell or via a `.env` file if you are using `python-dotenv`.

----------

## Example

To generate 100 fake voter records, run:

`python load_tool.py generate-voters --working-directory "/workspaces/step/packages/windmill/external-bin/janitor/scripts" --num-users 100` 

To duplicate 50,000 vote records, run:

`python load_tool.py duplicate-votes --num-votes 50000 --election-event-id whatever` 

To generate 100,000 applications with status REJECTED and verification type MANUAL, run:


`python load_tool.py generate-applications --working-directory "/workspaces/step/packages/windmill/external-bin/janitor/scripts" --num-applications 100000 --status REJECTED --type MANUAL`
