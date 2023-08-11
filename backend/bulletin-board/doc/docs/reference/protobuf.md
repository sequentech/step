---
title: "gRPC API"
sidebar_position: 0
---

# gRPC API
<a name="top"></a>



<a name="bulletin_board-proto"></a>

## bulletin_board.proto



<a name="bulletin_board-AddEntriesRequest"></a>

### AddEntriesRequest
AddEntries request.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_uuid | [string](#string) |  | UUID of the board in which the entry should be added. |
| entries | [NewDataEntry](#bulletin_board-NewDataEntry) | repeated | Data entries to be added. |






<a name="bulletin_board-AddEntriesResponse"></a>

### AddEntriesResponse
AddEntries response.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| entries | [BoardEntry](#bulletin_board-BoardEntry) | repeated | List of entries added, but with the entry data not included to avoid performance penalty. |
| checkpoint | [Checkpoint](#bulletin_board-Checkpoint) |  | Latest checkpoint that includes these entries. |






<a name="bulletin_board-Board"></a>

### Board
BulletinBoard message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| uuid | [string](#string) |  | Board UUID will coincide with the one requested. |
| name | [string](#string) |  | Human-readable name to assign to this board. Requirements: - it need not to be an empty string - it needs to be valid UTF-8 - it cannot contain any whitspace - it cannot contain the `+` character |
| description | [string](#string) | optional | Human-readable description of the board. Optional. |
| is_archived | [bool](#bool) |  | If the board is archived or not. |
| public_key | [string](#string) |  | Public Key of the board. |
| metadata | [Board.MetadataEntry](#bulletin_board-Board-MetadataEntry) | repeated | Metadata of the board. |
| permissions | [Permissions](#bulletin_board-Permissions) |  | Manage board permissions. |






<a name="bulletin_board-Board-MetadataEntry"></a>

### Board.MetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-BoardEntry"></a>

### BoardEntry
BoardEntry message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sequence_id | [uint64](#uint64) |  | Board entry sequence id, assigned by the service. |
| board | [Board](#bulletin_board-Board) |  |  |
| entry_data | [BoardEntryData](#bulletin_board-BoardEntryData) |  |  |
| metadata | [BoardEntry.MetadataEntry](#bulletin_board-BoardEntry-MetadataEntry) | repeated | Board entry metadata, provided by the signer. |
| signer_public_key | [string](#string) |  | Signer of the board entry. |
| signature | [string](#string) |  | Verifiable signature of the board entry by the signer. |
| timestamp | [uint64](#uint64) |  | Timestamp of when the board entry was added, provided by the service. |






<a name="bulletin_board-BoardEntry-MetadataEntry"></a>

### BoardEntry.MetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-BoardEntryData"></a>

### BoardEntryData



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data | [bytes](#bytes) | optional | Data is optional because for example in AddEntries method it doesn't make sense to reply with the whole entry again - it's inefficient |






<a name="bulletin_board-Checkpoint"></a>

### Checkpoint
Checkpoint message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| origin | [string](#string) |  | Origin is the string identifing the log which issued this checkpoint. |
| size | [uint64](#uint64) |  | Size is the number of entries in the log at this checkpoint. |
| hash | [string](#string) |  | Hash which commits to the contents of the entire log. |






<a name="bulletin_board-CreateBoardRequest"></a>

### CreateBoardRequest
CreateBoard request.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_uuid | [string](#string) |  | Board UUID will coincide with the one requested. |
| board_name | [string](#string) |  | Human-readable name to assign to this board. |
| board_description | [string](#string) | optional | Human-readable description of the board. Optional. |
| board_metadata | [CreateBoardRequest.BoardMetadataEntry](#bulletin_board-CreateBoardRequest-BoardMetadataEntry) | repeated | Metadata of the board. |
| permissions | [Permissions](#bulletin_board-Permissions) |  | Board permissions. |
| signer_public_key | [string](#string) |  | Signer of the board entry. |
| signature | [string](#string) |  | Verifiable signature of the board entry by the signer. |






<a name="bulletin_board-CreateBoardRequest-BoardMetadataEntry"></a>

### CreateBoardRequest.BoardMetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-CreateBoardResponse"></a>

### CreateBoardResponse
CreateBoard response.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| bulletin_board | [Board](#bulletin_board-Board) |  | Created Board. |
| checkpoint | [Checkpoint](#bulletin_board-Checkpoint) |  | Latest checkpoint of the board. |






<a name="bulletin_board-ListBoardItem"></a>

### ListBoardItem
Items that ties together a Board and its latest sequence id.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_last_sequence_id | [uint64](#uint64) |  | Latest sequence id of this board. |
| board | [Board](#bulletin_board-Board) |  | Latest configuration of this board. |






<a name="bulletin_board-ListBoardsRequest"></a>

### ListBoardsRequest
ListBoards request.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| is_archived | [bool](#bool) | optional | Filter by board archival state. |
| board_name | [string](#string) | optional | Listing by exact board name match. |
| board_uuid | [string](#string) | optional | Listing by exact board uuid match. |






<a name="bulletin_board-ListBoardsResponse"></a>

### ListBoardsResponse
ListBoards response.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| boards | [ListBoardItem](#bulletin_board-ListBoardItem) | repeated | Listed boards |






<a name="bulletin_board-ListEntriesRequest"></a>

### ListEntriesRequest
ListEntries request.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_uuid | [string](#string) |  | UUID of the board whose entries should be listed. |
| start_sequence_id | [uint64](#uint64) |  | Request listing entries in order, including and starting with this sequence id. |






<a name="bulletin_board-ListEntriesResponse"></a>

### ListEntriesResponse
ListEntries response.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_last_sequence_id | [uint64](#uint64) |  | Last sequence id of the requested board. |
| board_entries | [BoardEntry](#bulletin_board-BoardEntry) | repeated | List of entries included in the response. |






<a name="bulletin_board-ModifyBoardRequest"></a>

### ModifyBoardRequest
ModifyBoard request.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board_uuid | [string](#string) |  | UUID of the board to modify. |
| board_name | [string](#string) |  | Human-readable name to assign to this board. |
| board_description | [string](#string) | optional | Human-readable description of the board. Optional. |
| is_archived | [bool](#bool) |  | If the board is archived or not. |
| board_metadata | [ModifyBoardRequest.BoardMetadataEntry](#bulletin_board-ModifyBoardRequest-BoardMetadataEntry) | repeated | Metadata of the board. |
| permissions | [Permissions](#bulletin_board-Permissions) |  | Board permissions. |
| signer_public_key | [string](#string) |  | Signer of the board entry. |
| signature | [string](#string) |  | Verifiable signature of the board entry by the signer. |






<a name="bulletin_board-ModifyBoardRequest-BoardMetadataEntry"></a>

### ModifyBoardRequest.BoardMetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-ModifyBoardResponse"></a>

### ModifyBoardResponse
ModifyBoard response.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| board | [Board](#bulletin_board-Board) |  | Latest state of the Board, where it is archived. |






<a name="bulletin_board-NewDataEntry"></a>

### NewDataEntry
NewDataEntry message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data | [bytes](#bytes) |  | Entry data. Note this is the only part of the data that will be signed by the signer - not even the metadata will be signed. |
| metadata | [NewDataEntry.MetadataEntry](#bulletin_board-NewDataEntry-MetadataEntry) | repeated | Entry metadata. |
| signer_public_key | [string](#string) |  | Signer of the board entry. |
| signature | [string](#string) |  | Verifiable signature of the board entry by the signer. |






<a name="bulletin_board-NewDataEntry-MetadataEntry"></a>

### NewDataEntry.MetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-Permissions"></a>

### Permissions
Handles permissions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| users | [User](#bulletin_board-User) | repeated | List of users of the board. |
| roles | [Role](#bulletin_board-Role) | repeated | List of roles that can be assigned to users. |
| user_roles | [UserRole](#bulletin_board-UserRole) | repeated | List of assignment of roles to users. |






<a name="bulletin_board-Role"></a>

### Role
Represents a role with a set of permissions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | Name of the role. |
| permissions | [string](#string) | repeated | Permissions of the role. Available permissions: - "AddEntries" - "ModifyBoardConfig" |
| metadata | [Role.MetadataEntry](#bulletin_board-Role-MetadataEntry) | repeated | Metadata of the role. |






<a name="bulletin_board-Role-MetadataEntry"></a>

### Role.MetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-User"></a>

### User
Represents a user identity.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | Identity name of the user, unique for each board. |
| public_key | [string](#string) |  | Public key of the user. |
| metadata | [User.MetadataEntry](#bulletin_board-User-MetadataEntry) | repeated | Metadata of the user. |






<a name="bulletin_board-User-MetadataEntry"></a>

### User.MetadataEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="bulletin_board-UserRole"></a>

### UserRole
Assigns a set of roles to a user by name.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| user_name | [string](#string) |  | Name of the user. |
| role_names | [string](#string) | repeated | List of roles assigned to the user. |












<a name="bulletin_board-BulletinBoard"></a>

### BulletinBoard


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListBoards | [ListBoardsRequest](#bulletin_board-ListBoardsRequest) | [ListBoardsResponse](#bulletin_board-ListBoardsResponse) |  |
| CreateBoard | [CreateBoardRequest](#bulletin_board-CreateBoardRequest) | [CreateBoardResponse](#bulletin_board-CreateBoardResponse) |  |
| ListEntries | [ListEntriesRequest](#bulletin_board-ListEntriesRequest) | [ListEntriesResponse](#bulletin_board-ListEntriesResponse) |  |
| AddEntries | [AddEntriesRequest](#bulletin_board-AddEntriesRequest) | [AddEntriesResponse](#bulletin_board-AddEntriesResponse) |  |
| ModifyBoard | [ModifyBoardRequest](#bulletin_board-ModifyBoardRequest) | [ModifyBoardResponse](#bulletin_board-ModifyBoardResponse) |  |





## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a name="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a name="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a name="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a name="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a name="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a name="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a name="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |
