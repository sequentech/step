# Cast Vote Errors in ReviewScreen.tsx

This document provides a comprehensive overview of all error cases where `setErrorMsg` is used in the ReviewScreen.tsx component, explaining the possible reasons for each error and the functions/queries that could trigger them.

## Table of Contents

1. [Error Categories](#error-categories)
   1. [Network and Data Fetching Errors](#1-network-and-data-fetching-errors)
   2. [Cast Vote Operation Errors](#2-cast-vote-operation-errors)
   3. [More specific Cast Vote Error Codes](#3-more-specific-cast-vote-error-codes)
   4. [Ballot Data Processing Errors](#4-ballot-data-processing-errors)
   5. [More specific ballot Error Types](#5-more-specific-ballot-error-types)
   6. [Authentication and Session Errors](#6-authentication-and-session-errors)
2. [Debugging Tips](#debugging-tips)
3. [Related Files](#related-files)

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
- `useTryInsertCastVote()` - When `INSERT_CAST_VOTE` mutation returns errors in result.errors

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
- `useTryInsertCastVote()` - Generic fallback error when other specific error conditions don't match, (see [More specific Cast Vote Error Codes](#3-more-specific-cast-vote-error-codes)).

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


### 3. More specific Cast Vote Error Codes

The system also handles specific GraphQL error codes with dynamic error messages in the format `CAST_VOTE_${errorCode}`:

**Triggering Functions/Queries**:
- `useTryInsertCastVote()` - When GraphQL error has specific error code in extensions

**All CastVoteError Codes and Reasons**:

#### CAST_VOTE_VotingChannelNotEnabled
**Translation**: "There was an error while casting the vote: Voting channel not enabled. Please try again later or contact support for assistance."

**Possible Reasons**:
- The voting channel (e.g., internet voting) is disabled for this election
- Configuration issue with voting methods
- Administrative restriction on voting channel

#### CAST_VOTE_AreaNotFound
**Translation**: "There was an error while casting the vote: Area not found. Please try again later or contact support for assistance."

**Possible Reasons**:
- Invalid or non-existent voting area ID
- Database inconsistency with area records
- User assigned to non-existent area

#### CAST_VOTE_ElectionEventNotFound
**Translation**: "The election event could not be found. Please try again later or contact support for assistance."

**Possible Reasons**:
- Election event has been deleted or archived
- Invalid election event ID
- Database connectivity issues

#### CAST_VOTE_ElectoralLogNotFound
**Translation**: "Your voting record could not be found. Please contact support for assistance."

**Possible Reasons**:
- Electoral log not properly initialized
- Database corruption or missing records
- Configuration issues with electoral logging

#### CAST_VOTE_CheckStatusFailed
**Translation**: "Election does not allow casting the vote. Election might be closed, archived or you might be trying to vote outside grace period."

**Possible Reasons**:
- Election is not in active voting period
- Grace period has expired
- Election status is closed or archived
- Voting outside allowed time window

#### CAST_VOTE_CheckStatusInternalFailed
**Translation**: "An internal error occurred while checking election status. Please try again later or contact support for assistance."

**Possible Reasons**:
- Internal server error during status validation
- Database query failures
- Service unavailability

#### CAST_VOTE_CheckPreviousVotesFailed
**Translation**: "An error occurred while checking your voting status. Please try again later or contact support for assistance."

**Possible Reasons**:
- Database error when querying previous votes
- Voter ID validation issues
- System error during vote history lookup

#### CAST_VOTE_CheckRevotesFailed
**Translation**: "You have exceeded the allowed number of revotes. Please try again later or contact support for assistance."

**Possible Reasons**:
- Maximum number of revotes reached
- Revote limit configuration exceeded
- Multiple voting attempts beyond allowed threshold

#### CAST_VOTE_CheckVotesInOtherAreasFailed
**Translation**: "You have voted in another area already. Please try again later or contact support for assistance."

**Possible Reasons**:
- Voter has already cast vote in different area
- Cross-area voting restriction violation
- Data integrity check failure

#### CAST_VOTE_InsertFailed
**Translation**: "An error occurred while recording your vote. Please try again later or contact support for assistance."

**Possible Reasons**:
- Database insertion failure
- Transaction rollback
- Data validation errors during insert

#### CAST_VOTE_InsertFailedExceedsAllowedRevotes
**Translation**: "You have exceeded the revotes limit. Please try again later or contact support for assistance."

**Possible Reasons**:
- Exceeded maximum allowed revotes
- Revote policy violation
- Multiple voting attempts beyond limit

#### CAST_VOTE_CommitFailed
**Translation**: "An error occurred while finalizing your vote. Please try again later or contact support for assistance."

**Possible Reasons**:
- Database transaction commit failure
- Concurrent modification conflicts
- System resource exhaustion

#### CAST_VOTE_GetDbClientFailed
**Translation**: "Database connection error. Please try again later or contact support for assistance."

**Possible Reasons**:
- Database connection pool exhaustion
- Network connectivity to database
- Database server unavailability

#### CAST_VOTE_GetClientCredentialsFailed
**Translation**: "Failed to verify your credentials. Please try again later or contact support for assistance."

**Possible Reasons**:
- Authentication service failure
- Invalid or expired credentials
- OAuth/token validation errors

#### CAST_VOTE_GetAreaIdFailed
**Translation**: "An error occurred verifying your voting area. Please try again later or contact support for assistance."

**Possible Reasons**:
- Area ID lookup failure
- Invalid voter-area mapping
- Database query errors

#### CAST_VOTE_GetTransactionFailed
**Translation**: "An error occurred processing your vote. Please try again later or contact support for assistance."

**Possible Reasons**:
- Database transaction initialization failure
- Connection pool issues
- Resource allocation problems

#### CAST_VOTE_DeserializeBallotFailed
**Translation**: "An error occurred reading your ballot. Please try again later or contact support for assistance."

**Possible Reasons**:
- Invalid ballot format or structure
- Corrupted ballot data
- Version compatibility issues
- JSON/serialization parsing errors

#### CAST_VOTE_DeserializeContestsFailed
**Translation**: "An error occurred reading your selections. Please try again later or contact support for assistance."

**Possible Reasons**:
- Invalid contest data format
- Corrupted selection data
- Contest structure validation failure

#### CAST_VOTE_SerializeVoterIdFailed
**Translation**: "An error occurred processing your voter ID. Please try again later or contact support for assistance."

**Possible Reasons**:
- Voter ID serialization failure
- Invalid voter ID format
- Encoding/decoding errors

#### CAST_VOTE_SerializeBallotFailed
**Translation**: "An error occurred processing your ballot. Please try again later or contact support for assistance."

**Possible Reasons**:
- Ballot serialization failure
- Data structure conversion errors
- Memory allocation issues

#### CAST_VOTE_PokValidationFailed
**Translation**: "Failed to validate your vote. Please try again later or contact support for assistance."

**Possible Reasons**:
- Cryptographic proof validation failure
- Invalid zero-knowledge proofs
- Ballot integrity check failure
- Encryption validation errors

#### CAST_VOTE_BallotSignFailed
**Translation**: "Failed to sign your ballot. Please try again later or contact support for assistance."

**Possible Reasons**:
- Digital signature generation failure
- Cryptographic key issues
- Signing algorithm errors

#### CAST_VOTE_BallotVoterSignatureFailed
**Translation**: "Failed to validate voter signature. Please try again later or contact support for assistance."

**Possible Reasons**:
- Voter signature validation failure
- Invalid voter signing key
- Signature verification errors

#### CAST_VOTE_UuidParseFailed
**Translation**: "An error occurred processing your request. Please try again later or contact support for assistance."

**Possible Reasons**:
- Invalid UUID format
- UUID parsing errors
- Malformed identifier strings

#### CAST_VOTE_BallotIdMismatch
**Translation**: "The ballot id does not match with the cast vote."

**Possible Reasons**:
- Ballot ID doesn't match computed hash
- Ballot content tampering detected
- Hash verification failure
- Data integrity violation

#### CAST_VOTE_UnknownError
**Translation**: "An unknown error occurred while casting the vote. Please try again later or contact support for assistance."

**Possible Reasons**:
- Unhandled exception or error condition
- Unexpected system state
- Generic fallback for unclassified errors


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
- `ActionButtons.castBallotAction()` - When `toHashableBallot()` or `toHashableMultiBallot()` throws an exception. This is a generic fallback error when other specific error conditions don't match ([More specific ballot Error Types](#5-more-specific-ballot-error-types)).

**Possible Reasons**:
- Invalid ballot structure for hashing
- Missing required ballot fields
- Ballot data format incompatibility
- Cryptographic processing errors
- Contest selection validation failures

### 5. More specific ballot Error Types

**All BallotError Codes and Reasons**:

#### PARSE_ERROR
**Translation**: "There was an error parsing the ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with error_type `PARSE_ERROR`

**Possible Reasons**:
- Invalid JSON format in ballot data
- Malformed ballot structure
- Syntax errors in ballot content
- Corrupted ballot data during transmission
- Incompatible ballot format version

#### DESERIALIZE_AUDITABLE_ERROR
**Translation**: "There was an error deserializing the auditable ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with error_type `DESERIALIZE_AUDITABLE_ERROR`

**Possible Reasons**:
- Failed to deserialize auditable ballot contests
- Invalid auditable ballot structure
- Missing required fields in auditable ballot
- Data type mismatches during deserialization
- Version compatibility issues with auditable ballot format

#### DESERIALIZE_HASHABLE_ERROR
**Translation**: "There was an error deserializing the hashable ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with error_type `DESERIALIZE_HASHABLE_ERROR`

**Possible Reasons**:
- Failed to deserialize hashable ballot structure
- Invalid hashable ballot format
- Cryptographic data corruption in hashable ballot
- Missing cryptographic components
- Incompatible hashable ballot version

#### CONVERT_ERROR
**Translation**: "There was an error converting the ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with error_type `CONVERT_ERROR`

**Possible Reasons**:
- Failed conversion from auditable to hashable ballot
- Data transformation errors during ballot processing
- Incompatible ballot formats during conversion
- Missing required data for conversion process
- Cryptographic conversion failures

#### SERIALIZE_ERROR
**Translation**: "There was an error serializing the ballot. Please try again later or contact support for assistance."

**Triggering Functions/Queries**:
- `ActionButtons.castBallotAction()` - When ballot service throws `IBallotError` with error_type `SERIALIZE_ERROR`

**Possible Reasons**:
- Failed to serialize ballot data to JSON
- Memory allocation errors during serialization
- Data structure too complex for serialization
- Output format incompatibility
- System resource exhaustion during serialization

### 6. Authentication and Session Errors

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

## Debugging Tips

- Check browser console for detailed error messages
- Verify network connectivity and server availability
- Ensure browser supports required features (sessionStorage)
- Check if election is active and user is authorized
- Verify ballot data integrity and format
- Monitor server logs for backend errors (i.e. harvest, windmill)

## Related Files

- `packages/voting-portal/src/routes/ReviewScreen.tsx` - Main error handling logic
- `packages/voting-portal/src/translations/en.ts` - Error message translations
- `packages/voting-portal/src/queries/InsertCastVote.ts` - Cast vote mutation
- `packages/voting-portal/src/services/VotingPortalError.ts` - Error type definitions
