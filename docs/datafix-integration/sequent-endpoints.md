<!--
SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Sequent end-points for integration with Datafix´s VoterView

This document intends to define the Headers, Body and Errors of the Sequent´s end-points for the integration with VoterView.
Key requirements:
- Authetication will be managed through client-id and client-secret
- The client will send Voter IDs to identify the voter as username in sequent´s system.
- PIN (same as sequent´s password) should be configurable (by Area?), usually 16 digits.
- In Sequent´s system the Areas and sub areas are represented as a tree... TODO
- DELETE voter will actually mark the voter as disabled.
- A unique Id <code>datafix:id</code> will be stored in the annotations of the election events that belong to datafix.
- Full requirements: https://sequentech.atlassian.net/browse/SO-43
------------------------------------------------------------------------------------------

#### Delete Voter

<details>
 <summary><code>POST</code> <code><b>/api/datafix/delete-voter</b></code> <code>(Deletes a voter)</code></summary>

##### Headers

> | name      |  type     | data type               | description                                                                       |
> |-----------|-----------|-------------------------|-----------------------------------------------------------------------------------|
> | cli_id    |  required | string                  | Client ID                                                                         |
> | cli_sec   |  required | string                  | Client secret                                                                     |
> | tenant_id |  required | string                  | Realm id for keycloak                                                             |
> | event_id  |  required | string                  | To identify the election event. Matches Datafix ID in election event annotations  |


##### Parameters

> | name      |  type     | data type               | description                                                           |
> |-----------|-----------|-------------------------|-----------------------------------------------------------------------|
> | voter_id  |  required | string                  | Voter username/id (unique) to be deleted                              |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"code":"200","message":"Success"}`                  | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"InternalServerError"}`      | Internal Server Error   |

</details>
