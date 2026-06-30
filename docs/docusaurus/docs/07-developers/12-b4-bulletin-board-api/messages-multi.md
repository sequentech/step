---
sidebar_position: 4
---

# Messages (multi board)

These endpoints allow operating on multiple boards with a single request.

## POST `/boards/messages/multi/get`

Fetch messages from multiple boards.

- Request schema: `GetMessagesMultiRequest` (see `schemas.md`)
- Response schema: `GetMessagesMultiResponse` (see `schemas.md`)

## POST `/boards/messages/multi/initiate`

Initiate creation of multiple messages across boards.

- Request schema: `InitiateMessagesMultiRequest` (see `schemas.md`)
- Response schema: `InitiateMessagesMultiResponse` (see `schemas.md`)

## POST `/boards/messages/multi/confirm`

Confirm creation of multiple messages across boards.

- Request schema: `ConfirmMessagesMultiRequest` (see `schemas.md`)
- Response schema: `ConfirmMessagesMultiResponse` (see `schemas.md`)
