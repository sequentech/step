<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Load Testing Tool

This tool is a Python-based load testing utility that performs various actions such as generating fake voter data, duplicating votes in a PostgreSQL database, and (in the future) generating applications and activity logs. Configuration for the tool is centralized in a `config.json` file located in a specified working directory.

## Features

- **generate-voters:** Creates a CSV file with random voter records.
- **duplicate-votes:** Duplicates vote entries in a PostgreSQL database.
- **generate-applications:** Stub for generating applications in different states.
- **generate-activity-logs:** Stub for generating activity logs.

## Installation

1. **Clone the Repository**

   Clone or download the repository to your local machine.

2. **Install Python Dependencies**

   Ensure you have Python 3.7+ installed, then run:

   ```bash
   pip3 install -r requirements.txt`` 

Your `requirements.txt` should include packages like:

`Faker==13.3.4
psycopg2==2.9.10
openpyxl==3.1.5
pyzipper==0.3.6
python-dotenv==1.0.1
pybars3==0.9.7` 

## Configuration

Create a `config.json` file in your working directory (the directory you pass to the tool with the `--working-directory` option). Below is an example configuration:

`{
  "json_file": "export_election_event-dfaa3865-3e3f-4cb3-ad39-7637578d691a.json",
  "csv_file": "generated_users.csv",
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
  "min_age": 18,
  "max_age": 90,
  "overseas_reference": "B",
  "duplicate_votes": {
    "realm_name": "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-deb5b3ed-2ba6-4aa4-92d1-59df91354f51",
    "target_row_count": 50000,
    "row_id_to_clone": "6dd4b66f-e5bc-4950-94bb-9628240c7cf3"
  }
}` 

Customize the settings as needed for your environment and use cases.

## Usage

The tool is run via the command line and supports several subcommands. A global argument, `--working-directory`, specifies the directory containing the `config.json` file, `export_election_event_{id}.json` file (json_file in the config.json) and any other required files.

### Global Argument

-   `--working-directory`: Path to the directory with `config.json` and any required files.

### Subcommands

#### generate-voters

Generates a CSV file containing fake voter data.

Usage:

`python load_tool.py generate-voters --working-directory "/path/to/dir" --num-users 1000` 

-   `--num-users`: Number of voter records to generate.

#### duplicate-votes

Duplicates vote entries in the database.

Usage:

`python load_tool.py duplicate-votes --working-directory "/path/to/dir"` 

#### generate-applications

Stub for generating applications in different states.

Usage:

`python load_tool.py generate-applications --working-directory "/path/to/dir"` 

#### generate-activity-logs

Stub for generating activity logs.

Usage:

`python load_tool.py generate-activity-logs --working-directory "/path/to/dir"` 

_Note:_ If your directory path contains spaces, enclose it in quotes.

## Environment Variables

For the **duplicate-votes** action, ensure that your environment variables for database connections are set. The tool expects:

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

You can set these in your shell or via a `.env` file if you are using `python-dotenv`.

## Example

To generate 100 fake voter records, run:

`python load_tool.py generate-voters --working-directory "/workspaces/step/packages/windmill/external-bin/janitor/scripts" --num-users 100` 

## License