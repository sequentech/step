---
sidebar_position: 2
---

# Boards

## POST `/boards`

Creates a new board.

- Request schema: `CreateBoardRequest` (see `schemas.md`)
- Response schema: `BoardResponse` (see `schemas.md`)

## GET `/boards`

Lists all boards.

- Response schema: `BoardsListResponse` (see `schemas.md`)

## GET `/boards/:board`

Fetches a board by name.

- Path params:
  - `board`: board name
- Response schema: `BoardResponse` (see `schemas.md`)
