---
id: admin_portal_tutorials_create_and_set_google_meet_credentials
title: Create and set google meet credentials
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

Create google meet credentials and set them in the Admin portal.

**Note:** This configuration must be done via the google cloud console.

## Configuration Steps

## Google Cloud Console Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one. You must have superadmin access to
   set up the domain-wide delegation later on.
3. Enable the Google Calendar API.
4. Navigation menu > APIs and services > Credentials.
5. Create a Service account.
6. Edit > Keys > Add key. Then donwload the json file.
7. Add authorized domains for your application.
8. Paste the json file containing the Service Account key into Settings > Integrations >
   Google Calendar Service Account key.

## Set up domain-wide delegation for a service account

1. Follow the [steps in this link](https://developers.google.com/workspace/guides/create-credentials#optional_set_up_domain-wide_delegation_for_a_service_account)
2. Add the scopes for the authorized client ID: https://www.googleapis.com/auth/calendar, https://www.googleapis.com/auth/calendar.events
3. Paste the authorized email address into Settings > Integrations > Google Calendar 
   Authentication Email. **Note:** This  email will appear as the organizer of the meeting.
