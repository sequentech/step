---
sidebar_position: 1
---

# B4 Bulletin Board API

B4 is the bulletin board service.

## Base URL

The service listens on the address configured by `B4_BIND`.

## Authentication

JWT authentication is enabled.

All endpoints require a valid token with **trustee** role.

## CORS

CORS behavior is controlled by `B4_ALLOWED_ORIGINS`.

In production it cannot be `*`.

## Data model

- **Board**: a named container for messages.
- **Message**: metadata stored in the database; content is stored inline or in S3.

## Endpoints

- Boards: see `boards.md`
- Single-board messages: see `messages-single.md`
- Multi-board messages: see `messages-multi.md`
- Shared schemas: see `schemas.md`
