---
sidebar_position: 3
---

# Messages (single board)

B4 supports a two-step flow for publishing messages to a board.

The client first calls **initiate** to receive a `message_id` and (for large payloads) a pre-signed S3 URL.

## POST `/boards/:board/messages/initiate`

Initiates message publication.

- Request schema: `InitiateMessageRequest` (see `schemas.md`)
- Response schema: `InitiateMessageResponse` (see `schemas.md`)

If `should_upload` is `true`, upload the message body directly to S3 using `upload_url` and then confirm.

## POST `/boards/:board/messages/:id/confirm`

Confirms message publication.

- Request schema: `ConfirmMessageRequest` (see `schemas.md`)
- Response schema: `ConfirmMessageResponse` (see `schemas.md`)

If `should_upload` was `false`, provide the message bytes in `data`.

## GET `/boards/:board/messages/list`

Lists message metadata.

- Response schema: `ListMessagesResponse` (see `schemas.md`)

## GET `/boards/:board/messages`

Gets messages including pre-signed download URLs when content is stored in S3.

- Query params:
  - `last_id` (optional)
  - `limit` (optional)

- Response schema: `GetMessagesResponse` (see `schemas.md`)

## GET `/boards/:board/messages/:id`

Gets a single message.

- Response schema: `GetMessageResponse` (see `schemas.md`)
