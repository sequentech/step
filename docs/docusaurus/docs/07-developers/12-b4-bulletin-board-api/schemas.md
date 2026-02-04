---
sidebar_position: 5
---

# Schemas

This section lists the primary request/response bodies used by the B4 API.

## Boards

- `CreateBoardRequest`
  - `name: string`

- `BoardResponse`
  - `name: string`
  - `created_at: number`
  - `status: string`

- `BoardsListResponse`
  - `boards: BoardResponse[]`

## Messages (core)

- `Message`
  - `id: string`
  - `timestamp: number`
  - `content_type: ContentType`
  - `size: number`
  - `sender_pk: string`
  - `statement_kind: string`
  - `batch: number`
  - `mix_number: number`

- `ContentType`
  - Inline: `{ "message": "<base64>" }`
  - S3: `{ "key": "<s3_key>" }`

## Single-board message flow

- `InitiateMessageRequest`
  - `size: number`
  - `sender_pk: string`
  - `statement_kind: string`
  - `batch: number`
  - `mix_number: number`

- `InitiateMessageResponse`
  - `message_id: string`
  - `upload_url?: string`
  - `should_upload: boolean`

- `ConfirmMessageRequest`
  - `data?: bytes` (only for inline messages)
  - `sender_pk: string`
  - `statement_kind: string`
  - `batch: number`
  - `mix_number: number`

- `ConfirmMessageResponse`
  - `success: boolean`

- `ListMessagesResponse`
  - `messages: Message[]`

- `GetMessageResponse`
  - `message: Message`
  - `download_url?: string`

- `GetMessagesResponse`
  - `messages: MessageWithUrl[]`

- `MessageWithUrl`
  - All `Message` fields
  - `download_url?: string`

## Multi-board operations

- `BoardMessageRequest`
  - `board: string`
  - `last_id: number`
  - `limit?: number`

- `GetMessagesMultiRequest`
  - `requests: BoardMessageRequest[]`

- `BoardMessagesResponse`
  - `board: string`
  - `messages: MessageWithUrl[]`

- `GetMessagesMultiResponse`
  - `boards: BoardMessagesResponse[]`

- `MessageMetadata`
  - `size: number`
  - `sender_pk: string`
  - `statement_kind: string`
  - `batch: number`
  - `mix_number: number`

- `BoardInitiateRequest`
  - `board: string`
  - `messages: MessageMetadata[]`

- `InitiateMessagesMultiRequest`
  - `requests: BoardInitiateRequest[]`

- `MessageUploadInfo`
  - `message_id: string`
  - `upload_url?: string`
  - `should_upload: boolean`

- `BoardInitiateResponse`
  - `board: string`
  - `uploads: MessageUploadInfo[]`

- `InitiateMessagesMultiResponse`
  - `boards: BoardInitiateResponse[]`

- `MessageConfirmation`
  - `message_id: string`
  - `data?: bytes` (only for inline messages)

- `BoardConfirmRequest`
  - `board: string`
  - `confirmations: MessageConfirmation[]`

- `ConfirmMessagesMultiRequest`
  - `requests: BoardConfirmRequest[]`

- `ConfirmMessagesMultiResponse`
  - `success: boolean`
