<!--
SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Sequent end-points for integration with Datafix´s VoterView

This document intends to define the Headers, Body and Errors of the Sequent´s end-points for the integration with VoterView.

------------------------------------------------------------------------------------------


#### Sequent internal implementation:

- The client will send Voter IDs to identify the voter as username in sequent´s system.
- In Sequent´s system the Areas and sub areas are represented as a tree. At least one is required to create a new voter.
- DELETE voter will actually mark the voter as disabled.
- Full requirements: https://sequentech.atlassian.net/browse/SO-43

##### Datafix ID:

- The <code>datafix:id</code> will be stored in the annotations of the election events that belong to datafix.
- Datafix should have as many ids as election events are configured for them.
- Datafix should set the header field <code>event-id</code> for that.


##### Authetication:

- Authetication will be managed through client-id and client-secret sent in the header´s request.
- Then the Senquent endpoint will use these credentials to request and handle the access token to keycloak.
- This client-id must be configured in Keycloak with it´s allowed roles, in similar fashion to service-account.

##### PIN:

    PIN is the same as sequent´s password and it should be configurable (by election-event), usually it´s 16 numeric digits.
    PIN can be generated as either ( this should be configurable):
        - The voter password.
        - A combination of Voter ID + password.
        - The regular expression to generate the PIN should be configurable
    If voter is disabled, do not generate a PIN

#### Annotations content:

    DATAFIX: Json {
        id: String (ID datafix)
        password_policy: Json {
            base: Enum (ID_PLUS_PSW/PSW_ONLY)
            size: number (The size of the pass)
            characters: Enum (ALPHANUMERIC/NUMERIC)
        }
    }

------------------------------------------------------------------------------------------

#### Headers

<details>
 <summary>Headers</summary>

 ##### Headers for all the endpoints (will be provided by Sequent):

> | name            |  type     | data type | description                                                                                 |
> |-----------------|-----------|-----------|---------------------------------------------------------------------------------------------|
> | authorization   |  required | string    | Client ID and Client secret respectively. Format e.g.: `"authorization":"123456:987654"`    |
> | tenant-id       |  required | string    | Realm id for keycloak´s endpoint.                                                           |
> | event-id        |  required | string    | Unique for each election event. Matches Datafix ID in election event annotations.           |


</details>

#### Delete Voter

<details>
 <summary><code>POST</code> <code><b>/api/datafix/delete-voter</b></code> <code>(Deletes a voter)</code></summary>


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
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>

#### Add Voter

<details>
 <summary><code>POST</code> <code><b>/api/datafix/add-voter</b></code> <code>(Adds a new voter)</code></summary>
 

##### Parameters

> | name        |  type     | data type               | description                                                           |
> |-------------|-----------|-------------------------|-----------------------------------------------------------------------|
> | voter_id    |  required | string                  | Voter username/id (unique)                                            |
> | ward        |  required | string                  | Ward (area)                                                           |
> | schoolboard |  optional | string                  | Schoolboard (area) (Can be null or empty)                             |
> | poll        |  optional | string                  | Poll (area) (Can be null or empty)                                    |
> | birthdate   |  optional | date                    | Voter birthdate (Can be null or empty). ISO 8601 format YYYY-MM-DD    |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"code":"200","message":"Success"}`                  | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>

#### Update Voter information

<details>
 <summary><code>POST</code> <code><b>/api/datafix/update-voter-information</b></code> <code>(Updates voter information)</code></summary>
 

##### Parameters

> | name        |  type     | data type               | description                                                           |
> |-------------|-----------|-------------------------|-----------------------------------------------------------------------|
> | voter_id    |  required | string                  | Voter username/id (unique)                                            |
> | ward        |  required | string                  | Ward (area)                                                           |
> | schoolboard |  optional | string                  | Schoolboard (area) (Can be null or empty)                             |
> | poll        |  optional | string                  | Poll (area) (Can be null or empty)                                    |
> | birthdate   |  optional | date                    | Voter birthdate (Can be null or empty). ISO 8601 format YYYY-MM-DD    |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"code":"200","message":"Success"}`                  | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>

#### Mark voter as voted

<details>
 <summary><code>POST</code> <code><b>/api/datafix/mark-voted</b></code> <code>(Mark a voter as voted)</code></summary>
 

##### Parameters

> | name        |  type     | data type               | description                                                                          |
> |-------------|-----------|-------------------------|--------------------------------------------------------------------------------------|
> | voter_id    |  required | string                  | Voter username/id (unique)                                                           |
> | channel     |  required | string                  | The channel through which the voter casts their vote, e.g. online, in-person, Postal |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"code":"200","message":"Success"}`                  | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>


#### Unmark voter as voted

<details>
 <summary><code>POST</code> <code><b>/api/datafix/unmark-voted</b></code> <code>(Unmark a voter as voted)</code></summary>
 

##### Parameters

> | name        |  type     | data type               | description                                                           |
> |-------------|-----------|-------------------------|-----------------------------------------------------------------------|
> | voter_id    |  required | string                  | Voter username/id (unique)                                            |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"code":"200","message":"Success"}`                  | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>


#### Replace PIN

<details>
 <summary><code>POST</code> <code><b>/api/datafix/replace-pin</b></code> <code>(Replace a voter´s existing PIN. Also used to create it for the first time)</code> </summary>
 

##### Parameters

> | name        |  type     | data type               | description                                                           |
> |-------------|-----------|-------------------------|-----------------------------------------------------------------------|
> | voter_id    |  required | string                  | Voter username/id (unique)                                            |


##### Responses

> | http code     | content-type                      | response                                              | Asumption               |
> |---------------|-----------------------------------|-------------------------------------------------------|-------------------------|
> | `200`         | `application/json`                | `{"pin":"684400123987"}`                              | Action completed        |
> | `401`         | `application/json`                | `{"code":"401","message":"Unauthorized"}`             | Incorrect auth headers  |
> | `400`         | `application/json`                | `{"code":"400","message":"Bad Request"}`              | Incorrect request       |
> | `404`         | `application/json`                | `{"code":"404","message":"Not found"}`                | Voter does not exist    |
> | `500`         | `application/json`                | `{"code":"500","message":"Internal Server Error"}`    | Internal Server Error   |

</details>

