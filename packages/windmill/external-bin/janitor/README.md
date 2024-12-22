<!--
SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->
# janitor

Janitor is a conversion tool. It converts a zip with the Miru OCF files and an excel file
into an election event configuration that can be imported through the admin-portal.
 
## Installation

Before using it, you need to configure the python environment. In Ubuntu, run this command (as a non-root user):

```bash
cd janitor
source ./setup.sh
```

## Use

The script has two inputs, a zip with the OCF files, and an excel (.xls) file.
You can use the example files hereby included:

```bash
cd janitor
python3 ./run.py import-data/OCF-0-20241122.zip 17-12-2024-parameters-reports.xlsx
```

The output files are:
- `output/election-event.zip`. This is the election event zip that you
	can import in the admin portal.
- `admins.csv`. CSV to be imported to configure the admin users, including sbei users.
- `sbei_*.p12`. Encrypted files with the keys to be used by sbei users to sign the transmissions.
  
The tool also prints some output to provide feedback on the process.

## tenant configuration

You can change the tenant id in the file `janitor/config/baseConfig.json`.
