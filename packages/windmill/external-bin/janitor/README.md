<!--
SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->
# janitor

Janitor is a conversion tool. It converts a folder with the Miru OCF files and an excel file
into an election event configuration that can be imported through the admin-portal.
 
## Installation

Before using it, you need to configure the python environment. In Ubuntu, run this command (as a non-root user):

```bash
cd janitor
source ./setup.sh
```

## Use

The script has two inputs, a folder with the OCF files, and an excel (.xls) file.
You can use the example files hereby included:

```bash
cd janitor
python3 ./run.py import-data import-data/10-11-2024-field-test-preparations.xlsx
```

The output files are:
- `output/election-event.zip`. This is the election event zip that you
	can import in the admin portal.
- `admins.csv`. CSV to be imported to configure the admin users, including sbei users.
- `sbei_*.p12`. Encrypted files with the keys to be used by sbei users to sign the transmissions.
  
The tool also prints some output to provide feedback on the process.

## Extract ezip

After downloading the exported election event as a zip, with a name and with the password provided
by the admin portal, extract the ezip with this command (replace the name/password):

	openssl enc -aes-256-cbc -d -in <event-filename>.ezip  -out <event-filename>.zip -pass pass:"<pass>" -md md5

## tenant configuration

You can change the tenant id in the file `janitor/config/baseConfig.json`.
