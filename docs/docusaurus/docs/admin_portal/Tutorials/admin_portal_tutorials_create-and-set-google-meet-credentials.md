---
id: admin_portal_tutorials_create_and_set_google_meet_credentials
title: Create and set google meet credentials
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

Create and set google meet credentials

**Note:** This configuration must be done via the google cloud console.

## Configuration Steps

## Google Cloud Console Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the Google Calendar API
4. Navigation menu > APIs and services > Credentials
5. Create a Service account.
6. Edit > Keys > Add key. Then donwload the json file.
7. Add authorized domains for your application
8. Paste the client secret json file into Settings > Google meet.
