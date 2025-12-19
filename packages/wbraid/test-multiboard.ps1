# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Test script for multi-board bulletin board service

$baseUrl = "http://127.0.0.1:3000"

Write-Host "`n=== Testing Multi-Board Bulletin Board Service ===" -ForegroundColor Cyan

# Test 1: Create boards
Write-Host "`n1. Creating boards..." -ForegroundColor Yellow
$board1 = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards" -ContentType "application/json" -Body '{"name":"election-2024"}'
Write-Host "Created board: $($board1.name)" -ForegroundColor Green

$board2 = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards" -ContentType "application/json" -Body '{"name":"test-board"}'
Write-Host "Created board: $($board2.name)" -ForegroundColor Green

# Test 2: List boards
Write-Host "`n2. Listing all boards..." -ForegroundColor Yellow
$boards = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards"
Write-Host "Found $($boards.Count) boards:" -ForegroundColor Green
$boards | ForEach-Object { Write-Host "  - $($_.name) (created: $($_.created_at))" }

# Test 3: Get specific board
Write-Host "`n3. Getting board details..." -ForegroundColor Yellow
$board = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards/election-2024"
Write-Host "Board: $($board.name), Status: $($board.status)" -ForegroundColor Green

# Test 4: Post small message (inline)
Write-Host "`n4. Posting small message to election-2024..." -ForegroundColor Yellow
$smallData = [System.Text.Encoding]::UTF8.GetBytes("Hello from election-2024!")
$initResponse = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards/election-2024/messages/initiate" -ContentType "application/json" -Body "{`"size`":$($smallData.Length)}"
Write-Host "Message ID: $($initResponse.message_id), Should upload to S3: $($initResponse.should_upload)" -ForegroundColor Green

$confirmBody = @{
    data = @($smallData)
} | ConvertTo-Json
$confirmResponse = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards/election-2024/messages/$($initResponse.message_id)/confirm" -ContentType "application/json" -Body $confirmBody
Write-Host "Confirmed: $($confirmResponse.success)" -ForegroundColor Green

# Test 5: Post another message to different board
Write-Host "`n5. Posting message to test-board..." -ForegroundColor Yellow
$data2 = [System.Text.Encoding]::UTF8.GetBytes("Hello from test-board!")
$initResponse2 = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards/test-board/messages/initiate" -ContentType "application/json" -Body "{`"size`":$($data2.Length)}"
$confirmBody2 = @{
    data = @($data2)
} | ConvertTo-Json
$confirmResponse2 = Invoke-RestMethod -Method Post -Uri "$baseUrl/boards/test-board/messages/$($initResponse2.message_id)/confirm" -ContentType "application/json" -Body $confirmBody2
Write-Host "Message ID: $($initResponse2.message_id)" -ForegroundColor Green

# Test 6: List messages from each board
Write-Host "`n6. Listing messages from election-2024..." -ForegroundColor Yellow
$messages1 = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards/election-2024/messages"
Write-Host "Found $($messages1.messages.Count) messages" -ForegroundColor Green
$messages1.messages | ForEach-Object { Write-Host "  Message ID: $($_.id), Size: $($_.size) bytes" }

Write-Host "`n7. Listing messages from test-board..." -ForegroundColor Yellow
$messages2 = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards/test-board/messages"
Write-Host "Found $($messages2.messages.Count) messages" -ForegroundColor Green
$messages2.messages | ForEach-Object { Write-Host "  Message ID: $($_.id), Size: $($_.size) bytes" }

# Test 7: Range-based retrieval
Write-Host "`n8. Testing range-based retrieval (last_id=0)..." -ForegroundColor Yellow
$rangeMessages = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards/election-2024/messages?last_id=0"
Write-Host "Found $($rangeMessages.messages.Count) messages with ID > 0" -ForegroundColor Green

# Test 8: Get specific message
Write-Host "`n9. Getting specific message..." -ForegroundColor Yellow
$firstMsgId = $messages1.messages[0].id
$message = Invoke-RestMethod -Method Get -Uri "$baseUrl/boards/election-2024/messages/$firstMsgId"
Write-Host "Retrieved message ID: $($message.message.id)" -ForegroundColor Green
Write-Host "Content type: $($message.message.content_type)" -ForegroundColor Green

Write-Host "`n=== All Tests Passed! ===" -ForegroundColor Cyan
