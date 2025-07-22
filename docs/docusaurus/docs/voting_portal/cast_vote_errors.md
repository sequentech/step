# Cast Vote Errors in ReviewScreen.tsx

This document provides a comprehensive overview of all error cases where `setErrorMsg` is used in the ReviewScreen.tsx component, explaining the possible reasons for each error and the functions/queries that could trigger them.

## Error Categories

### 1. Network and Data Fetching Errors

#### NETWORK_ERROR
**Translation**: "There was a network problem. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When `ApolloError` with `networkError` occurs during `INSERT_CAST_VOTE` mutation
- `ReviewScreen.onError()` - When GraphQL query `GET_ELECTIONS` fails with network error

**Possible Reasons**:
- Internet connectivity issues
- Server is unreachable
- Network timeout
- DNS resolution problems
- Firewall blocking the request

#### UNABLE_TO_FETCH_DATA
**Translation**: "There was a problem fetching the data. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ReviewScreen.onError()` - When GraphQL query `GET_ELECTIONS` fails without network error (e.g., GraphQL errors)

**Possible Reasons**:
- GraphQL query syntax errors
- Server-side validation errors
- Authentication/authorization issues
- Database connectivity problems
- Invalid query parameters

### 2. Cast Vote Operation Errors

#### CAST_VOTE
**Translation**: "There was an unknown error while casting the vote. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When `INSERT_CAST_VOTE` mutation returns errors in result.errors
- `useTryInsertCastVote()` - Generic fallback error when other specific error conditions don't match

**Possible Reasons**:
- General mutation execution failure
- Unexpected server response
- Data validation errors on server side
- Unknown server-side exceptions

#### CAST_VOTE_TIMEOUT
**Translation**: "Timeout error to cast the vote. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When GraphQL error code is `UNEXPECTED` and internal error message is `TIMEOUT_ERROR`

**Possible Reasons**:
- Server processing timeout
- Database query timeout
- Network request timeout
- Heavy server load causing slow response

#### INTERNAL_ERROR
**Translation**: "There was an internal error while casting the vote. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When error message includes "internal error"

**Possible Reasons**:
- Backend server panic or crash
- Unhandled server-side exceptions
- Database internal errors
- Memory or resource exhaustion on server

### 3. Authentication and Session Errors

#### REAUTH_FAILED
**Translation**: "Authentication failed. Please try again or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.storeBallotDataAndReauth()` - When `reauthWithGold()` throws an exception during golden user re-authentication

**Possible Reasons**:
- Invalid authentication credentials
- Authentication service unavailable
- Token expiration during re-auth process
- User cancelled authentication dialog
- Authentication server errors

#### SESSION_EXPIRED
**Translation**: "Your session has expired. Please try again from the beginning."

**Triggering Functions/Queries**:
- `ReviewScreen.getBallotDataFromSessionStorage()` - When stored session data expiration time has passed

**Possible Reasons**:
- Session timeout (5-minute expiration for security)
- System clock changes
- User took too long to complete authentication
- Browser was inactive for extended period

#### SESSION_STORAGE_ERROR
**Translation**: "Session storage is not available. Please try again or contact support."

**Triggering Functions/Queries**:
- `ActionButtons.storeBallotDataAndReauth()` - When storing ballot data to sessionStorage fails
- `ReviewScreen.getBallotDataFromSessionStorage()` - When required session storage keys are missing

**Possible Reasons**:
- Browser doesn't support sessionStorage
- Storage quota exceeded
- Private/incognito mode restrictions
- Browser security settings blocking storage
- Storage corruption or unavailability

### 4. Ballot Data Processing Errors

#### PARSE_BALLOT_DATA_ERROR
**Translation**: "There was an error parsing the ballot data. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ReviewScreen.getBallotDataFromSessionStorage()` - When `JSON.parse()` fails on stored session data

**Possible Reasons**:
- Corrupted session storage data
- Invalid JSON format in stored data
- Data tampering or modification
- Encoding/decoding issues
- Browser storage corruption

#### NOT_VALID_BALLOT_DATA_ERROR
**Translation**: "Ballot data is not valid. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ReviewScreen.getBallotDataFromSessionStorage()` - When parsed ballot data is missing required fields (ballotId, electionId, ballot)

**Possible Reasons**:
- Incomplete ballot data stored
- Data structure changes between versions
- Required fields missing from stored data
- Data corruption during storage/retrieval
- Programming errors in data structure

#### TO_HASHABLE_BALLOT_ERROR
**Translation**: "Error converting to hashable ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When `toHashableBallot()` or `toHashableMultiBallot()` throws an exception

**Possible Reasons**:
- Invalid ballot structure for hashing
- Missing required ballot fields
- Ballot data format incompatibility
- Cryptographic processing errors
- Contest selection validation failures

### 5. Specific Cast Vote Error Codes

The system also handles specific GraphQL error codes with dynamic error messages in the format `CAST_VOTE_${errorCode}`:

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When GraphQL error has specific error code in extensions

**Common Error Codes and Reasons**:
- `CAST_VOTE_Unauthorized` - User not authorized to vote
- `CAST_VOTE_ElectionEventNotFound` - Election event doesn't exist
- `CAST_VOTE_CheckStatusFailed` - Election closed or outside voting period
- `CAST_VOTE_InsertFailedExceedsAllowedRevotes` - Exceeded revote limits
- `CAST_VOTE_BallotIdMismatch` - Ballot ID doesn't match cast vote

### 6. Ballot Error Types

#### Dynamic Ballot Errors
**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with specific error_type

**Translation**: Uses the error_type from the ballot error directly

**Possible Reasons**:
- Ballot validation failures
- Contest selection errors
- Write-in candidate issues
- Ballot structure problems
- Encoding/decoding errors

## Error Handling Flow

1. **Network Operations**: Errors from GraphQL queries/mutations are caught and categorized
2. **Authentication Flow**: Re-authentication failures during golden user flow
3. **Session Management**: Storage and retrieval errors for ballot data
4. **Ballot Processing**: Conversion and validation errors during ballot preparation
5. **Cast Vote Execution**: Final submission errors with specific error codes

## Debugging Tips

- Check browser console for detailed error messages
- Verify network connectivity and server availability
- Ensure browser supports required features (sessionStorage)
- Check if election is active and user is authorized
- Verify ballot data integrity and format
- Monitor server logs for backend errors

## Related Files

- `packages/voting-portal/src/routes/ReviewScreen.tsx` - Main error handling logic
- `packages/voting-portal/src/translations/en.ts` - Error message translations
- `packages/voting-portal/src/queries/InsertCastVote.ts` - Cast vote mutation
- `packages/voting-portal/src/services/VotingPortalError.ts` - Error type definitions
