/* eslint-disable */
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
  bytea: { input: any; output: any; }
  jsonb: { input: any; output: any; }
  timestamptz: { input: any; output: any; }
  uuid: { input: any; output: any; }
};

export type Aggregate = {
  __typename?: 'Aggregate';
  count: Scalars['Int']['output'];
};

/** Boolean expression to compare columns of type "Boolean". All fields are combined with logical 'AND'. */
export type Boolean_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['Boolean']['input']>;
  _gt?: InputMaybe<Scalars['Boolean']['input']>;
  _gte?: InputMaybe<Scalars['Boolean']['input']>;
  _in?: InputMaybe<Array<Scalars['Boolean']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['Boolean']['input']>;
  _lte?: InputMaybe<Scalars['Boolean']['input']>;
  _neq?: InputMaybe<Scalars['Boolean']['input']>;
  _nin?: InputMaybe<Array<Scalars['Boolean']['input']>>;
};

export type CheckPrivateKeyInput = {
  election_event_id: Scalars['String']['input'];
  keys_ceremony_id: Scalars['String']['input'];
  private_key_base64: Scalars['String']['input'];
};

export type CheckPrivateKeyOutput = {
  __typename?: 'CheckPrivateKeyOutput';
  is_valid: Scalars['Boolean']['output'];
};

export type CreateElectionEventInput = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  audit_election_event_id?: InputMaybe<Scalars['String']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['String']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['String']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name: Scalars['String']['input'];
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id: Scalars['String']['input'];
  updated_at?: InputMaybe<Scalars['String']['input']>;
  user_boards?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

export type CreateElectionEventOutput = {
  __typename?: 'CreateElectionEventOutput';
  id: Scalars['String']['output'];
};

export type CreateKeysCeremonyInput = {
  election_event_id: Scalars['String']['input'];
  threshold: Scalars['Int']['input'];
  trustee_names?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type CreateKeysCeremonyOutput = {
  __typename?: 'CreateKeysCeremonyOutput';
  keys_ceremony_id: Scalars['String']['output'];
};

export type CreatePermissionInput = {
  permission: KeycloakPermission2;
  tenant_id: Scalars['String']['input'];
};

export type DataListPgAudit = {
  __typename?: 'DataListPgAudit';
  items: Array<Maybe<PgAuditRow>>;
  total: TotalAggregate;
};

export type DeleteUserOutput = {
  __typename?: 'DeleteUserOutput';
  id?: Maybe<Scalars['String']['output']>;
};

export type EditUsersInput = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  email?: InputMaybe<Scalars['String']['input']>;
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  first_name?: InputMaybe<Scalars['String']['input']>;
  groups?: InputMaybe<Array<Scalars['String']['input']>>;
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
  username?: InputMaybe<Scalars['String']['input']>;
};

export type FetchDocumentOutput = {
  __typename?: 'FetchDocumentOutput';
  url: Scalars['String']['output'];
};

export type GetPermissionsInput = {
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};

export type GetPermissionsOutput = {
  __typename?: 'GetPermissionsOutput';
  items: Array<KeycloakPermission>;
  total: TotalAggregate;
};

export type GetPrivateKeyInput = {
  election_event_id: Scalars['String']['input'];
};

export type GetPrivateKeyOutput = {
  __typename?: 'GetPrivateKeyOutput';
  private_key_base64: Scalars['String']['output'];
};

export type GetRolesInput = {
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};

export type GetRolesOutput = {
  __typename?: 'GetRolesOutput';
  items: Array<KeycloakRole>;
  total: TotalAggregate;
};

export type GetUploadUrlOutput = {
  __typename?: 'GetUploadUrlOutput';
  document_id: Scalars['String']['output'];
  url: Scalars['String']['output'];
};

export type GetUsersInput = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  email?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};

export type GetUsersOutput = {
  __typename?: 'GetUsersOutput';
  items: Array<KeycloakUser>;
  total: TotalAggregate;
};

export type InsertTenantOutput = {
  __typename?: 'InsertTenantOutput';
  id: Scalars['uuid']['output'];
  slug: Scalars['String']['output'];
};

/** Boolean expression to compare columns of type "Int". All fields are combined with logical 'AND'. */
export type Int_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['Int']['input']>;
  _gt?: InputMaybe<Scalars['Int']['input']>;
  _gte?: InputMaybe<Scalars['Int']['input']>;
  _in?: InputMaybe<Array<Scalars['Int']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['Int']['input']>;
  _lte?: InputMaybe<Scalars['Int']['input']>;
  _neq?: InputMaybe<Scalars['Int']['input']>;
  _nin?: InputMaybe<Array<Scalars['Int']['input']>>;
};

export type KeycloakPermission = {
  __typename?: 'KeycloakPermission';
  attributes?: Maybe<Scalars['jsonb']['output']>;
  container_id?: Maybe<Scalars['String']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
};

export type KeycloakPermission2 = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  container_id?: InputMaybe<Scalars['String']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
};

export type KeycloakRole = {
  __typename?: 'KeycloakRole';
  access?: Maybe<Scalars['jsonb']['output']>;
  attributes?: Maybe<Scalars['jsonb']['output']>;
  client_roles?: Maybe<Scalars['jsonb']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  permissions?: Maybe<Scalars['jsonb']['output']>;
};

export type KeycloakRole2 = {
  access?: InputMaybe<Scalars['jsonb']['input']>;
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  client_roles?: InputMaybe<Scalars['jsonb']['input']>;
  id?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permissions?: InputMaybe<Scalars['jsonb']['input']>;
};

export type KeycloakUser = {
  __typename?: 'KeycloakUser';
  attributes?: Maybe<Scalars['jsonb']['output']>;
  email?: Maybe<Scalars['String']['output']>;
  email_verified?: Maybe<Scalars['Boolean']['output']>;
  enabled?: Maybe<Scalars['Boolean']['output']>;
  first_name?: Maybe<Scalars['String']['output']>;
  groups?: Maybe<Array<Scalars['String']['output']>>;
  id?: Maybe<Scalars['String']['output']>;
  last_name?: Maybe<Scalars['String']['output']>;
  username?: Maybe<Scalars['String']['output']>;
};

export type KeycloakUser2 = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  email?: InputMaybe<Scalars['String']['input']>;
  email_verified?: InputMaybe<Scalars['Boolean']['input']>;
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  first_name?: InputMaybe<Scalars['String']['input']>;
  groups?: InputMaybe<Array<Scalars['String']['input']>>;
  id?: InputMaybe<Scalars['String']['input']>;
  last_name?: InputMaybe<Scalars['String']['input']>;
  username?: InputMaybe<Scalars['String']['input']>;
};

export enum OrderDirection {
  Asc = 'asc',
  Desc = 'desc'
}

export type PgAuditOrderBy = {
  audit_type?: InputMaybe<OrderDirection>;
  class?: InputMaybe<OrderDirection>;
  command?: InputMaybe<OrderDirection>;
  dbname?: InputMaybe<OrderDirection>;
  id?: InputMaybe<OrderDirection>;
  server_timestamp?: InputMaybe<OrderDirection>;
  session_id?: InputMaybe<OrderDirection>;
  statement?: InputMaybe<OrderDirection>;
  user?: InputMaybe<OrderDirection>;
};

export type PgAuditRow = {
  __typename?: 'PgAuditRow';
  audit_type: Scalars['String']['output'];
  class: Scalars['String']['output'];
  command: Scalars['String']['output'];
  dbname: Scalars['String']['output'];
  id: Scalars['Int']['output'];
  server_timestamp: Scalars['Int']['output'];
  session_id: Scalars['String']['output'];
  statement: Scalars['String']['output'];
  user: Scalars['String']['output'];
};

export type ScheduledEventOutput3 = {
  __typename?: 'ScheduledEventOutput3';
  id?: Maybe<Scalars['String']['output']>;
};

export type SetRolePermissionOutput = {
  __typename?: 'SetRolePermissionOutput';
  id?: Maybe<Scalars['String']['output']>;
};

export type SetUserRoleOutput = {
  __typename?: 'SetUserRoleOutput';
  id?: Maybe<Scalars['String']['output']>;
};

/** Boolean expression to compare columns of type "String". All fields are combined with logical 'AND'. */
export type String_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['String']['input']>;
  _gt?: InputMaybe<Scalars['String']['input']>;
  _gte?: InputMaybe<Scalars['String']['input']>;
  /** does the column match the given case-insensitive pattern */
  _ilike?: InputMaybe<Scalars['String']['input']>;
  _in?: InputMaybe<Array<Scalars['String']['input']>>;
  /** does the column match the given POSIX regular expression, case insensitive */
  _iregex?: InputMaybe<Scalars['String']['input']>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  /** does the column match the given pattern */
  _like?: InputMaybe<Scalars['String']['input']>;
  _lt?: InputMaybe<Scalars['String']['input']>;
  _lte?: InputMaybe<Scalars['String']['input']>;
  _neq?: InputMaybe<Scalars['String']['input']>;
  /** does the column NOT match the given case-insensitive pattern */
  _nilike?: InputMaybe<Scalars['String']['input']>;
  _nin?: InputMaybe<Array<Scalars['String']['input']>>;
  /** does the column NOT match the given POSIX regular expression, case insensitive */
  _niregex?: InputMaybe<Scalars['String']['input']>;
  /** does the column NOT match the given pattern */
  _nlike?: InputMaybe<Scalars['String']['input']>;
  /** does the column NOT match the given POSIX regular expression, case sensitive */
  _nregex?: InputMaybe<Scalars['String']['input']>;
  /** does the column NOT match the given SQL regular expression */
  _nsimilar?: InputMaybe<Scalars['String']['input']>;
  /** does the column match the given POSIX regular expression, case sensitive */
  _regex?: InputMaybe<Scalars['String']['input']>;
  /** does the column match the given SQL regular expression */
  _similar?: InputMaybe<Scalars['String']['input']>;
};

export type TotalAggregate = {
  __typename?: 'TotalAggregate';
  aggregate: Aggregate;
};

/** Boolean expression to compare columns of type "bytea". All fields are combined with logical 'AND'. */
export type Bytea_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['bytea']['input']>;
  _gt?: InputMaybe<Scalars['bytea']['input']>;
  _gte?: InputMaybe<Scalars['bytea']['input']>;
  _in?: InputMaybe<Array<Scalars['bytea']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['bytea']['input']>;
  _lte?: InputMaybe<Scalars['bytea']['input']>;
  _neq?: InputMaybe<Scalars['bytea']['input']>;
  _nin?: InputMaybe<Array<Scalars['bytea']['input']>>;
};

/** ordering argument of a cursor */
export enum Cursor_Ordering {
  /** ascending ordering of the cursor */
  Asc = 'ASC',
  /** descending ordering of the cursor */
  Desc = 'DESC'
}

export type Jsonb_Cast_Exp = {
  String?: InputMaybe<String_Comparison_Exp>;
};

/** Boolean expression to compare columns of type "jsonb". All fields are combined with logical 'AND'. */
export type Jsonb_Comparison_Exp = {
  _cast?: InputMaybe<Jsonb_Cast_Exp>;
  /** is the column contained in the given json value */
  _contained_in?: InputMaybe<Scalars['jsonb']['input']>;
  /** does the column contain the given json value at the top level */
  _contains?: InputMaybe<Scalars['jsonb']['input']>;
  _eq?: InputMaybe<Scalars['jsonb']['input']>;
  _gt?: InputMaybe<Scalars['jsonb']['input']>;
  _gte?: InputMaybe<Scalars['jsonb']['input']>;
  /** does the string exist as a top-level key in the column */
  _has_key?: InputMaybe<Scalars['String']['input']>;
  /** do all of these strings exist as top-level keys in the column */
  _has_keys_all?: InputMaybe<Array<Scalars['String']['input']>>;
  /** do any of these strings exist as top-level keys in the column */
  _has_keys_any?: InputMaybe<Array<Scalars['String']['input']>>;
  _in?: InputMaybe<Array<Scalars['jsonb']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['jsonb']['input']>;
  _lte?: InputMaybe<Scalars['jsonb']['input']>;
  _neq?: InputMaybe<Scalars['jsonb']['input']>;
  _nin?: InputMaybe<Array<Scalars['jsonb']['input']>>;
};

/** mutation root */
export type Mutation_Root = {
  __typename?: 'mutation_root';
  /** check private key */
  check_private_key?: Maybe<CheckPrivateKeyOutput>;
  /** create scheduled event */
  createScheduledEvent?: Maybe<ScheduledEventOutput3>;
  /** create keys ceremony */
  create_keys_ceremony?: Maybe<CreateKeysCeremonyOutput>;
  create_permission?: Maybe<KeycloakPermission>;
  create_role: KeycloakRole;
  create_user: KeycloakUser;
  delete_permission?: Maybe<SetRolePermissionOutput>;
  delete_role?: Maybe<SetUserRoleOutput>;
  delete_role_permission?: Maybe<SetRolePermissionOutput>;
  /** delete data from the table: "sequent_backend.area" */
  delete_sequent_backend_area?: Maybe<Sequent_Backend_Area_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.area" */
  delete_sequent_backend_area_by_pk?: Maybe<Sequent_Backend_Area>;
  /** delete data from the table: "sequent_backend.area_contest" */
  delete_sequent_backend_area_contest?: Maybe<Sequent_Backend_Area_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.area_contest" */
  delete_sequent_backend_area_contest_by_pk?: Maybe<Sequent_Backend_Area_Contest>;
  /** delete data from the table: "sequent_backend.ballot_style" */
  delete_sequent_backend_ballot_style?: Maybe<Sequent_Backend_Ballot_Style_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.ballot_style" */
  delete_sequent_backend_ballot_style_by_pk?: Maybe<Sequent_Backend_Ballot_Style>;
  /** delete data from the table: "sequent_backend.candidate" */
  delete_sequent_backend_candidate?: Maybe<Sequent_Backend_Candidate_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.candidate" */
  delete_sequent_backend_candidate_by_pk?: Maybe<Sequent_Backend_Candidate>;
  /** delete data from the table: "sequent_backend.cast_vote" */
  delete_sequent_backend_cast_vote?: Maybe<Sequent_Backend_Cast_Vote_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.cast_vote" */
  delete_sequent_backend_cast_vote_by_pk?: Maybe<Sequent_Backend_Cast_Vote>;
  /** delete data from the table: "sequent_backend.contest" */
  delete_sequent_backend_contest?: Maybe<Sequent_Backend_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.contest" */
  delete_sequent_backend_contest_by_pk?: Maybe<Sequent_Backend_Contest>;
  /** delete data from the table: "sequent_backend.document" */
  delete_sequent_backend_document?: Maybe<Sequent_Backend_Document_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.document" */
  delete_sequent_backend_document_by_pk?: Maybe<Sequent_Backend_Document>;
  /** delete data from the table: "sequent_backend.election" */
  delete_sequent_backend_election?: Maybe<Sequent_Backend_Election_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.election" */
  delete_sequent_backend_election_by_pk?: Maybe<Sequent_Backend_Election>;
  /** delete data from the table: "sequent_backend.election_event" */
  delete_sequent_backend_election_event?: Maybe<Sequent_Backend_Election_Event_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.election_event" */
  delete_sequent_backend_election_event_by_pk?: Maybe<Sequent_Backend_Election_Event>;
  /** delete data from the table: "sequent_backend.election_result" */
  delete_sequent_backend_election_result?: Maybe<Sequent_Backend_Election_Result_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.election_result" */
  delete_sequent_backend_election_result_by_pk?: Maybe<Sequent_Backend_Election_Result>;
  /** delete data from the table: "sequent_backend.election_type" */
  delete_sequent_backend_election_type?: Maybe<Sequent_Backend_Election_Type_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.election_type" */
  delete_sequent_backend_election_type_by_pk?: Maybe<Sequent_Backend_Election_Type>;
  /** delete data from the table: "sequent_backend.event_execution" */
  delete_sequent_backend_event_execution?: Maybe<Sequent_Backend_Event_Execution_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.event_execution" */
  delete_sequent_backend_event_execution_by_pk?: Maybe<Sequent_Backend_Event_Execution>;
  /** delete data from the table: "sequent_backend.keys_ceremony" */
  delete_sequent_backend_keys_ceremony?: Maybe<Sequent_Backend_Keys_Ceremony_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.keys_ceremony" */
  delete_sequent_backend_keys_ceremony_by_pk?: Maybe<Sequent_Backend_Keys_Ceremony>;
  /** delete data from the table: "sequent_backend.lock" */
  delete_sequent_backend_lock?: Maybe<Sequent_Backend_Lock_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.lock" */
  delete_sequent_backend_lock_by_pk?: Maybe<Sequent_Backend_Lock>;
  /** delete data from the table: "sequent_backend.scheduled_event" */
  delete_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.scheduled_event" */
  delete_sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** delete data from the table: "sequent_backend.tally_session" */
  delete_sequent_backend_tally_session?: Maybe<Sequent_Backend_Tally_Session_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tally_session" */
  delete_sequent_backend_tally_session_by_pk?: Maybe<Sequent_Backend_Tally_Session>;
  /** delete data from the table: "sequent_backend.tally_session_contest" */
  delete_sequent_backend_tally_session_contest?: Maybe<Sequent_Backend_Tally_Session_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tally_session_contest" */
  delete_sequent_backend_tally_session_contest_by_pk?: Maybe<Sequent_Backend_Tally_Session_Contest>;
  /** delete data from the table: "sequent_backend.tally_session_execution" */
  delete_sequent_backend_tally_session_execution?: Maybe<Sequent_Backend_Tally_Session_Execution_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tally_session_execution" */
  delete_sequent_backend_tally_session_execution_by_pk?: Maybe<Sequent_Backend_Tally_Session_Execution>;
  /** delete data from the table: "sequent_backend.tenant" */
  delete_sequent_backend_tenant?: Maybe<Sequent_Backend_Tenant_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tenant" */
  delete_sequent_backend_tenant_by_pk?: Maybe<Sequent_Backend_Tenant>;
  /** delete data from the table: "sequent_backend.trustee" */
  delete_sequent_backend_trustee?: Maybe<Sequent_Backend_Trustee_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.trustee" */
  delete_sequent_backend_trustee_by_pk?: Maybe<Sequent_Backend_Trustee>;
  delete_user?: Maybe<DeleteUserOutput>;
  delete_user_role?: Maybe<SetUserRoleOutput>;
  edit_user: KeycloakUser;
  /** get private key */
  get_private_key?: Maybe<GetPrivateKeyOutput>;
  get_upload_url?: Maybe<GetUploadUrlOutput>;
  insertElectionEvent?: Maybe<CreateElectionEventOutput>;
  /** insertTenant */
  insertTenant?: Maybe<InsertTenantOutput>;
  /** insert data into the table: "sequent_backend.area" */
  insert_sequent_backend_area?: Maybe<Sequent_Backend_Area_Mutation_Response>;
  /** insert data into the table: "sequent_backend.area_contest" */
  insert_sequent_backend_area_contest?: Maybe<Sequent_Backend_Area_Contest_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.area_contest" */
  insert_sequent_backend_area_contest_one?: Maybe<Sequent_Backend_Area_Contest>;
  /** insert a single row into the table: "sequent_backend.area" */
  insert_sequent_backend_area_one?: Maybe<Sequent_Backend_Area>;
  /** insert data into the table: "sequent_backend.ballot_style" */
  insert_sequent_backend_ballot_style?: Maybe<Sequent_Backend_Ballot_Style_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.ballot_style" */
  insert_sequent_backend_ballot_style_one?: Maybe<Sequent_Backend_Ballot_Style>;
  /** insert data into the table: "sequent_backend.candidate" */
  insert_sequent_backend_candidate?: Maybe<Sequent_Backend_Candidate_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.candidate" */
  insert_sequent_backend_candidate_one?: Maybe<Sequent_Backend_Candidate>;
  /** insert data into the table: "sequent_backend.cast_vote" */
  insert_sequent_backend_cast_vote?: Maybe<Sequent_Backend_Cast_Vote_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.cast_vote" */
  insert_sequent_backend_cast_vote_one?: Maybe<Sequent_Backend_Cast_Vote>;
  /** insert data into the table: "sequent_backend.contest" */
  insert_sequent_backend_contest?: Maybe<Sequent_Backend_Contest_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.contest" */
  insert_sequent_backend_contest_one?: Maybe<Sequent_Backend_Contest>;
  /** insert data into the table: "sequent_backend.document" */
  insert_sequent_backend_document?: Maybe<Sequent_Backend_Document_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.document" */
  insert_sequent_backend_document_one?: Maybe<Sequent_Backend_Document>;
  /** insert data into the table: "sequent_backend.election" */
  insert_sequent_backend_election?: Maybe<Sequent_Backend_Election_Mutation_Response>;
  /** insert data into the table: "sequent_backend.election_event" */
  insert_sequent_backend_election_event?: Maybe<Sequent_Backend_Election_Event_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.election_event" */
  insert_sequent_backend_election_event_one?: Maybe<Sequent_Backend_Election_Event>;
  /** insert a single row into the table: "sequent_backend.election" */
  insert_sequent_backend_election_one?: Maybe<Sequent_Backend_Election>;
  /** insert data into the table: "sequent_backend.election_result" */
  insert_sequent_backend_election_result?: Maybe<Sequent_Backend_Election_Result_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.election_result" */
  insert_sequent_backend_election_result_one?: Maybe<Sequent_Backend_Election_Result>;
  /** insert data into the table: "sequent_backend.election_type" */
  insert_sequent_backend_election_type?: Maybe<Sequent_Backend_Election_Type_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.election_type" */
  insert_sequent_backend_election_type_one?: Maybe<Sequent_Backend_Election_Type>;
  /** insert data into the table: "sequent_backend.event_execution" */
  insert_sequent_backend_event_execution?: Maybe<Sequent_Backend_Event_Execution_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.event_execution" */
  insert_sequent_backend_event_execution_one?: Maybe<Sequent_Backend_Event_Execution>;
  /** insert data into the table: "sequent_backend.keys_ceremony" */
  insert_sequent_backend_keys_ceremony?: Maybe<Sequent_Backend_Keys_Ceremony_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.keys_ceremony" */
  insert_sequent_backend_keys_ceremony_one?: Maybe<Sequent_Backend_Keys_Ceremony>;
  /** insert data into the table: "sequent_backend.lock" */
  insert_sequent_backend_lock?: Maybe<Sequent_Backend_Lock_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.lock" */
  insert_sequent_backend_lock_one?: Maybe<Sequent_Backend_Lock>;
  /** insert data into the table: "sequent_backend.scheduled_event" */
  insert_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.scheduled_event" */
  insert_sequent_backend_scheduled_event_one?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** insert data into the table: "sequent_backend.tally_session" */
  insert_sequent_backend_tally_session?: Maybe<Sequent_Backend_Tally_Session_Mutation_Response>;
  /** insert data into the table: "sequent_backend.tally_session_contest" */
  insert_sequent_backend_tally_session_contest?: Maybe<Sequent_Backend_Tally_Session_Contest_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tally_session_contest" */
  insert_sequent_backend_tally_session_contest_one?: Maybe<Sequent_Backend_Tally_Session_Contest>;
  /** insert data into the table: "sequent_backend.tally_session_execution" */
  insert_sequent_backend_tally_session_execution?: Maybe<Sequent_Backend_Tally_Session_Execution_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tally_session_execution" */
  insert_sequent_backend_tally_session_execution_one?: Maybe<Sequent_Backend_Tally_Session_Execution>;
  /** insert a single row into the table: "sequent_backend.tally_session" */
  insert_sequent_backend_tally_session_one?: Maybe<Sequent_Backend_Tally_Session>;
  /** insert data into the table: "sequent_backend.tenant" */
  insert_sequent_backend_tenant?: Maybe<Sequent_Backend_Tenant_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tenant" */
  insert_sequent_backend_tenant_one?: Maybe<Sequent_Backend_Tenant>;
  /** insert data into the table: "sequent_backend.trustee" */
  insert_sequent_backend_trustee?: Maybe<Sequent_Backend_Trustee_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.trustee" */
  insert_sequent_backend_trustee_one?: Maybe<Sequent_Backend_Trustee>;
  list_user_roles: Array<KeycloakRole>;
  set_role_permission?: Maybe<SetRolePermissionOutput>;
  set_user_role?: Maybe<SetUserRoleOutput>;
  /** update data of the table: "sequent_backend.area" */
  update_sequent_backend_area?: Maybe<Sequent_Backend_Area_Mutation_Response>;
  /** update single row of the table: "sequent_backend.area" */
  update_sequent_backend_area_by_pk?: Maybe<Sequent_Backend_Area>;
  /** update data of the table: "sequent_backend.area_contest" */
  update_sequent_backend_area_contest?: Maybe<Sequent_Backend_Area_Contest_Mutation_Response>;
  /** update single row of the table: "sequent_backend.area_contest" */
  update_sequent_backend_area_contest_by_pk?: Maybe<Sequent_Backend_Area_Contest>;
  /** update multiples rows of table: "sequent_backend.area_contest" */
  update_sequent_backend_area_contest_many?: Maybe<Array<Maybe<Sequent_Backend_Area_Contest_Mutation_Response>>>;
  /** update multiples rows of table: "sequent_backend.area" */
  update_sequent_backend_area_many?: Maybe<Array<Maybe<Sequent_Backend_Area_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.ballot_style" */
  update_sequent_backend_ballot_style?: Maybe<Sequent_Backend_Ballot_Style_Mutation_Response>;
  /** update single row of the table: "sequent_backend.ballot_style" */
  update_sequent_backend_ballot_style_by_pk?: Maybe<Sequent_Backend_Ballot_Style>;
  /** update multiples rows of table: "sequent_backend.ballot_style" */
  update_sequent_backend_ballot_style_many?: Maybe<Array<Maybe<Sequent_Backend_Ballot_Style_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.candidate" */
  update_sequent_backend_candidate?: Maybe<Sequent_Backend_Candidate_Mutation_Response>;
  /** update single row of the table: "sequent_backend.candidate" */
  update_sequent_backend_candidate_by_pk?: Maybe<Sequent_Backend_Candidate>;
  /** update multiples rows of table: "sequent_backend.candidate" */
  update_sequent_backend_candidate_many?: Maybe<Array<Maybe<Sequent_Backend_Candidate_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.cast_vote" */
  update_sequent_backend_cast_vote?: Maybe<Sequent_Backend_Cast_Vote_Mutation_Response>;
  /** update single row of the table: "sequent_backend.cast_vote" */
  update_sequent_backend_cast_vote_by_pk?: Maybe<Sequent_Backend_Cast_Vote>;
  /** update multiples rows of table: "sequent_backend.cast_vote" */
  update_sequent_backend_cast_vote_many?: Maybe<Array<Maybe<Sequent_Backend_Cast_Vote_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.contest" */
  update_sequent_backend_contest?: Maybe<Sequent_Backend_Contest_Mutation_Response>;
  /** update single row of the table: "sequent_backend.contest" */
  update_sequent_backend_contest_by_pk?: Maybe<Sequent_Backend_Contest>;
  /** update multiples rows of table: "sequent_backend.contest" */
  update_sequent_backend_contest_many?: Maybe<Array<Maybe<Sequent_Backend_Contest_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.document" */
  update_sequent_backend_document?: Maybe<Sequent_Backend_Document_Mutation_Response>;
  /** update single row of the table: "sequent_backend.document" */
  update_sequent_backend_document_by_pk?: Maybe<Sequent_Backend_Document>;
  /** update multiples rows of table: "sequent_backend.document" */
  update_sequent_backend_document_many?: Maybe<Array<Maybe<Sequent_Backend_Document_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.election" */
  update_sequent_backend_election?: Maybe<Sequent_Backend_Election_Mutation_Response>;
  /** update single row of the table: "sequent_backend.election" */
  update_sequent_backend_election_by_pk?: Maybe<Sequent_Backend_Election>;
  /** update data of the table: "sequent_backend.election_event" */
  update_sequent_backend_election_event?: Maybe<Sequent_Backend_Election_Event_Mutation_Response>;
  /** update single row of the table: "sequent_backend.election_event" */
  update_sequent_backend_election_event_by_pk?: Maybe<Sequent_Backend_Election_Event>;
  /** update multiples rows of table: "sequent_backend.election_event" */
  update_sequent_backend_election_event_many?: Maybe<Array<Maybe<Sequent_Backend_Election_Event_Mutation_Response>>>;
  /** update multiples rows of table: "sequent_backend.election" */
  update_sequent_backend_election_many?: Maybe<Array<Maybe<Sequent_Backend_Election_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.election_result" */
  update_sequent_backend_election_result?: Maybe<Sequent_Backend_Election_Result_Mutation_Response>;
  /** update single row of the table: "sequent_backend.election_result" */
  update_sequent_backend_election_result_by_pk?: Maybe<Sequent_Backend_Election_Result>;
  /** update multiples rows of table: "sequent_backend.election_result" */
  update_sequent_backend_election_result_many?: Maybe<Array<Maybe<Sequent_Backend_Election_Result_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.election_type" */
  update_sequent_backend_election_type?: Maybe<Sequent_Backend_Election_Type_Mutation_Response>;
  /** update single row of the table: "sequent_backend.election_type" */
  update_sequent_backend_election_type_by_pk?: Maybe<Sequent_Backend_Election_Type>;
  /** update multiples rows of table: "sequent_backend.election_type" */
  update_sequent_backend_election_type_many?: Maybe<Array<Maybe<Sequent_Backend_Election_Type_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.event_execution" */
  update_sequent_backend_event_execution?: Maybe<Sequent_Backend_Event_Execution_Mutation_Response>;
  /** update single row of the table: "sequent_backend.event_execution" */
  update_sequent_backend_event_execution_by_pk?: Maybe<Sequent_Backend_Event_Execution>;
  /** update multiples rows of table: "sequent_backend.event_execution" */
  update_sequent_backend_event_execution_many?: Maybe<Array<Maybe<Sequent_Backend_Event_Execution_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.keys_ceremony" */
  update_sequent_backend_keys_ceremony?: Maybe<Sequent_Backend_Keys_Ceremony_Mutation_Response>;
  /** update single row of the table: "sequent_backend.keys_ceremony" */
  update_sequent_backend_keys_ceremony_by_pk?: Maybe<Sequent_Backend_Keys_Ceremony>;
  /** update multiples rows of table: "sequent_backend.keys_ceremony" */
  update_sequent_backend_keys_ceremony_many?: Maybe<Array<Maybe<Sequent_Backend_Keys_Ceremony_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.lock" */
  update_sequent_backend_lock?: Maybe<Sequent_Backend_Lock_Mutation_Response>;
  /** update single row of the table: "sequent_backend.lock" */
  update_sequent_backend_lock_by_pk?: Maybe<Sequent_Backend_Lock>;
  /** update multiples rows of table: "sequent_backend.lock" */
  update_sequent_backend_lock_many?: Maybe<Array<Maybe<Sequent_Backend_Lock_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** update single row of the table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** update multiples rows of table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event_many?: Maybe<Array<Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.tally_session" */
  update_sequent_backend_tally_session?: Maybe<Sequent_Backend_Tally_Session_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tally_session" */
  update_sequent_backend_tally_session_by_pk?: Maybe<Sequent_Backend_Tally_Session>;
  /** update data of the table: "sequent_backend.tally_session_contest" */
  update_sequent_backend_tally_session_contest?: Maybe<Sequent_Backend_Tally_Session_Contest_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tally_session_contest" */
  update_sequent_backend_tally_session_contest_by_pk?: Maybe<Sequent_Backend_Tally_Session_Contest>;
  /** update multiples rows of table: "sequent_backend.tally_session_contest" */
  update_sequent_backend_tally_session_contest_many?: Maybe<Array<Maybe<Sequent_Backend_Tally_Session_Contest_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.tally_session_execution" */
  update_sequent_backend_tally_session_execution?: Maybe<Sequent_Backend_Tally_Session_Execution_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tally_session_execution" */
  update_sequent_backend_tally_session_execution_by_pk?: Maybe<Sequent_Backend_Tally_Session_Execution>;
  /** update multiples rows of table: "sequent_backend.tally_session_execution" */
  update_sequent_backend_tally_session_execution_many?: Maybe<Array<Maybe<Sequent_Backend_Tally_Session_Execution_Mutation_Response>>>;
  /** update multiples rows of table: "sequent_backend.tally_session" */
  update_sequent_backend_tally_session_many?: Maybe<Array<Maybe<Sequent_Backend_Tally_Session_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.tenant" */
  update_sequent_backend_tenant?: Maybe<Sequent_Backend_Tenant_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tenant" */
  update_sequent_backend_tenant_by_pk?: Maybe<Sequent_Backend_Tenant>;
  /** update multiples rows of table: "sequent_backend.tenant" */
  update_sequent_backend_tenant_many?: Maybe<Array<Maybe<Sequent_Backend_Tenant_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.trustee" */
  update_sequent_backend_trustee?: Maybe<Sequent_Backend_Trustee_Mutation_Response>;
  /** update single row of the table: "sequent_backend.trustee" */
  update_sequent_backend_trustee_by_pk?: Maybe<Sequent_Backend_Trustee>;
  /** update multiples rows of table: "sequent_backend.trustee" */
  update_sequent_backend_trustee_many?: Maybe<Array<Maybe<Sequent_Backend_Trustee_Mutation_Response>>>;
};


/** mutation root */
export type Mutation_RootCheck_Private_KeyArgs = {
  object: CheckPrivateKeyInput;
};


/** mutation root */
export type Mutation_RootCreateScheduledEventArgs = {
  created_by: Scalars['String']['input'];
  cron_config?: InputMaybe<Scalars['String']['input']>;
  election_event_id: Scalars['String']['input'];
  event_payload: Scalars['jsonb']['input'];
  event_processor: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootCreate_Keys_CeremonyArgs = {
  object: CreateKeysCeremonyInput;
};


/** mutation root */
export type Mutation_RootCreate_PermissionArgs = {
  body: CreatePermissionInput;
};


/** mutation root */
export type Mutation_RootCreate_RoleArgs = {
  role: KeycloakRole2;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootCreate_UserArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  user: KeycloakUser2;
};


/** mutation root */
export type Mutation_RootDelete_PermissionArgs = {
  permission_name: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootDelete_RoleArgs = {
  role_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Role_PermissionArgs = {
  permission_name: Scalars['String']['input'];
  role_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_AreaArgs = {
  where: Sequent_Backend_Area_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Area_ContestArgs = {
  where: Sequent_Backend_Area_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Area_Contest_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Ballot_StyleArgs = {
  where: Sequent_Backend_Ballot_Style_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Ballot_Style_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_CandidateArgs = {
  where: Sequent_Backend_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Cast_VoteArgs = {
  where: Sequent_Backend_Cast_Vote_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Cast_Vote_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_ContestArgs = {
  where: Sequent_Backend_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_DocumentArgs = {
  where: Sequent_Backend_Document_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Document_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_ElectionArgs = {
  where: Sequent_Backend_Election_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_EventArgs = {
  where: Sequent_Backend_Election_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_ResultArgs = {
  where: Sequent_Backend_Election_Result_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_Result_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_TypeArgs = {
  where: Sequent_Backend_Election_Type_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Election_Type_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Event_ExecutionArgs = {
  where: Sequent_Backend_Event_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Event_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Keys_CeremonyArgs = {
  where: Sequent_Backend_Keys_Ceremony_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Keys_Ceremony_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_LockArgs = {
  where: Sequent_Backend_Lock_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Lock_By_PkArgs = {
  key: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Scheduled_EventArgs = {
  where: Sequent_Backend_Scheduled_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Scheduled_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_SessionArgs = {
  where: Sequent_Backend_Tally_Session_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Session_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Session_ContestArgs = {
  where: Sequent_Backend_Tally_Session_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Session_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Session_ExecutionArgs = {
  where: Sequent_Backend_Tally_Session_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Session_Execution_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_TenantArgs = {
  where: Sequent_Backend_Tenant_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tenant_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_TrusteeArgs = {
  where: Sequent_Backend_Trustee_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Trustee_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_UserArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootDelete_User_RoleArgs = {
  role_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootEdit_UserArgs = {
  body: EditUsersInput;
};


/** mutation root */
export type Mutation_RootGet_Private_KeyArgs = {
  object: GetPrivateKeyInput;
};


/** mutation root */
export type Mutation_RootGet_Upload_UrlArgs = {
  media_type: Scalars['String']['input'];
  name: Scalars['String']['input'];
  size: Scalars['Int']['input'];
};


/** mutation root */
export type Mutation_RootInsertElectionEventArgs = {
  object: CreateElectionEventInput;
};


/** mutation root */
export type Mutation_RootInsertTenantArgs = {
  slug: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_AreaArgs = {
  objects: Array<Sequent_Backend_Area_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Area_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Area_ContestArgs = {
  objects: Array<Sequent_Backend_Area_Contest_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Area_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Area_Contest_OneArgs = {
  object: Sequent_Backend_Area_Contest_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Area_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Area_OneArgs = {
  object: Sequent_Backend_Area_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Area_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Ballot_StyleArgs = {
  objects: Array<Sequent_Backend_Ballot_Style_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Ballot_Style_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Ballot_Style_OneArgs = {
  object: Sequent_Backend_Ballot_Style_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Ballot_Style_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_CandidateArgs = {
  objects: Array<Sequent_Backend_Candidate_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Candidate_OneArgs = {
  object: Sequent_Backend_Candidate_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Cast_VoteArgs = {
  objects: Array<Sequent_Backend_Cast_Vote_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Cast_Vote_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Cast_Vote_OneArgs = {
  object: Sequent_Backend_Cast_Vote_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Cast_Vote_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_ContestArgs = {
  objects: Array<Sequent_Backend_Contest_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Contest_OneArgs = {
  object: Sequent_Backend_Contest_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_DocumentArgs = {
  objects: Array<Sequent_Backend_Document_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Document_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Document_OneArgs = {
  object: Sequent_Backend_Document_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Document_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_ElectionArgs = {
  objects: Array<Sequent_Backend_Election_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Election_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_EventArgs = {
  objects: Array<Sequent_Backend_Election_Event_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Event_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_Event_OneArgs = {
  object: Sequent_Backend_Election_Event_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Event_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_OneArgs = {
  object: Sequent_Backend_Election_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Election_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_ResultArgs = {
  objects: Array<Sequent_Backend_Election_Result_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Result_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_Result_OneArgs = {
  object: Sequent_Backend_Election_Result_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Result_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_TypeArgs = {
  objects: Array<Sequent_Backend_Election_Type_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Type_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Election_Type_OneArgs = {
  object: Sequent_Backend_Election_Type_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Election_Type_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Event_ExecutionArgs = {
  objects: Array<Sequent_Backend_Event_Execution_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Event_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Event_Execution_OneArgs = {
  object: Sequent_Backend_Event_Execution_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Event_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Keys_CeremonyArgs = {
  objects: Array<Sequent_Backend_Keys_Ceremony_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Keys_Ceremony_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Keys_Ceremony_OneArgs = {
  object: Sequent_Backend_Keys_Ceremony_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Keys_Ceremony_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_LockArgs = {
  objects: Array<Sequent_Backend_Lock_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Lock_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Lock_OneArgs = {
  object: Sequent_Backend_Lock_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Lock_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Scheduled_EventArgs = {
  objects: Array<Sequent_Backend_Scheduled_Event_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Scheduled_Event_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Scheduled_Event_OneArgs = {
  object: Sequent_Backend_Scheduled_Event_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Scheduled_Event_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_SessionArgs = {
  objects: Array<Sequent_Backend_Tally_Session_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Session_ContestArgs = {
  objects: Array<Sequent_Backend_Tally_Session_Contest_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Session_Contest_OneArgs = {
  object: Sequent_Backend_Tally_Session_Contest_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Session_ExecutionArgs = {
  objects: Array<Sequent_Backend_Tally_Session_Execution_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Session_Execution_OneArgs = {
  object: Sequent_Backend_Tally_Session_Execution_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Session_OneArgs = {
  object: Sequent_Backend_Tally_Session_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Session_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_TenantArgs = {
  objects: Array<Sequent_Backend_Tenant_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tenant_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tenant_OneArgs = {
  object: Sequent_Backend_Tenant_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tenant_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_TrusteeArgs = {
  objects: Array<Sequent_Backend_Trustee_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Trustee_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Trustee_OneArgs = {
  object: Sequent_Backend_Trustee_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Trustee_On_Conflict>;
};


/** mutation root */
export type Mutation_RootList_User_RolesArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootSet_Role_PermissionArgs = {
  permission_name: Scalars['String']['input'];
  role_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootSet_User_RoleArgs = {
  role_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_AreaArgs = {
  _append?: InputMaybe<Sequent_Backend_Area_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Area_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Area_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Area_Set_Input>;
  where: Sequent_Backend_Area_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Area_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Area_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Area_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Area_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Area_Set_Input>;
  pk_columns: Sequent_Backend_Area_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Area_ContestArgs = {
  _append?: InputMaybe<Sequent_Backend_Area_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Area_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Area_Contest_Set_Input>;
  where: Sequent_Backend_Area_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Area_Contest_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Area_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Area_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Area_Contest_Set_Input>;
  pk_columns: Sequent_Backend_Area_Contest_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Area_Contest_ManyArgs = {
  updates: Array<Sequent_Backend_Area_Contest_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Area_ManyArgs = {
  updates: Array<Sequent_Backend_Area_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Ballot_StyleArgs = {
  _append?: InputMaybe<Sequent_Backend_Ballot_Style_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Style_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Ballot_Style_Set_Input>;
  where: Sequent_Backend_Ballot_Style_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Ballot_Style_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Ballot_Style_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Style_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Ballot_Style_Set_Input>;
  pk_columns: Sequent_Backend_Ballot_Style_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Ballot_Style_ManyArgs = {
  updates: Array<Sequent_Backend_Ballot_Style_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_CandidateArgs = {
  _append?: InputMaybe<Sequent_Backend_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Candidate_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Candidate_Set_Input>;
  where: Sequent_Backend_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Candidate_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Candidate_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Candidate_Set_Input>;
  pk_columns: Sequent_Backend_Candidate_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Candidate_ManyArgs = {
  updates: Array<Sequent_Backend_Candidate_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Cast_VoteArgs = {
  _append?: InputMaybe<Sequent_Backend_Cast_Vote_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Cast_Vote_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Cast_Vote_Set_Input>;
  where: Sequent_Backend_Cast_Vote_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Cast_Vote_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Cast_Vote_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Cast_Vote_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Cast_Vote_Set_Input>;
  pk_columns: Sequent_Backend_Cast_Vote_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Cast_Vote_ManyArgs = {
  updates: Array<Sequent_Backend_Cast_Vote_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_ContestArgs = {
  _append?: InputMaybe<Sequent_Backend_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Contest_Set_Input>;
  where: Sequent_Backend_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Contest_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Contest_Set_Input>;
  pk_columns: Sequent_Backend_Contest_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Contest_ManyArgs = {
  updates: Array<Sequent_Backend_Contest_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_DocumentArgs = {
  _append?: InputMaybe<Sequent_Backend_Document_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Document_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Document_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Document_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Document_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Document_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Document_Set_Input>;
  where: Sequent_Backend_Document_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Document_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Document_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Document_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Document_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Document_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Document_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Document_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Document_Set_Input>;
  pk_columns: Sequent_Backend_Document_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Document_ManyArgs = {
  updates: Array<Sequent_Backend_Document_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_ElectionArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Election_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Set_Input>;
  where: Sequent_Backend_Election_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Election_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Set_Input>;
  pk_columns: Sequent_Backend_Election_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_EventArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Event_Set_Input>;
  where: Sequent_Backend_Election_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Event_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Event_Set_Input>;
  pk_columns: Sequent_Backend_Election_Event_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Event_ManyArgs = {
  updates: Array<Sequent_Backend_Election_Event_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_ManyArgs = {
  updates: Array<Sequent_Backend_Election_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_ResultArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Result_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Result_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Result_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Result_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Result_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Result_Set_Input>;
  where: Sequent_Backend_Election_Result_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Result_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Result_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Result_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Result_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Result_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Result_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Result_Set_Input>;
  pk_columns: Sequent_Backend_Election_Result_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Result_ManyArgs = {
  updates: Array<Sequent_Backend_Election_Result_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_TypeArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Type_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Type_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Type_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Type_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Type_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Type_Set_Input>;
  where: Sequent_Backend_Election_Type_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Type_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Election_Type_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Type_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Type_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Election_Type_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Election_Type_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Election_Type_Set_Input>;
  pk_columns: Sequent_Backend_Election_Type_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Election_Type_ManyArgs = {
  updates: Array<Sequent_Backend_Election_Type_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Event_ExecutionArgs = {
  _append?: InputMaybe<Sequent_Backend_Event_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Event_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Event_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Event_Execution_Set_Input>;
  where: Sequent_Backend_Event_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Event_Execution_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Event_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Event_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Event_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Event_Execution_Set_Input>;
  pk_columns: Sequent_Backend_Event_Execution_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Event_Execution_ManyArgs = {
  updates: Array<Sequent_Backend_Event_Execution_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Keys_CeremonyArgs = {
  _append?: InputMaybe<Sequent_Backend_Keys_Ceremony_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Keys_Ceremony_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Keys_Ceremony_Set_Input>;
  where: Sequent_Backend_Keys_Ceremony_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Keys_Ceremony_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Keys_Ceremony_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Keys_Ceremony_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Keys_Ceremony_Set_Input>;
  pk_columns: Sequent_Backend_Keys_Ceremony_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Keys_Ceremony_ManyArgs = {
  updates: Array<Sequent_Backend_Keys_Ceremony_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_LockArgs = {
  _set?: InputMaybe<Sequent_Backend_Lock_Set_Input>;
  where: Sequent_Backend_Lock_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Lock_By_PkArgs = {
  _set?: InputMaybe<Sequent_Backend_Lock_Set_Input>;
  pk_columns: Sequent_Backend_Lock_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Lock_ManyArgs = {
  updates: Array<Sequent_Backend_Lock_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Scheduled_EventArgs = {
  _append?: InputMaybe<Sequent_Backend_Scheduled_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Scheduled_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Scheduled_Event_Set_Input>;
  where: Sequent_Backend_Scheduled_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Scheduled_Event_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Scheduled_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Scheduled_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Scheduled_Event_Set_Input>;
  pk_columns: Sequent_Backend_Scheduled_Event_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Scheduled_Event_ManyArgs = {
  updates: Array<Sequent_Backend_Scheduled_Event_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_SessionArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Set_Input>;
  where: Sequent_Backend_Tally_Session_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Set_Input>;
  pk_columns: Sequent_Backend_Tally_Session_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_ContestArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Set_Input>;
  where: Sequent_Backend_Tally_Session_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_Contest_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Set_Input>;
  pk_columns: Sequent_Backend_Tally_Session_Contest_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_Contest_ManyArgs = {
  updates: Array<Sequent_Backend_Tally_Session_Contest_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_ExecutionArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Set_Input>;
  where: Sequent_Backend_Tally_Session_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_Execution_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Set_Input>;
  pk_columns: Sequent_Backend_Tally_Session_Execution_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_Execution_ManyArgs = {
  updates: Array<Sequent_Backend_Tally_Session_Execution_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Session_ManyArgs = {
  updates: Array<Sequent_Backend_Tally_Session_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_TenantArgs = {
  _append?: InputMaybe<Sequent_Backend_Tenant_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tenant_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tenant_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tenant_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tenant_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tenant_Set_Input>;
  where: Sequent_Backend_Tenant_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tenant_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tenant_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tenant_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tenant_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tenant_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tenant_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tenant_Set_Input>;
  pk_columns: Sequent_Backend_Tenant_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tenant_ManyArgs = {
  updates: Array<Sequent_Backend_Tenant_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_TrusteeArgs = {
  _append?: InputMaybe<Sequent_Backend_Trustee_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Trustee_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Trustee_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Trustee_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Trustee_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Trustee_Set_Input>;
  where: Sequent_Backend_Trustee_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Trustee_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Trustee_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Trustee_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Trustee_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Trustee_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Trustee_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Trustee_Set_Input>;
  pk_columns: Sequent_Backend_Trustee_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Trustee_ManyArgs = {
  updates: Array<Sequent_Backend_Trustee_Updates>;
};

/** column ordering options */
export enum Order_By {
  /** in ascending order, nulls last */
  Asc = 'asc',
  /** in ascending order, nulls first */
  AscNullsFirst = 'asc_nulls_first',
  /** in ascending order, nulls last */
  AscNullsLast = 'asc_nulls_last',
  /** in descending order, nulls first */
  Desc = 'desc',
  /** in descending order, nulls first */
  DescNullsFirst = 'desc_nulls_first',
  /** in descending order, nulls last */
  DescNullsLast = 'desc_nulls_last'
}

export type Query_Root = {
  __typename?: 'query_root';
  /** fetch document */
  fetchDocument?: Maybe<FetchDocumentOutput>;
  /** list permissions */
  get_permissions: GetPermissionsOutput;
  get_roles: GetRolesOutput;
  get_users: GetUsersOutput;
  /** List PostgreSQL audit logs */
  listPgaudit?: Maybe<DataListPgAudit>;
  /** fetch data from the table: "sequent_backend.area" */
  sequent_backend_area: Array<Sequent_Backend_Area>;
  /** fetch aggregated fields from the table: "sequent_backend.area" */
  sequent_backend_area_aggregate: Sequent_Backend_Area_Aggregate;
  /** fetch data from the table: "sequent_backend.area" using primary key columns */
  sequent_backend_area_by_pk?: Maybe<Sequent_Backend_Area>;
  /** fetch data from the table: "sequent_backend.area_contest" */
  sequent_backend_area_contest: Array<Sequent_Backend_Area_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.area_contest" */
  sequent_backend_area_contest_aggregate: Sequent_Backend_Area_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.area_contest" using primary key columns */
  sequent_backend_area_contest_by_pk?: Maybe<Sequent_Backend_Area_Contest>;
  /** fetch data from the table: "sequent_backend.ballot_style" */
  sequent_backend_ballot_style: Array<Sequent_Backend_Ballot_Style>;
  /** fetch aggregated fields from the table: "sequent_backend.ballot_style" */
  sequent_backend_ballot_style_aggregate: Sequent_Backend_Ballot_Style_Aggregate;
  /** fetch data from the table: "sequent_backend.ballot_style" using primary key columns */
  sequent_backend_ballot_style_by_pk?: Maybe<Sequent_Backend_Ballot_Style>;
  /** fetch data from the table: "sequent_backend.candidate" */
  sequent_backend_candidate: Array<Sequent_Backend_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.candidate" */
  sequent_backend_candidate_aggregate: Sequent_Backend_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.candidate" using primary key columns */
  sequent_backend_candidate_by_pk?: Maybe<Sequent_Backend_Candidate>;
  /** fetch data from the table: "sequent_backend.cast_vote" */
  sequent_backend_cast_vote: Array<Sequent_Backend_Cast_Vote>;
  /** fetch aggregated fields from the table: "sequent_backend.cast_vote" */
  sequent_backend_cast_vote_aggregate: Sequent_Backend_Cast_Vote_Aggregate;
  /** fetch data from the table: "sequent_backend.cast_vote" using primary key columns */
  sequent_backend_cast_vote_by_pk?: Maybe<Sequent_Backend_Cast_Vote>;
  /** fetch data from the table: "sequent_backend.contest" */
  sequent_backend_contest: Array<Sequent_Backend_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.contest" */
  sequent_backend_contest_aggregate: Sequent_Backend_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.contest" using primary key columns */
  sequent_backend_contest_by_pk?: Maybe<Sequent_Backend_Contest>;
  /** fetch data from the table: "sequent_backend.document" */
  sequent_backend_document: Array<Sequent_Backend_Document>;
  /** fetch aggregated fields from the table: "sequent_backend.document" */
  sequent_backend_document_aggregate: Sequent_Backend_Document_Aggregate;
  /** fetch data from the table: "sequent_backend.document" using primary key columns */
  sequent_backend_document_by_pk?: Maybe<Sequent_Backend_Document>;
  /** fetch data from the table: "sequent_backend.election" */
  sequent_backend_election: Array<Sequent_Backend_Election>;
  /** fetch aggregated fields from the table: "sequent_backend.election" */
  sequent_backend_election_aggregate: Sequent_Backend_Election_Aggregate;
  /** fetch data from the table: "sequent_backend.election" using primary key columns */
  sequent_backend_election_by_pk?: Maybe<Sequent_Backend_Election>;
  /** fetch data from the table: "sequent_backend.election_event" */
  sequent_backend_election_event: Array<Sequent_Backend_Election_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.election_event" */
  sequent_backend_election_event_aggregate: Sequent_Backend_Election_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.election_event" using primary key columns */
  sequent_backend_election_event_by_pk?: Maybe<Sequent_Backend_Election_Event>;
  /** fetch data from the table: "sequent_backend.election_result" */
  sequent_backend_election_result: Array<Sequent_Backend_Election_Result>;
  /** fetch aggregated fields from the table: "sequent_backend.election_result" */
  sequent_backend_election_result_aggregate: Sequent_Backend_Election_Result_Aggregate;
  /** fetch data from the table: "sequent_backend.election_result" using primary key columns */
  sequent_backend_election_result_by_pk?: Maybe<Sequent_Backend_Election_Result>;
  /** fetch data from the table: "sequent_backend.election_type" */
  sequent_backend_election_type: Array<Sequent_Backend_Election_Type>;
  /** fetch aggregated fields from the table: "sequent_backend.election_type" */
  sequent_backend_election_type_aggregate: Sequent_Backend_Election_Type_Aggregate;
  /** fetch data from the table: "sequent_backend.election_type" using primary key columns */
  sequent_backend_election_type_by_pk?: Maybe<Sequent_Backend_Election_Type>;
  /** fetch data from the table: "sequent_backend.event_execution" */
  sequent_backend_event_execution: Array<Sequent_Backend_Event_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.event_execution" */
  sequent_backend_event_execution_aggregate: Sequent_Backend_Event_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.event_execution" using primary key columns */
  sequent_backend_event_execution_by_pk?: Maybe<Sequent_Backend_Event_Execution>;
  /** fetch data from the table: "sequent_backend.keys_ceremony" */
  sequent_backend_keys_ceremony: Array<Sequent_Backend_Keys_Ceremony>;
  /** fetch aggregated fields from the table: "sequent_backend.keys_ceremony" */
  sequent_backend_keys_ceremony_aggregate: Sequent_Backend_Keys_Ceremony_Aggregate;
  /** fetch data from the table: "sequent_backend.keys_ceremony" using primary key columns */
  sequent_backend_keys_ceremony_by_pk?: Maybe<Sequent_Backend_Keys_Ceremony>;
  /** fetch data from the table: "sequent_backend.lock" */
  sequent_backend_lock: Array<Sequent_Backend_Lock>;
  /** fetch aggregated fields from the table: "sequent_backend.lock" */
  sequent_backend_lock_aggregate: Sequent_Backend_Lock_Aggregate;
  /** fetch data from the table: "sequent_backend.lock" using primary key columns */
  sequent_backend_lock_by_pk?: Maybe<Sequent_Backend_Lock>;
  /** fetch data from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_aggregate: Sequent_Backend_Scheduled_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.scheduled_event" using primary key columns */
  sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table: "sequent_backend.tally_session" */
  sequent_backend_tally_session: Array<Sequent_Backend_Tally_Session>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session" */
  sequent_backend_tally_session_aggregate: Sequent_Backend_Tally_Session_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session" using primary key columns */
  sequent_backend_tally_session_by_pk?: Maybe<Sequent_Backend_Tally_Session>;
  /** fetch data from the table: "sequent_backend.tally_session_contest" */
  sequent_backend_tally_session_contest: Array<Sequent_Backend_Tally_Session_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session_contest" */
  sequent_backend_tally_session_contest_aggregate: Sequent_Backend_Tally_Session_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session_contest" using primary key columns */
  sequent_backend_tally_session_contest_by_pk?: Maybe<Sequent_Backend_Tally_Session_Contest>;
  /** fetch data from the table: "sequent_backend.tally_session_execution" */
  sequent_backend_tally_session_execution: Array<Sequent_Backend_Tally_Session_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session_execution" */
  sequent_backend_tally_session_execution_aggregate: Sequent_Backend_Tally_Session_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session_execution" using primary key columns */
  sequent_backend_tally_session_execution_by_pk?: Maybe<Sequent_Backend_Tally_Session_Execution>;
  /** fetch data from the table: "sequent_backend.tenant" */
  sequent_backend_tenant: Array<Sequent_Backend_Tenant>;
  /** fetch aggregated fields from the table: "sequent_backend.tenant" */
  sequent_backend_tenant_aggregate: Sequent_Backend_Tenant_Aggregate;
  /** fetch data from the table: "sequent_backend.tenant" using primary key columns */
  sequent_backend_tenant_by_pk?: Maybe<Sequent_Backend_Tenant>;
  /** fetch data from the table: "sequent_backend.trustee" */
  sequent_backend_trustee: Array<Sequent_Backend_Trustee>;
  /** fetch aggregated fields from the table: "sequent_backend.trustee" */
  sequent_backend_trustee_aggregate: Sequent_Backend_Trustee_Aggregate;
  /** fetch data from the table: "sequent_backend.trustee" using primary key columns */
  sequent_backend_trustee_by_pk?: Maybe<Sequent_Backend_Trustee>;
};


export type Query_RootFetchDocumentArgs = {
  document_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


export type Query_RootGet_PermissionsArgs = {
  body: GetPermissionsInput;
};


export type Query_RootGet_RolesArgs = {
  body: GetRolesInput;
};


export type Query_RootGet_UsersArgs = {
  body: GetUsersInput;
};


export type Query_RootListPgauditArgs = {
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<PgAuditOrderBy>;
};


export type Query_RootSequent_Backend_AreaArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};


export type Query_RootSequent_Backend_Area_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};


export type Query_RootSequent_Backend_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Area_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Area_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Area_Contest_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Ballot_StyleArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};


export type Query_RootSequent_Backend_Ballot_Style_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};


export type Query_RootSequent_Backend_Ballot_Style_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Cast_VoteArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};


export type Query_RootSequent_Backend_Cast_Vote_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};


export type Query_RootSequent_Backend_Cast_Vote_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_DocumentArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Document_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Document_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};


export type Query_RootSequent_Backend_Document_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Document_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Document_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};


export type Query_RootSequent_Backend_Document_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_ElectionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Election_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Election_ResultArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Result_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Result_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Result_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Result_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Result_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Result_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Election_TypeArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Type_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Type_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Type_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Type_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Type_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};


export type Query_RootSequent_Backend_Election_Type_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Event_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Event_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Event_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Event_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Event_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Event_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Event_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Keys_CeremonyArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};


export type Query_RootSequent_Backend_Keys_Ceremony_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};


export type Query_RootSequent_Backend_Keys_Ceremony_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_LockArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Lock_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Lock_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};


export type Query_RootSequent_Backend_Lock_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Lock_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Lock_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};


export type Query_RootSequent_Backend_Lock_By_PkArgs = {
  key: Scalars['String']['input'];
};


export type Query_RootSequent_Backend_Scheduled_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Scheduled_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Scheduled_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Tally_SessionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Tally_Session_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Tally_Session_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Session_Execution_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_TenantArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tenant_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tenant_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tenant_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tenant_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tenant_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tenant_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_TrusteeArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


export type Query_RootSequent_Backend_Trustee_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


export type Query_RootSequent_Backend_Trustee_By_PkArgs = {
  id: Scalars['uuid']['input'];
};

/** columns and relationships of "sequent_backend.area" */
export type Sequent_Backend_Area = {
  __typename?: 'sequent_backend_area';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
  type?: Maybe<Scalars['String']['output']>;
};


/** columns and relationships of "sequent_backend.area" */
export type Sequent_Backend_AreaAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.area" */
export type Sequent_Backend_AreaLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.area" */
export type Sequent_Backend_Area_Aggregate = {
  __typename?: 'sequent_backend_area_aggregate';
  aggregate?: Maybe<Sequent_Backend_Area_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Area>;
};

/** aggregate fields of "sequent_backend.area" */
export type Sequent_Backend_Area_Aggregate_Fields = {
  __typename?: 'sequent_backend_area_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Area_Max_Fields>;
  min?: Maybe<Sequent_Backend_Area_Min_Fields>;
};


/** aggregate fields of "sequent_backend.area" */
export type Sequent_Backend_Area_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Area_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Area_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.area". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Area_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Area_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Area_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.area" */
export enum Sequent_Backend_Area_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  AreaPkey = 'area_pkey'
}

/** columns and relationships of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest = {
  __typename?: 'sequent_backend_area_contest';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  /** An object relationship */
  area?: Maybe<Sequent_Backend_Area>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  /** An object relationship */
  contest?: Maybe<Sequent_Backend_Contest>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_ContestAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_ContestLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Aggregate = {
  __typename?: 'sequent_backend_area_contest_aggregate';
  aggregate?: Maybe<Sequent_Backend_Area_Contest_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Area_Contest>;
};

/** aggregate fields of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Aggregate_Fields = {
  __typename?: 'sequent_backend_area_contest_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Area_Contest_Max_Fields>;
  min?: Maybe<Sequent_Backend_Area_Contest_Min_Fields>;
};


/** aggregate fields of "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Area_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Area_Contest_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.area_contest". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Area_Contest_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Area_Contest_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Area_Contest_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  contest?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.area_contest" */
export enum Sequent_Backend_Area_Contest_Constraint {
  /** unique or primary key constraint on columns "id" */
  AreaContextPkey = 'area_context_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Area_Contest_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Area_Contest_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Area_Contest_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area?: InputMaybe<Sequent_Backend_Area_Obj_Rel_Insert_Input>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest?: InputMaybe<Sequent_Backend_Contest_Obj_Rel_Insert_Input>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Area_Contest_Max_Fields = {
  __typename?: 'sequent_backend_area_contest_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Area_Contest_Min_Fields = {
  __typename?: 'sequent_backend_area_contest_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Mutation_Response = {
  __typename?: 'sequent_backend_area_contest_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Area_Contest>;
};

/** on_conflict condition type for table "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_On_Conflict = {
  constraint: Sequent_Backend_Area_Contest_Constraint;
  update_columns?: Array<Sequent_Backend_Area_Contest_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.area_contest". */
export type Sequent_Backend_Area_Contest_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area?: InputMaybe<Sequent_Backend_Area_Order_By>;
  area_id?: InputMaybe<Order_By>;
  contest?: InputMaybe<Sequent_Backend_Contest_Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.area_contest */
export type Sequent_Backend_Area_Contest_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Area_Contest_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.area_contest" */
export enum Sequent_Backend_Area_Contest_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.area_contest" */
export type Sequent_Backend_Area_Contest_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_area_contest" */
export type Sequent_Backend_Area_Contest_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Area_Contest_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Area_Contest_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.area_contest" */
export enum Sequent_Backend_Area_Contest_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Area_Contest_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Area_Contest_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Contest_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Area_Contest_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Area_Contest_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Area_Contest_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Area_Contest_Bool_Exp;
};

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Area_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Area_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Area_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.area" */
export type Sequent_Backend_Area_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Area_Max_Fields = {
  __typename?: 'sequent_backend_area_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Area_Min_Fields = {
  __typename?: 'sequent_backend_area_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.area" */
export type Sequent_Backend_Area_Mutation_Response = {
  __typename?: 'sequent_backend_area_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Area>;
};

/** input type for inserting object relation for remote table "sequent_backend.area" */
export type Sequent_Backend_Area_Obj_Rel_Insert_Input = {
  data: Sequent_Backend_Area_Insert_Input;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Area_On_Conflict>;
};

/** on_conflict condition type for table "sequent_backend.area" */
export type Sequent_Backend_Area_On_Conflict = {
  constraint: Sequent_Backend_Area_Constraint;
  update_columns?: Array<Sequent_Backend_Area_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.area". */
export type Sequent_Backend_Area_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.area */
export type Sequent_Backend_Area_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Area_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.area" */
export enum Sequent_Backend_Area_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

/** input type for updating data in table "sequent_backend.area" */
export type Sequent_Backend_Area_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_area" */
export type Sequent_Backend_Area_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Area_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Area_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.area" */
export enum Sequent_Backend_Area_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

export type Sequent_Backend_Area_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Area_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Area_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Area_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Area_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Area_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Area_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Area_Bool_Exp;
};

/** columns and relationships of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style = {
  __typename?: 'sequent_backend_ballot_style';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  ballot_eml?: Maybe<Scalars['String']['output']>;
  ballot_signature?: Maybe<Scalars['bytea']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  status?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_StyleAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_StyleLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Aggregate = {
  __typename?: 'sequent_backend_ballot_style_aggregate';
  aggregate?: Maybe<Sequent_Backend_Ballot_Style_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Ballot_Style>;
};

/** aggregate fields of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Aggregate_Fields = {
  __typename?: 'sequent_backend_ballot_style_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Ballot_Style_Max_Fields>;
  min?: Maybe<Sequent_Backend_Ballot_Style_Min_Fields>;
};


/** aggregate fields of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Ballot_Style_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.ballot_style". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Ballot_Style_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  ballot_eml?: InputMaybe<String_Comparison_Exp>;
  ballot_signature?: InputMaybe<Bytea_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  deleted_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  status?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.ballot_style" */
export enum Sequent_Backend_Ballot_Style_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  BallotStylePkey = 'ballot_style_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Ballot_Style_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Ballot_Style_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Ballot_Style_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  ballot_eml?: InputMaybe<Scalars['String']['input']>;
  ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Ballot_Style_Max_Fields = {
  __typename?: 'sequent_backend_ballot_style_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  ballot_eml?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  status?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Ballot_Style_Min_Fields = {
  __typename?: 'sequent_backend_ballot_style_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  ballot_eml?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  status?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Mutation_Response = {
  __typename?: 'sequent_backend_ballot_style_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Ballot_Style>;
};

/** on_conflict condition type for table "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_On_Conflict = {
  constraint: Sequent_Backend_Ballot_Style_Constraint;
  update_columns?: Array<Sequent_Backend_Ballot_Style_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.ballot_style". */
export type Sequent_Backend_Ballot_Style_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  ballot_eml?: InputMaybe<Order_By>;
  ballot_signature?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  deleted_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.ballot_style */
export type Sequent_Backend_Ballot_Style_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Ballot_Style_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.ballot_style" */
export enum Sequent_Backend_Ballot_Style_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  BallotEml = 'ballot_eml',
  /** column name */
  BallotSignature = 'ballot_signature',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  DeletedAt = 'deleted_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  ballot_eml?: InputMaybe<Scalars['String']['input']>;
  ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_ballot_style" */
export type Sequent_Backend_Ballot_Style_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Ballot_Style_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Ballot_Style_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  ballot_eml?: InputMaybe<Scalars['String']['input']>;
  ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.ballot_style" */
export enum Sequent_Backend_Ballot_Style_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  BallotEml = 'ballot_eml',
  /** column name */
  BallotSignature = 'ballot_signature',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  DeletedAt = 'deleted_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Ballot_Style_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Ballot_Style_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Style_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Style_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Ballot_Style_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Ballot_Style_Bool_Exp;
};

/** columns and relationships of "sequent_backend.candidate" */
export type Sequent_Backend_Candidate = {
  __typename?: 'sequent_backend_candidate';
  alias?: Maybe<Scalars['String']['output']>;
  annotations?: Maybe<Scalars['jsonb']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  image_document_id?: Maybe<Scalars['String']['output']>;
  is_public?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  presentation?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  type?: Maybe<Scalars['String']['output']>;
};


/** columns and relationships of "sequent_backend.candidate" */
export type Sequent_Backend_CandidateAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.candidate" */
export type Sequent_Backend_CandidateLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.candidate" */
export type Sequent_Backend_CandidatePresentationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Aggregate = {
  __typename?: 'sequent_backend_candidate_aggregate';
  aggregate?: Maybe<Sequent_Backend_Candidate_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Candidate>;
};

export type Sequent_Backend_Candidate_Aggregate_Bool_Exp = {
  bool_and?: InputMaybe<Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_And>;
  bool_or?: InputMaybe<Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_Or>;
  count?: InputMaybe<Sequent_Backend_Candidate_Aggregate_Bool_Exp_Count>;
};

export type Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_And = {
  arguments: Sequent_Backend_Candidate_Select_Column_Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_And_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_Or = {
  arguments: Sequent_Backend_Candidate_Select_Column_Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Candidate_Aggregate_Bool_Exp_Count = {
  arguments?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
  predicate: Int_Comparison_Exp;
};

/** aggregate fields of "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Aggregate_Fields = {
  __typename?: 'sequent_backend_candidate_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Candidate_Max_Fields>;
  min?: Maybe<Sequent_Backend_Candidate_Min_Fields>;
};


/** aggregate fields of "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** order by aggregate values of table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Aggregate_Order_By = {
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Sequent_Backend_Candidate_Max_Order_By>;
  min?: InputMaybe<Sequent_Backend_Candidate_Min_Order_By>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Candidate_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
};

/** input type for inserting array relation for remote table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Arr_Rel_Insert_Input = {
  data: Array<Sequent_Backend_Candidate_Insert_Input>;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Candidate_On_Conflict>;
};

/** Boolean expression to filter rows from the table "sequent_backend.candidate". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Candidate_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Candidate_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Candidate_Bool_Exp>>;
  alias?: InputMaybe<String_Comparison_Exp>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  image_document_id?: InputMaybe<String_Comparison_Exp>;
  is_public?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.candidate" */
export enum Sequent_Backend_Candidate_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  CandidatePkey = 'candidate_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Candidate_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Candidate_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Candidate_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Insert_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Candidate_Max_Fields = {
  __typename?: 'sequent_backend_candidate_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** order by max() on columns of table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Max_Order_By = {
  alias?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Sequent_Backend_Candidate_Min_Fields = {
  __typename?: 'sequent_backend_candidate_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** order by min() on columns of table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Min_Order_By = {
  alias?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** response of any mutation on the table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Mutation_Response = {
  __typename?: 'sequent_backend_candidate_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Candidate>;
};

/** on_conflict condition type for table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_On_Conflict = {
  constraint: Sequent_Backend_Candidate_Constraint;
  update_columns?: Array<Sequent_Backend_Candidate_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.candidate". */
export type Sequent_Backend_Candidate_Order_By = {
  alias?: InputMaybe<Order_By>;
  annotations?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  is_public?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.candidate */
export type Sequent_Backend_Candidate_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Candidate_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.candidate" */
export enum Sequent_Backend_Candidate_Select_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsPublic = 'is_public',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

/** select "sequent_backend_candidate_aggregate_bool_exp_bool_and_arguments_columns" columns of table "sequent_backend.candidate" */
export enum Sequent_Backend_Candidate_Select_Column_Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_And_Arguments_Columns {
  /** column name */
  IsPublic = 'is_public'
}

/** select "sequent_backend_candidate_aggregate_bool_exp_bool_or_arguments_columns" columns of table "sequent_backend.candidate" */
export enum Sequent_Backend_Candidate_Select_Column_Sequent_Backend_Candidate_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns {
  /** column name */
  IsPublic = 'is_public'
}

/** input type for updating data in table "sequent_backend.candidate" */
export type Sequent_Backend_Candidate_Set_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_candidate" */
export type Sequent_Backend_Candidate_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Candidate_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Candidate_Stream_Cursor_Value_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.candidate" */
export enum Sequent_Backend_Candidate_Update_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsPublic = 'is_public',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

export type Sequent_Backend_Candidate_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Candidate_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Candidate_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Candidate_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Candidate_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Candidate_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Candidate_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Candidate_Bool_Exp;
};

/** columns and relationships of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote = {
  __typename?: 'sequent_backend_cast_vote';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  cast_ballot_signature?: Maybe<Scalars['bytea']['output']>;
  content?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id: Scalars['uuid']['output'];
  voter_id_string?: Maybe<Scalars['String']['output']>;
};


/** columns and relationships of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_VoteAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_VoteLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Aggregate = {
  __typename?: 'sequent_backend_cast_vote_aggregate';
  aggregate?: Maybe<Sequent_Backend_Cast_Vote_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Cast_Vote>;
};

/** aggregate fields of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Aggregate_Fields = {
  __typename?: 'sequent_backend_cast_vote_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Cast_Vote_Max_Fields>;
  min?: Maybe<Sequent_Backend_Cast_Vote_Min_Fields>;
};


/** aggregate fields of "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Cast_Vote_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.cast_vote". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Cast_Vote_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  cast_ballot_signature?: InputMaybe<Bytea_Comparison_Exp>;
  content?: InputMaybe<String_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  voter_id_string?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.cast_vote" */
export enum Sequent_Backend_Cast_Vote_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  CastVotePkey = 'cast_vote_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Cast_Vote_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Cast_Vote_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Cast_Vote_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  content?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voter_id_string?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Cast_Vote_Max_Fields = {
  __typename?: 'sequent_backend_cast_vote_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  content?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voter_id_string?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Cast_Vote_Min_Fields = {
  __typename?: 'sequent_backend_cast_vote_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  content?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voter_id_string?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Mutation_Response = {
  __typename?: 'sequent_backend_cast_vote_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Cast_Vote>;
};

/** on_conflict condition type for table "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_On_Conflict = {
  constraint: Sequent_Backend_Cast_Vote_Constraint;
  update_columns?: Array<Sequent_Backend_Cast_Vote_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.cast_vote". */
export type Sequent_Backend_Cast_Vote_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  cast_ballot_signature?: InputMaybe<Order_By>;
  content?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  voter_id_string?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.cast_vote */
export type Sequent_Backend_Cast_Vote_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Cast_Vote_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.cast_vote" */
export enum Sequent_Backend_Cast_Vote_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CastBallotSignature = 'cast_ballot_signature',
  /** column name */
  Content = 'content',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VoterIdString = 'voter_id_string'
}

/** input type for updating data in table "sequent_backend.cast_vote" */
export type Sequent_Backend_Cast_Vote_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  content?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voter_id_string?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_cast_vote" */
export type Sequent_Backend_Cast_Vote_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Cast_Vote_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Cast_Vote_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_ballot_signature?: InputMaybe<Scalars['bytea']['input']>;
  content?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voter_id_string?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.cast_vote" */
export enum Sequent_Backend_Cast_Vote_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CastBallotSignature = 'cast_ballot_signature',
  /** column name */
  Content = 'content',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VoterIdString = 'voter_id_string'
}

export type Sequent_Backend_Cast_Vote_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Cast_Vote_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Cast_Vote_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Cast_Vote_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Cast_Vote_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Cast_Vote_Bool_Exp;
};

/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_Contest = {
  __typename?: 'sequent_backend_contest';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  /** An array relationship */
  candidates: Array<Sequent_Backend_Candidate>;
  /** An aggregate relationship */
  candidates_aggregate: Sequent_Backend_Candidate_Aggregate;
  conditions?: Maybe<Scalars['jsonb']['output']>;
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  image_document_id?: Maybe<Scalars['String']['output']>;
  is_acclaimed?: Maybe<Scalars['Boolean']['output']>;
  is_active?: Maybe<Scalars['Boolean']['output']>;
  is_encrypted?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  max_votes?: Maybe<Scalars['Int']['output']>;
  min_votes?: Maybe<Scalars['Int']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  order_answers?: Maybe<Scalars['String']['output']>;
  presentation?: Maybe<Scalars['jsonb']['output']>;
  tally_configuration?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  voting_type?: Maybe<Scalars['String']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestCandidatesArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestCandidates_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestConditionsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestPresentationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.contest" */
export type Sequent_Backend_ContestTally_ConfigurationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.contest" */
export type Sequent_Backend_Contest_Aggregate = {
  __typename?: 'sequent_backend_contest_aggregate';
  aggregate?: Maybe<Sequent_Backend_Contest_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Contest>;
};

export type Sequent_Backend_Contest_Aggregate_Bool_Exp = {
  bool_and?: InputMaybe<Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_And>;
  bool_or?: InputMaybe<Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_Or>;
  count?: InputMaybe<Sequent_Backend_Contest_Aggregate_Bool_Exp_Count>;
};

export type Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_And = {
  arguments: Sequent_Backend_Contest_Select_Column_Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_And_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_Or = {
  arguments: Sequent_Backend_Contest_Select_Column_Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Contest_Aggregate_Bool_Exp_Count = {
  arguments?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  predicate: Int_Comparison_Exp;
};

/** aggregate fields of "sequent_backend.contest" */
export type Sequent_Backend_Contest_Aggregate_Fields = {
  __typename?: 'sequent_backend_contest_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Contest_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Contest_Max_Fields>;
  min?: Maybe<Sequent_Backend_Contest_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Contest_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Contest_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Contest_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Contest_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Contest_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Contest_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Contest_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.contest" */
export type Sequent_Backend_Contest_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** order by aggregate values of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Aggregate_Order_By = {
  avg?: InputMaybe<Sequent_Backend_Contest_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Sequent_Backend_Contest_Max_Order_By>;
  min?: InputMaybe<Sequent_Backend_Contest_Min_Order_By>;
  stddev?: InputMaybe<Sequent_Backend_Contest_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Sequent_Backend_Contest_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Sequent_Backend_Contest_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Sequent_Backend_Contest_Sum_Order_By>;
  var_pop?: InputMaybe<Sequent_Backend_Contest_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Sequent_Backend_Contest_Var_Samp_Order_By>;
  variance?: InputMaybe<Sequent_Backend_Contest_Variance_Order_By>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Contest_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  conditions?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
};

/** input type for inserting array relation for remote table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Arr_Rel_Insert_Input = {
  data: Array<Sequent_Backend_Contest_Insert_Input>;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Contest_On_Conflict>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Contest_Avg_Fields = {
  __typename?: 'sequent_backend_contest_avg_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by avg() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Avg_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "sequent_backend.contest". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Contest_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Contest_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Contest_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  candidates?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
  candidates_aggregate?: InputMaybe<Sequent_Backend_Candidate_Aggregate_Bool_Exp>;
  conditions?: InputMaybe<Jsonb_Comparison_Exp>;
  counting_algorithm?: InputMaybe<String_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  image_document_id?: InputMaybe<String_Comparison_Exp>;
  is_acclaimed?: InputMaybe<Boolean_Comparison_Exp>;
  is_active?: InputMaybe<Boolean_Comparison_Exp>;
  is_encrypted?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  max_votes?: InputMaybe<Int_Comparison_Exp>;
  min_votes?: InputMaybe<Int_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  order_answers?: InputMaybe<String_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
  tally_configuration?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  voting_type?: InputMaybe<String_Comparison_Exp>;
  winning_candidates_num?: InputMaybe<Int_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.contest" */
export enum Sequent_Backend_Contest_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  ContestPkey = 'contest_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Contest_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  conditions?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
  tally_configuration?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Contest_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  conditions?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
  tally_configuration?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Contest_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  conditions?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
  tally_configuration?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Inc_Input = {
  max_votes?: InputMaybe<Scalars['Int']['input']>;
  min_votes?: InputMaybe<Scalars['Int']['input']>;
  winning_candidates_num?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  candidates?: InputMaybe<Sequent_Backend_Candidate_Arr_Rel_Insert_Input>;
  conditions?: InputMaybe<Scalars['jsonb']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_acclaimed?: InputMaybe<Scalars['Boolean']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  is_encrypted?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  max_votes?: InputMaybe<Scalars['Int']['input']>;
  min_votes?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  order_answers?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
  winning_candidates_num?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Contest_Max_Fields = {
  __typename?: 'sequent_backend_contest_max_fields';
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  max_votes?: Maybe<Scalars['Int']['output']>;
  min_votes?: Maybe<Scalars['Int']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  order_answers?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};

/** order by max() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Max_Order_By = {
  counting_algorithm?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  order_answers?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  voting_type?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Sequent_Backend_Contest_Min_Fields = {
  __typename?: 'sequent_backend_contest_min_fields';
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  max_votes?: Maybe<Scalars['Int']['output']>;
  min_votes?: Maybe<Scalars['Int']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  order_answers?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};

/** order by min() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Min_Order_By = {
  counting_algorithm?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  order_answers?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  voting_type?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** response of any mutation on the table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Mutation_Response = {
  __typename?: 'sequent_backend_contest_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Contest>;
};

/** input type for inserting object relation for remote table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Obj_Rel_Insert_Input = {
  data: Sequent_Backend_Contest_Insert_Input;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Contest_On_Conflict>;
};

/** on_conflict condition type for table "sequent_backend.contest" */
export type Sequent_Backend_Contest_On_Conflict = {
  constraint: Sequent_Backend_Contest_Constraint;
  update_columns?: Array<Sequent_Backend_Contest_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.contest". */
export type Sequent_Backend_Contest_Order_By = {
  annotations?: InputMaybe<Order_By>;
  candidates_aggregate?: InputMaybe<Sequent_Backend_Candidate_Aggregate_Order_By>;
  conditions?: InputMaybe<Order_By>;
  counting_algorithm?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  is_acclaimed?: InputMaybe<Order_By>;
  is_active?: InputMaybe<Order_By>;
  is_encrypted?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  order_answers?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
  tally_configuration?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  voting_type?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.contest */
export type Sequent_Backend_Contest_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Contest_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  conditions?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.contest" */
export enum Sequent_Backend_Contest_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  Conditions = 'conditions',
  /** column name */
  CountingAlgorithm = 'counting_algorithm',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsAcclaimed = 'is_acclaimed',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  IsEncrypted = 'is_encrypted',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  MaxVotes = 'max_votes',
  /** column name */
  MinVotes = 'min_votes',
  /** column name */
  Name = 'name',
  /** column name */
  OrderAnswers = 'order_answers',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  TallyConfiguration = 'tally_configuration',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VotingType = 'voting_type',
  /** column name */
  WinningCandidatesNum = 'winning_candidates_num'
}

/** select "sequent_backend_contest_aggregate_bool_exp_bool_and_arguments_columns" columns of table "sequent_backend.contest" */
export enum Sequent_Backend_Contest_Select_Column_Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_And_Arguments_Columns {
  /** column name */
  IsAcclaimed = 'is_acclaimed',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  IsEncrypted = 'is_encrypted'
}

/** select "sequent_backend_contest_aggregate_bool_exp_bool_or_arguments_columns" columns of table "sequent_backend.contest" */
export enum Sequent_Backend_Contest_Select_Column_Sequent_Backend_Contest_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns {
  /** column name */
  IsAcclaimed = 'is_acclaimed',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  IsEncrypted = 'is_encrypted'
}

/** input type for updating data in table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  conditions?: InputMaybe<Scalars['jsonb']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_acclaimed?: InputMaybe<Scalars['Boolean']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  is_encrypted?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  max_votes?: InputMaybe<Scalars['Int']['input']>;
  min_votes?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  order_answers?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
  winning_candidates_num?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Contest_Stddev_Fields = {
  __typename?: 'sequent_backend_contest_stddev_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Stddev_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Contest_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_contest_stddev_pop_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev_pop() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Stddev_Pop_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Contest_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_contest_stddev_samp_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev_samp() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Stddev_Samp_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "sequent_backend_contest" */
export type Sequent_Backend_Contest_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Contest_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Contest_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  conditions?: InputMaybe<Scalars['jsonb']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_acclaimed?: InputMaybe<Scalars['Boolean']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  is_encrypted?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  max_votes?: InputMaybe<Scalars['Int']['input']>;
  min_votes?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  order_answers?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
  winning_candidates_num?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Contest_Sum_Fields = {
  __typename?: 'sequent_backend_contest_sum_fields';
  max_votes?: Maybe<Scalars['Int']['output']>;
  min_votes?: Maybe<Scalars['Int']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};

/** order by sum() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Sum_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** update columns of table "sequent_backend.contest" */
export enum Sequent_Backend_Contest_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  Conditions = 'conditions',
  /** column name */
  CountingAlgorithm = 'counting_algorithm',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsAcclaimed = 'is_acclaimed',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  IsEncrypted = 'is_encrypted',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  MaxVotes = 'max_votes',
  /** column name */
  MinVotes = 'min_votes',
  /** column name */
  Name = 'name',
  /** column name */
  OrderAnswers = 'order_answers',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  TallyConfiguration = 'tally_configuration',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VotingType = 'voting_type',
  /** column name */
  WinningCandidatesNum = 'winning_candidates_num'
}

export type Sequent_Backend_Contest_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Contest_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Contest_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Contest_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Contest_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Contest_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Contest_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Contest_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Contest_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Contest_Var_Pop_Fields = {
  __typename?: 'sequent_backend_contest_var_pop_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by var_pop() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Var_Pop_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Contest_Var_Samp_Fields = {
  __typename?: 'sequent_backend_contest_var_samp_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by var_samp() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Var_Samp_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Contest_Variance_Fields = {
  __typename?: 'sequent_backend_contest_variance_fields';
  max_votes?: Maybe<Scalars['Float']['output']>;
  min_votes?: Maybe<Scalars['Float']['output']>;
  winning_candidates_num?: Maybe<Scalars['Float']['output']>;
};

/** order by variance() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Variance_Order_By = {
  max_votes?: InputMaybe<Order_By>;
  min_votes?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** columns and relationships of "sequent_backend.document" */
export type Sequent_Backend_Document = {
  __typename?: 'sequent_backend_document';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  is_public?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  media_type?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  size?: Maybe<Scalars['Int']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.document" */
export type Sequent_Backend_DocumentAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.document" */
export type Sequent_Backend_DocumentLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.document" */
export type Sequent_Backend_Document_Aggregate = {
  __typename?: 'sequent_backend_document_aggregate';
  aggregate?: Maybe<Sequent_Backend_Document_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Document>;
};

/** aggregate fields of "sequent_backend.document" */
export type Sequent_Backend_Document_Aggregate_Fields = {
  __typename?: 'sequent_backend_document_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Document_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Document_Max_Fields>;
  min?: Maybe<Sequent_Backend_Document_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Document_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Document_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Document_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Document_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Document_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Document_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Document_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.document" */
export type Sequent_Backend_Document_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Document_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Document_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Document_Avg_Fields = {
  __typename?: 'sequent_backend_document_avg_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.document". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Document_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Document_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Document_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_public?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  media_type?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  size?: InputMaybe<Int_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.document" */
export enum Sequent_Backend_Document_Constraint {
  /** unique or primary key constraint on columns "id" */
  ElectionDocumentPkey = 'election_document_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Document_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Document_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Document_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.document" */
export type Sequent_Backend_Document_Inc_Input = {
  size?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.document" */
export type Sequent_Backend_Document_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  media_type?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  size?: InputMaybe<Scalars['Int']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Document_Max_Fields = {
  __typename?: 'sequent_backend_document_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  media_type?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  size?: Maybe<Scalars['Int']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Document_Min_Fields = {
  __typename?: 'sequent_backend_document_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  media_type?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  size?: Maybe<Scalars['Int']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.document" */
export type Sequent_Backend_Document_Mutation_Response = {
  __typename?: 'sequent_backend_document_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Document>;
};

/** on_conflict condition type for table "sequent_backend.document" */
export type Sequent_Backend_Document_On_Conflict = {
  constraint: Sequent_Backend_Document_Constraint;
  update_columns?: Array<Sequent_Backend_Document_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.document". */
export type Sequent_Backend_Document_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_public?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  media_type?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  size?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.document */
export type Sequent_Backend_Document_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Document_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.document" */
export enum Sequent_Backend_Document_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  IsPublic = 'is_public',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  MediaType = 'media_type',
  /** column name */
  Name = 'name',
  /** column name */
  Size = 'size',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.document" */
export type Sequent_Backend_Document_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  media_type?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  size?: InputMaybe<Scalars['Int']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Document_Stddev_Fields = {
  __typename?: 'sequent_backend_document_stddev_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Document_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_document_stddev_pop_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Document_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_document_stddev_samp_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_document" */
export type Sequent_Backend_Document_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Document_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Document_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_public?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  media_type?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  size?: InputMaybe<Scalars['Int']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Document_Sum_Fields = {
  __typename?: 'sequent_backend_document_sum_fields';
  size?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.document" */
export enum Sequent_Backend_Document_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  IsPublic = 'is_public',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  MediaType = 'media_type',
  /** column name */
  Name = 'name',
  /** column name */
  Size = 'size',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Document_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Document_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Document_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Document_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Document_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Document_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Document_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Document_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Document_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Document_Var_Pop_Fields = {
  __typename?: 'sequent_backend_document_var_pop_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Document_Var_Samp_Fields = {
  __typename?: 'sequent_backend_document_var_samp_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Document_Variance_Fields = {
  __typename?: 'sequent_backend_document_variance_fields';
  size?: Maybe<Scalars['Float']['output']>;
};

/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_Election = {
  __typename?: 'sequent_backend_election';
  alias?: Maybe<Scalars['String']['output']>;
  annotations?: Maybe<Scalars['jsonb']['output']>;
  /** An array relationship */
  contests: Array<Sequent_Backend_Contest>;
  /** An aggregate relationship */
  contests_aggregate: Sequent_Backend_Contest_Aggregate;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  dates?: Maybe<Scalars['jsonb']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  eml?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  image_document_id?: Maybe<Scalars['String']['output']>;
  is_consolidated_ballot_encoding?: Maybe<Scalars['Boolean']['output']>;
  is_kiosk?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name: Scalars['String']['output'];
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  presentation?: Maybe<Scalars['jsonb']['output']>;
  spoil_ballot_option?: Maybe<Scalars['Boolean']['output']>;
  status?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  voting_channels?: Maybe<Scalars['jsonb']['output']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionContestsArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionContests_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionDatesArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionPresentationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionStatusArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionVoting_ChannelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.election" */
export type Sequent_Backend_Election_Aggregate = {
  __typename?: 'sequent_backend_election_aggregate';
  aggregate?: Maybe<Sequent_Backend_Election_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Election>;
};

export type Sequent_Backend_Election_Aggregate_Bool_Exp = {
  bool_and?: InputMaybe<Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_And>;
  bool_or?: InputMaybe<Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_Or>;
  count?: InputMaybe<Sequent_Backend_Election_Aggregate_Bool_Exp_Count>;
};

export type Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_And = {
  arguments: Sequent_Backend_Election_Select_Column_Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_And_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_Or = {
  arguments: Sequent_Backend_Election_Select_Column_Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
  predicate: Boolean_Comparison_Exp;
};

export type Sequent_Backend_Election_Aggregate_Bool_Exp_Count = {
  arguments?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
  predicate: Int_Comparison_Exp;
};

/** aggregate fields of "sequent_backend.election" */
export type Sequent_Backend_Election_Aggregate_Fields = {
  __typename?: 'sequent_backend_election_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Election_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Election_Max_Fields>;
  min?: Maybe<Sequent_Backend_Election_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Election_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Election_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Election_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Election_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Election_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Election_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Election_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.election" */
export type Sequent_Backend_Election_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** order by aggregate values of table "sequent_backend.election" */
export type Sequent_Backend_Election_Aggregate_Order_By = {
  avg?: InputMaybe<Sequent_Backend_Election_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Sequent_Backend_Election_Max_Order_By>;
  min?: InputMaybe<Sequent_Backend_Election_Min_Order_By>;
  stddev?: InputMaybe<Sequent_Backend_Election_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Sequent_Backend_Election_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Sequent_Backend_Election_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Sequent_Backend_Election_Sum_Order_By>;
  var_pop?: InputMaybe<Sequent_Backend_Election_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Sequent_Backend_Election_Var_Samp_Order_By>;
  variance?: InputMaybe<Sequent_Backend_Election_Variance_Order_By>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** input type for inserting array relation for remote table "sequent_backend.election" */
export type Sequent_Backend_Election_Arr_Rel_Insert_Input = {
  data: Array<Sequent_Backend_Election_Insert_Input>;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Election_On_Conflict>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Election_Avg_Fields = {
  __typename?: 'sequent_backend_election_avg_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by avg() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Avg_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "sequent_backend.election". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Election_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Election_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Election_Bool_Exp>>;
  alias?: InputMaybe<String_Comparison_Exp>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  contests?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
  contests_aggregate?: InputMaybe<Sequent_Backend_Contest_Aggregate_Bool_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  dates?: InputMaybe<Jsonb_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  eml?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  image_document_id?: InputMaybe<String_Comparison_Exp>;
  is_consolidated_ballot_encoding?: InputMaybe<Boolean_Comparison_Exp>;
  is_kiosk?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  num_allowed_revotes?: InputMaybe<Int_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
  spoil_ballot_option?: InputMaybe<Boolean_Comparison_Exp>;
  status?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  voting_channels?: InputMaybe<Jsonb_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.election" */
export enum Sequent_Backend_Election_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  ElectionPkey = 'election_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Election_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  dates?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  dates?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  dates?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['String']['input']>;
};

/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event = {
  __typename?: 'sequent_backend_election_event';
  alias?: Maybe<Scalars['String']['output']>;
  annotations?: Maybe<Scalars['jsonb']['output']>;
  audit_election_event_id?: Maybe<Scalars['uuid']['output']>;
  bulletin_board_reference?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  dates?: Maybe<Scalars['jsonb']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  /** An array relationship */
  elections: Array<Sequent_Backend_Election>;
  /** An aggregate relationship */
  elections_aggregate: Sequent_Backend_Election_Aggregate;
  encryption_protocol: Scalars['String']['output'];
  id: Scalars['uuid']['output'];
  is_archived: Scalars['Boolean']['output'];
  is_audit?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  name: Scalars['String']['output'];
  presentation?: Maybe<Scalars['jsonb']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  status?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
  user_boards?: Maybe<Scalars['String']['output']>;
  voting_channels?: Maybe<Scalars['jsonb']['output']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventBulletin_Board_ReferenceArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventDatesArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventElectionsArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventElections_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventPresentationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventStatusArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_event" */
export type Sequent_Backend_Election_EventVoting_ChannelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Aggregate = {
  __typename?: 'sequent_backend_election_event_aggregate';
  aggregate?: Maybe<Sequent_Backend_Election_Event_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Election_Event>;
};

/** aggregate fields of "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Aggregate_Fields = {
  __typename?: 'sequent_backend_election_event_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Election_Event_Max_Fields>;
  min?: Maybe<Sequent_Backend_Election_Event_Min_Fields>;
};


/** aggregate fields of "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Election_Event_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Event_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.election_event". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Election_Event_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Election_Event_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Election_Event_Bool_Exp>>;
  alias?: InputMaybe<String_Comparison_Exp>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  audit_election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  bulletin_board_reference?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  dates?: InputMaybe<Jsonb_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  elections?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
  elections_aggregate?: InputMaybe<Sequent_Backend_Election_Aggregate_Bool_Exp>;
  encryption_protocol?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_archived?: InputMaybe<Boolean_Comparison_Exp>;
  is_audit?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
  public_key?: InputMaybe<String_Comparison_Exp>;
  status?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  user_boards?: InputMaybe<String_Comparison_Exp>;
  voting_channels?: InputMaybe<Jsonb_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.election_event" */
export enum Sequent_Backend_Election_Event_Constraint {
  /** unique or primary key constraint on columns "id" */
  EventPkey = 'event_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Election_Event_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  bulletin_board_reference?: InputMaybe<Array<Scalars['String']['input']>>;
  dates?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Event_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['Int']['input']>;
  dates?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Event_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['String']['input']>;
  dates?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Insert_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  audit_election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  elections?: InputMaybe<Sequent_Backend_Election_Arr_Rel_Insert_Input>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  user_boards?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Election_Event_Max_Fields = {
  __typename?: 'sequent_backend_election_event_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
  audit_election_event_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  encryption_protocol?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
  user_boards?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Election_Event_Min_Fields = {
  __typename?: 'sequent_backend_election_event_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
  audit_election_event_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  encryption_protocol?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
  user_boards?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Mutation_Response = {
  __typename?: 'sequent_backend_election_event_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Election_Event>;
};

/** on_conflict condition type for table "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_On_Conflict = {
  constraint: Sequent_Backend_Election_Event_Constraint;
  update_columns?: Array<Sequent_Backend_Election_Event_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.election_event". */
export type Sequent_Backend_Election_Event_Order_By = {
  alias?: InputMaybe<Order_By>;
  annotations?: InputMaybe<Order_By>;
  audit_election_event_id?: InputMaybe<Order_By>;
  bulletin_board_reference?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  dates?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  elections_aggregate?: InputMaybe<Sequent_Backend_Election_Aggregate_Order_By>;
  encryption_protocol?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_archived?: InputMaybe<Order_By>;
  is_audit?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
  public_key?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
  user_boards?: InputMaybe<Order_By>;
  voting_channels?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.election_event */
export type Sequent_Backend_Election_Event_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Event_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.election_event" */
export enum Sequent_Backend_Election_Event_Select_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AuditElectionEventId = 'audit_election_event_id',
  /** column name */
  BulletinBoardReference = 'bulletin_board_reference',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Dates = 'dates',
  /** column name */
  Description = 'description',
  /** column name */
  EncryptionProtocol = 'encryption_protocol',
  /** column name */
  Id = 'id',
  /** column name */
  IsArchived = 'is_archived',
  /** column name */
  IsAudit = 'is_audit',
  /** column name */
  Labels = 'labels',
  /** column name */
  Name = 'name',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  PublicKey = 'public_key',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  UserBoards = 'user_boards',
  /** column name */
  VotingChannels = 'voting_channels'
}

/** input type for updating data in table "sequent_backend.election_event" */
export type Sequent_Backend_Election_Event_Set_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  audit_election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  user_boards?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Streaming cursor of the table "sequent_backend_election_event" */
export type Sequent_Backend_Election_Event_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Election_Event_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Election_Event_Stream_Cursor_Value_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  audit_election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  user_boards?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** update columns of table "sequent_backend.election_event" */
export enum Sequent_Backend_Election_Event_Update_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AuditElectionEventId = 'audit_election_event_id',
  /** column name */
  BulletinBoardReference = 'bulletin_board_reference',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Dates = 'dates',
  /** column name */
  Description = 'description',
  /** column name */
  EncryptionProtocol = 'encryption_protocol',
  /** column name */
  Id = 'id',
  /** column name */
  IsArchived = 'is_archived',
  /** column name */
  IsAudit = 'is_audit',
  /** column name */
  Labels = 'labels',
  /** column name */
  Name = 'name',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  PublicKey = 'public_key',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  UserBoards = 'user_boards',
  /** column name */
  VotingChannels = 'voting_channels'
}

export type Sequent_Backend_Election_Event_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Election_Event_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Event_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Event_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Election_Event_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Election_Event_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Election_Event_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Election_Event_Bool_Exp;
};

/** input type for incrementing numeric columns in table "sequent_backend.election" */
export type Sequent_Backend_Election_Inc_Input = {
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.election" */
export type Sequent_Backend_Election_Insert_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  contests?: InputMaybe<Sequent_Backend_Contest_Arr_Rel_Insert_Input>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Election_Max_Fields = {
  __typename?: 'sequent_backend_election_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  eml?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** order by max() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Max_Order_By = {
  alias?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  eml?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Sequent_Backend_Election_Min_Fields = {
  __typename?: 'sequent_backend_election_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  eml?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  image_document_id?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** order by min() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Min_Order_By = {
  alias?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  eml?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** response of any mutation on the table "sequent_backend.election" */
export type Sequent_Backend_Election_Mutation_Response = {
  __typename?: 'sequent_backend_election_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Election>;
};

/** on_conflict condition type for table "sequent_backend.election" */
export type Sequent_Backend_Election_On_Conflict = {
  constraint: Sequent_Backend_Election_Constraint;
  update_columns?: Array<Sequent_Backend_Election_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.election". */
export type Sequent_Backend_Election_Order_By = {
  alias?: InputMaybe<Order_By>;
  annotations?: InputMaybe<Order_By>;
  contests_aggregate?: InputMaybe<Sequent_Backend_Contest_Aggregate_Order_By>;
  created_at?: InputMaybe<Order_By>;
  dates?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  eml?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  is_consolidated_ballot_encoding?: InputMaybe<Order_By>;
  is_kiosk?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
  spoil_ballot_option?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  voting_channels?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.election */
export type Sequent_Backend_Election_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** columns and relationships of "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result = {
  __typename?: 'sequent_backend_election_result';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  result_eml?: Maybe<Scalars['String']['output']>;
  result_eml_signature?: Maybe<Scalars['bytea']['output']>;
  statistics?: Maybe<Scalars['jsonb']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.election_result" */
export type Sequent_Backend_Election_ResultAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_result" */
export type Sequent_Backend_Election_ResultLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_result" */
export type Sequent_Backend_Election_ResultStatisticsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Aggregate = {
  __typename?: 'sequent_backend_election_result_aggregate';
  aggregate?: Maybe<Sequent_Backend_Election_Result_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Election_Result>;
};

/** aggregate fields of "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Aggregate_Fields = {
  __typename?: 'sequent_backend_election_result_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Election_Result_Max_Fields>;
  min?: Maybe<Sequent_Backend_Election_Result_Min_Fields>;
};


/** aggregate fields of "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Election_Result_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Result_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.election_result". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Election_Result_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Election_Result_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Election_Result_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  result_eml?: InputMaybe<String_Comparison_Exp>;
  result_eml_signature?: InputMaybe<Bytea_Comparison_Exp>;
  statistics?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.election_result" */
export enum Sequent_Backend_Election_Result_Constraint {
  /** unique or primary key constraint on columns "id" */
  ElectionResultPkey = 'election_result_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Election_Result_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  statistics?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Result_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  statistics?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Result_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  statistics?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  result_eml?: InputMaybe<Scalars['String']['input']>;
  result_eml_signature?: InputMaybe<Scalars['bytea']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Election_Result_Max_Fields = {
  __typename?: 'sequent_backend_election_result_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  result_eml?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Election_Result_Min_Fields = {
  __typename?: 'sequent_backend_election_result_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  result_eml?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Mutation_Response = {
  __typename?: 'sequent_backend_election_result_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Election_Result>;
};

/** on_conflict condition type for table "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_On_Conflict = {
  constraint: Sequent_Backend_Election_Result_Constraint;
  update_columns?: Array<Sequent_Backend_Election_Result_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.election_result". */
export type Sequent_Backend_Election_Result_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  result_eml?: InputMaybe<Order_By>;
  result_eml_signature?: InputMaybe<Order_By>;
  statistics?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.election_result */
export type Sequent_Backend_Election_Result_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Result_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.election_result" */
export enum Sequent_Backend_Election_Result_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultEml = 'result_eml',
  /** column name */
  ResultEmlSignature = 'result_eml_signature',
  /** column name */
  Statistics = 'statistics',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.election_result" */
export type Sequent_Backend_Election_Result_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  result_eml?: InputMaybe<Scalars['String']['input']>;
  result_eml_signature?: InputMaybe<Scalars['bytea']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_election_result" */
export type Sequent_Backend_Election_Result_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Election_Result_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Election_Result_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  result_eml?: InputMaybe<Scalars['String']['input']>;
  result_eml_signature?: InputMaybe<Scalars['bytea']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.election_result" */
export enum Sequent_Backend_Election_Result_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultEml = 'result_eml',
  /** column name */
  ResultEmlSignature = 'result_eml_signature',
  /** column name */
  Statistics = 'statistics',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Election_Result_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Election_Result_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Result_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Result_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Election_Result_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Election_Result_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Election_Result_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Election_Result_Bool_Exp;
};

/** select columns of table "sequent_backend.election" */
export enum Sequent_Backend_Election_Select_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Dates = 'dates',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Eml = 'eml',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  NumAllowedRevotes = 'num_allowed_revotes',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VotingChannels = 'voting_channels'
}

/** select "sequent_backend_election_aggregate_bool_exp_bool_and_arguments_columns" columns of table "sequent_backend.election" */
export enum Sequent_Backend_Election_Select_Column_Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_And_Arguments_Columns {
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option'
}

/** select "sequent_backend_election_aggregate_bool_exp_bool_or_arguments_columns" columns of table "sequent_backend.election" */
export enum Sequent_Backend_Election_Select_Column_Sequent_Backend_Election_Aggregate_Bool_Exp_Bool_Or_Arguments_Columns {
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option'
}

/** input type for updating data in table "sequent_backend.election" */
export type Sequent_Backend_Election_Set_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Election_Stddev_Fields = {
  __typename?: 'sequent_backend_election_stddev_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Stddev_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Election_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_election_stddev_pop_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev_pop() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Stddev_Pop_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Election_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_election_stddev_samp_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by stddev_samp() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Stddev_Samp_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "sequent_backend_election" */
export type Sequent_Backend_Election_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Election_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Election_Stream_Cursor_Value_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  dates?: InputMaybe<Scalars['jsonb']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Election_Sum_Fields = {
  __typename?: 'sequent_backend_election_sum_fields';
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
};

/** order by sum() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Sum_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** columns and relationships of "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type = {
  __typename?: 'sequent_backend_election_type';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  name: Scalars['String']['output'];
  tenant_id: Scalars['uuid']['output'];
  updated_at: Scalars['timestamptz']['output'];
};


/** columns and relationships of "sequent_backend.election_type" */
export type Sequent_Backend_Election_TypeAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election_type" */
export type Sequent_Backend_Election_TypeLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Aggregate = {
  __typename?: 'sequent_backend_election_type_aggregate';
  aggregate?: Maybe<Sequent_Backend_Election_Type_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Election_Type>;
};

/** aggregate fields of "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Aggregate_Fields = {
  __typename?: 'sequent_backend_election_type_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Election_Type_Max_Fields>;
  min?: Maybe<Sequent_Backend_Election_Type_Min_Fields>;
};


/** aggregate fields of "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Election_Type_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Type_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.election_type". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Election_Type_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Election_Type_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Election_Type_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.election_type" */
export enum Sequent_Backend_Election_Type_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id" */
  ElectionTypePkey = 'election_type_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Election_Type_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Type_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Type_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Election_Type_Max_Fields = {
  __typename?: 'sequent_backend_election_type_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Election_Type_Min_Fields = {
  __typename?: 'sequent_backend_election_type_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** response of any mutation on the table "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Mutation_Response = {
  __typename?: 'sequent_backend_election_type_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Election_Type>;
};

/** on_conflict condition type for table "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_On_Conflict = {
  constraint: Sequent_Backend_Election_Type_Constraint;
  update_columns?: Array<Sequent_Backend_Election_Type_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.election_type". */
export type Sequent_Backend_Election_Type_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.election_type */
export type Sequent_Backend_Election_Type_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Election_Type_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.election_type" */
export enum Sequent_Backend_Election_Type_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Name = 'name',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at'
}

/** input type for updating data in table "sequent_backend.election_type" */
export type Sequent_Backend_Election_Type_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** Streaming cursor of the table "sequent_backend_election_type" */
export type Sequent_Backend_Election_Type_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Election_Type_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Election_Type_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** update columns of table "sequent_backend.election_type" */
export enum Sequent_Backend_Election_Type_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Name = 'name',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at'
}

export type Sequent_Backend_Election_Type_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Election_Type_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Type_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Type_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Election_Type_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Election_Type_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Election_Type_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Election_Type_Bool_Exp;
};

/** update columns of table "sequent_backend.election" */
export enum Sequent_Backend_Election_Update_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Dates = 'dates',
  /** column name */
  Description = 'description',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Eml = 'eml',
  /** column name */
  Id = 'id',
  /** column name */
  ImageDocumentId = 'image_document_id',
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  NumAllowedRevotes = 'num_allowed_revotes',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  VotingChannels = 'voting_channels'
}

export type Sequent_Backend_Election_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Election_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Election_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Election_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Election_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Election_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Election_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Election_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Election_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Election_Var_Pop_Fields = {
  __typename?: 'sequent_backend_election_var_pop_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by var_pop() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Var_Pop_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Election_Var_Samp_Fields = {
  __typename?: 'sequent_backend_election_var_samp_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by var_samp() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Var_Samp_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Election_Variance_Fields = {
  __typename?: 'sequent_backend_election_variance_fields';
  num_allowed_revotes?: Maybe<Scalars['Float']['output']>;
};

/** order by variance() on columns of table "sequent_backend.election" */
export type Sequent_Backend_Election_Variance_Order_By = {
  num_allowed_revotes?: InputMaybe<Order_By>;
};

/** columns and relationships of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution = {
  __typename?: 'sequent_backend_event_execution';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  ended_at?: Maybe<Scalars['timestamptz']['output']>;
  execution_payload?: Maybe<Scalars['jsonb']['output']>;
  execution_state?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  result_payload?: Maybe<Scalars['jsonb']['output']>;
  scheduled_event_id: Scalars['uuid']['output'];
  started_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_ExecutionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_ExecutionExecution_PayloadArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_ExecutionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_ExecutionResult_PayloadArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Aggregate = {
  __typename?: 'sequent_backend_event_execution_aggregate';
  aggregate?: Maybe<Sequent_Backend_Event_Execution_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Event_Execution>;
};

/** aggregate fields of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Aggregate_Fields = {
  __typename?: 'sequent_backend_event_execution_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Event_Execution_Max_Fields>;
  min?: Maybe<Sequent_Backend_Event_Execution_Min_Fields>;
};


/** aggregate fields of "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Event_Execution_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Event_Execution_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  execution_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  result_payload?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.event_execution". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Event_Execution_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Event_Execution_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Event_Execution_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  ended_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  execution_payload?: InputMaybe<Jsonb_Comparison_Exp>;
  execution_state?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  result_payload?: InputMaybe<Jsonb_Comparison_Exp>;
  scheduled_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  started_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.event_execution" */
export enum Sequent_Backend_Event_Execution_Constraint {
  /** unique or primary key constraint on columns "id" */
  EventExecutionPkey = 'event_execution_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Event_Execution_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  execution_payload?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  result_payload?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Event_Execution_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  execution_payload?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  result_payload?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Event_Execution_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  execution_payload?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  result_payload?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  ended_at?: InputMaybe<Scalars['timestamptz']['input']>;
  execution_payload?: InputMaybe<Scalars['jsonb']['input']>;
  execution_state?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  result_payload?: InputMaybe<Scalars['jsonb']['input']>;
  scheduled_event_id?: InputMaybe<Scalars['uuid']['input']>;
  started_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Event_Execution_Max_Fields = {
  __typename?: 'sequent_backend_event_execution_max_fields';
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  ended_at?: Maybe<Scalars['timestamptz']['output']>;
  execution_state?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  scheduled_event_id?: Maybe<Scalars['uuid']['output']>;
  started_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Event_Execution_Min_Fields = {
  __typename?: 'sequent_backend_event_execution_min_fields';
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  ended_at?: Maybe<Scalars['timestamptz']['output']>;
  execution_state?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  scheduled_event_id?: Maybe<Scalars['uuid']['output']>;
  started_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Mutation_Response = {
  __typename?: 'sequent_backend_event_execution_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Event_Execution>;
};

/** on_conflict condition type for table "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_On_Conflict = {
  constraint: Sequent_Backend_Event_Execution_Constraint;
  update_columns?: Array<Sequent_Backend_Event_Execution_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.event_execution". */
export type Sequent_Backend_Event_Execution_Order_By = {
  annotations?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  ended_at?: InputMaybe<Order_By>;
  execution_payload?: InputMaybe<Order_By>;
  execution_state?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  result_payload?: InputMaybe<Order_By>;
  scheduled_event_id?: InputMaybe<Order_By>;
  started_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.event_execution */
export type Sequent_Backend_Event_Execution_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Event_Execution_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  execution_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  result_payload?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.event_execution" */
export enum Sequent_Backend_Event_Execution_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EndedAt = 'ended_at',
  /** column name */
  ExecutionPayload = 'execution_payload',
  /** column name */
  ExecutionState = 'execution_state',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  ResultPayload = 'result_payload',
  /** column name */
  ScheduledEventId = 'scheduled_event_id',
  /** column name */
  StartedAt = 'started_at',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.event_execution" */
export type Sequent_Backend_Event_Execution_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  ended_at?: InputMaybe<Scalars['timestamptz']['input']>;
  execution_payload?: InputMaybe<Scalars['jsonb']['input']>;
  execution_state?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  result_payload?: InputMaybe<Scalars['jsonb']['input']>;
  scheduled_event_id?: InputMaybe<Scalars['uuid']['input']>;
  started_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_event_execution" */
export type Sequent_Backend_Event_Execution_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Event_Execution_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Event_Execution_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  ended_at?: InputMaybe<Scalars['timestamptz']['input']>;
  execution_payload?: InputMaybe<Scalars['jsonb']['input']>;
  execution_state?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  result_payload?: InputMaybe<Scalars['jsonb']['input']>;
  scheduled_event_id?: InputMaybe<Scalars['uuid']['input']>;
  started_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.event_execution" */
export enum Sequent_Backend_Event_Execution_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EndedAt = 'ended_at',
  /** column name */
  ExecutionPayload = 'execution_payload',
  /** column name */
  ExecutionState = 'execution_state',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  ResultPayload = 'result_payload',
  /** column name */
  ScheduledEventId = 'scheduled_event_id',
  /** column name */
  StartedAt = 'started_at',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Event_Execution_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Event_Execution_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Event_Execution_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Event_Execution_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Event_Execution_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Event_Execution_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Event_Execution_Bool_Exp;
};

/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony = {
  __typename?: 'sequent_backend_keys_ceremony';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  election_event_id: Scalars['uuid']['output'];
  execution_status?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at: Scalars['timestamptz']['output'];
  status?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  trustee_ids: Array<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyStatusArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Aggregate = {
  __typename?: 'sequent_backend_keys_ceremony_aggregate';
  aggregate?: Maybe<Sequent_Backend_Keys_Ceremony_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Keys_Ceremony>;
};

/** aggregate fields of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Aggregate_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Keys_Ceremony_Max_Fields>;
  min?: Maybe<Sequent_Backend_Keys_Ceremony_Min_Fields>;
};


/** aggregate fields of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Keys_Ceremony_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.keys_ceremony". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Keys_Ceremony_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  execution_status?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  status?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  trustee_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.keys_ceremony" */
export enum Sequent_Backend_Keys_Ceremony_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  KeysCeremonyPkey = 'keys_ceremony_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Keys_Ceremony_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Keys_Ceremony_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Keys_Ceremony_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** aggregate max on columns */
export type Sequent_Backend_Keys_Ceremony_Max_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  trustee_ids?: Maybe<Array<Scalars['uuid']['output']>>;
};

/** aggregate min on columns */
export type Sequent_Backend_Keys_Ceremony_Min_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  trustee_ids?: Maybe<Array<Scalars['uuid']['output']>>;
};

/** response of any mutation on the table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Mutation_Response = {
  __typename?: 'sequent_backend_keys_ceremony_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Keys_Ceremony>;
};

/** on_conflict condition type for table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_On_Conflict = {
  constraint: Sequent_Backend_Keys_Ceremony_Constraint;
  update_columns?: Array<Sequent_Backend_Keys_Ceremony_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.keys_ceremony". */
export type Sequent_Backend_Keys_Ceremony_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  execution_status?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  trustee_ids?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.keys_ceremony */
export type Sequent_Backend_Keys_Ceremony_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Keys_Ceremony_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.keys_ceremony" */
export enum Sequent_Backend_Keys_Ceremony_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TrusteeIds = 'trustee_ids'
}

/** input type for updating data in table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** Streaming cursor of the table "sequent_backend_keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Keys_Ceremony_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Keys_Ceremony_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** update columns of table "sequent_backend.keys_ceremony" */
export enum Sequent_Backend_Keys_Ceremony_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TrusteeIds = 'trustee_ids'
}

export type Sequent_Backend_Keys_Ceremony_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Keys_Ceremony_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Keys_Ceremony_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Keys_Ceremony_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Keys_Ceremony_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Keys_Ceremony_Bool_Exp;
};

/** columns and relationships of "sequent_backend.lock" */
export type Sequent_Backend_Lock = {
  __typename?: 'sequent_backend_lock';
  created_at: Scalars['timestamptz']['output'];
  expiry_date?: Maybe<Scalars['timestamptz']['output']>;
  key: Scalars['String']['output'];
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  value: Scalars['String']['output'];
};

/** aggregated selection of "sequent_backend.lock" */
export type Sequent_Backend_Lock_Aggregate = {
  __typename?: 'sequent_backend_lock_aggregate';
  aggregate?: Maybe<Sequent_Backend_Lock_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Lock>;
};

/** aggregate fields of "sequent_backend.lock" */
export type Sequent_Backend_Lock_Aggregate_Fields = {
  __typename?: 'sequent_backend_lock_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Lock_Max_Fields>;
  min?: Maybe<Sequent_Backend_Lock_Min_Fields>;
};


/** aggregate fields of "sequent_backend.lock" */
export type Sequent_Backend_Lock_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Lock_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.lock". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Lock_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Lock_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Lock_Bool_Exp>>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  expiry_date?: InputMaybe<Timestamptz_Comparison_Exp>;
  key?: InputMaybe<String_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  value?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.lock" */
export enum Sequent_Backend_Lock_Constraint {
  /** unique or primary key constraint on columns "key" */
  LockPkey = 'lock_pkey'
}

/** input type for inserting data into table "sequent_backend.lock" */
export type Sequent_Backend_Lock_Insert_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  expiry_date?: InputMaybe<Scalars['timestamptz']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  value?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Lock_Max_Fields = {
  __typename?: 'sequent_backend_lock_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  expiry_date?: Maybe<Scalars['timestamptz']['output']>;
  key?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  value?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Lock_Min_Fields = {
  __typename?: 'sequent_backend_lock_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  expiry_date?: Maybe<Scalars['timestamptz']['output']>;
  key?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  value?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.lock" */
export type Sequent_Backend_Lock_Mutation_Response = {
  __typename?: 'sequent_backend_lock_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Lock>;
};

/** on_conflict condition type for table "sequent_backend.lock" */
export type Sequent_Backend_Lock_On_Conflict = {
  constraint: Sequent_Backend_Lock_Constraint;
  update_columns?: Array<Sequent_Backend_Lock_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.lock". */
export type Sequent_Backend_Lock_Order_By = {
  created_at?: InputMaybe<Order_By>;
  expiry_date?: InputMaybe<Order_By>;
  key?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  value?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.lock */
export type Sequent_Backend_Lock_Pk_Columns_Input = {
  key: Scalars['String']['input'];
};

/** select columns of table "sequent_backend.lock" */
export enum Sequent_Backend_Lock_Select_Column {
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ExpiryDate = 'expiry_date',
  /** column name */
  Key = 'key',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Value = 'value'
}

/** input type for updating data in table "sequent_backend.lock" */
export type Sequent_Backend_Lock_Set_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  expiry_date?: InputMaybe<Scalars['timestamptz']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  value?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_lock" */
export type Sequent_Backend_Lock_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Lock_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Lock_Stream_Cursor_Value_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  expiry_date?: InputMaybe<Scalars['timestamptz']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  value?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.lock" */
export enum Sequent_Backend_Lock_Update_Column {
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ExpiryDate = 'expiry_date',
  /** column name */
  Key = 'key',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Value = 'value'
}

export type Sequent_Backend_Lock_Updates = {
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Lock_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Lock_Bool_Exp;
};

/** columns and relationships of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event = {
  __typename?: 'sequent_backend_scheduled_event';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  cron_config?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  event_payload?: Maybe<Scalars['jsonb']['output']>;
  event_processor?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  stopped_at?: Maybe<Scalars['timestamptz']['output']>;
  task_id?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_EventAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_EventEvent_PayloadArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_EventLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Aggregate = {
  __typename?: 'sequent_backend_scheduled_event_aggregate';
  aggregate?: Maybe<Sequent_Backend_Scheduled_Event_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Scheduled_Event>;
};

/** aggregate fields of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Aggregate_Fields = {
  __typename?: 'sequent_backend_scheduled_event_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Scheduled_Event_Max_Fields>;
  min?: Maybe<Sequent_Backend_Scheduled_Event_Min_Fields>;
};


/** aggregate fields of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Scheduled_Event_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.scheduled_event". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Scheduled_Event_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_by?: InputMaybe<String_Comparison_Exp>;
  cron_config?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  event_payload?: InputMaybe<Jsonb_Comparison_Exp>;
  event_processor?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  stopped_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  task_id?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.scheduled_event" */
export enum Sequent_Backend_Scheduled_Event_Constraint {
  /** unique or primary key constraint on columns "id" */
  ScheduledEventPkey = 'scheduled_event_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Scheduled_Event_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  event_payload?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Scheduled_Event_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  event_payload?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Scheduled_Event_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  event_payload?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  event_processor?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  stopped_at?: InputMaybe<Scalars['timestamptz']['input']>;
  task_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Scheduled_Event_Max_Fields = {
  __typename?: 'sequent_backend_scheduled_event_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  cron_config?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  event_processor?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  stopped_at?: Maybe<Scalars['timestamptz']['output']>;
  task_id?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Scheduled_Event_Min_Fields = {
  __typename?: 'sequent_backend_scheduled_event_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  cron_config?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  event_processor?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  stopped_at?: Maybe<Scalars['timestamptz']['output']>;
  task_id?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Mutation_Response = {
  __typename?: 'sequent_backend_scheduled_event_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Scheduled_Event>;
};

/** on_conflict condition type for table "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_On_Conflict = {
  constraint: Sequent_Backend_Scheduled_Event_Constraint;
  update_columns?: Array<Sequent_Backend_Scheduled_Event_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.scheduled_event". */
export type Sequent_Backend_Scheduled_Event_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  created_by?: InputMaybe<Order_By>;
  cron_config?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  event_payload?: InputMaybe<Order_By>;
  event_processor?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  stopped_at?: InputMaybe<Order_By>;
  task_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.scheduled_event */
export type Sequent_Backend_Scheduled_Event_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Scheduled_Event_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.scheduled_event" */
export enum Sequent_Backend_Scheduled_Event_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedBy = 'created_by',
  /** column name */
  CronConfig = 'cron_config',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EventPayload = 'event_payload',
  /** column name */
  EventProcessor = 'event_processor',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  StoppedAt = 'stopped_at',
  /** column name */
  TaskId = 'task_id',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  event_processor?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  stopped_at?: InputMaybe<Scalars['timestamptz']['input']>;
  task_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Scheduled_Event_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Scheduled_Event_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  event_processor?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  stopped_at?: InputMaybe<Scalars['timestamptz']['input']>;
  task_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.scheduled_event" */
export enum Sequent_Backend_Scheduled_Event_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedBy = 'created_by',
  /** column name */
  CronConfig = 'cron_config',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EventPayload = 'event_payload',
  /** column name */
  EventProcessor = 'event_processor',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  StoppedAt = 'stopped_at',
  /** column name */
  TaskId = 'task_id',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Scheduled_Event_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Scheduled_Event_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Scheduled_Event_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Scheduled_Event_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Scheduled_Event_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Scheduled_Event_Bool_Exp;
};

/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session = {
  __typename?: 'sequent_backend_tally_session';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id: Scalars['uuid']['output'];
  is_execution_completed: Scalars['Boolean']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id: Scalars['uuid']['output'];
  trustee_ids?: Maybe<Array<Scalars['uuid']['output']>>;
};


/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_SessionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_SessionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Aggregate = {
  __typename?: 'sequent_backend_tally_session_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tally_Session_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tally_Session>;
};

/** aggregate fields of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Aggregate_Fields = {
  __typename?: 'sequent_backend_tally_session_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tally_Session_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tally_Session_Min_Fields>;
};


/** aggregate fields of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tally_session". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tally_Session_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tally_Session_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tally_Session_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_execution_completed?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  trustee_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tally_session" */
export enum Sequent_Backend_Tally_Session_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  TallyPkey = 'tally_pkey'
}

/** columns and relationships of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest = {
  __typename?: 'sequent_backend_tally_session_contest';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id: Scalars['uuid']['output'];
  contest_id: Scalars['uuid']['output'];
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  session_id: Scalars['Int']['output'];
  tally_session_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_ContestAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_ContestLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Aggregate = {
  __typename?: 'sequent_backend_tally_session_contest_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tally_Session_Contest_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tally_Session_Contest>;
};

/** aggregate fields of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Aggregate_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Tally_Session_Contest_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tally_Session_Contest_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tally_Session_Contest_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Tally_Session_Contest_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Tally_Session_Contest_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Tally_Session_Contest_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Tally_Session_Contest_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Tally_Session_Contest_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Tally_Session_Contest_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Tally_Session_Contest_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Contest_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Tally_Session_Contest_Avg_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_avg_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tally_session_contest". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tally_Session_Contest_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  session_id?: InputMaybe<Int_Comparison_Exp>;
  tally_session_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tally_session_contest" */
export enum Sequent_Backend_Tally_Session_Contest_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  TallyContestPkey = 'tally_contest_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tally_Session_Contest_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Session_Contest_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Session_Contest_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Inc_Input = {
  session_id?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  session_id?: InputMaybe<Scalars['Int']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Session_Contest_Max_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  session_id?: Maybe<Scalars['Int']['output']>;
  tally_session_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Session_Contest_Min_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  session_id?: Maybe<Scalars['Int']['output']>;
  tally_session_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Mutation_Response = {
  __typename?: 'sequent_backend_tally_session_contest_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tally_Session_Contest>;
};

/** on_conflict condition type for table "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_On_Conflict = {
  constraint: Sequent_Backend_Tally_Session_Contest_Constraint;
  update_columns?: Array<Sequent_Backend_Tally_Session_Contest_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tally_session_contest". */
export type Sequent_Backend_Tally_Session_Contest_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  session_id?: InputMaybe<Order_By>;
  tally_session_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tally_session_contest */
export type Sequent_Backend_Tally_Session_Contest_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Contest_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tally_session_contest" */
export enum Sequent_Backend_Tally_Session_Contest_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  SessionId = 'session_id',
  /** column name */
  TallySessionId = 'tally_session_id',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  session_id?: InputMaybe<Scalars['Int']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Tally_Session_Contest_Stddev_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_stddev_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Tally_Session_Contest_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_stddev_pop_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Tally_Session_Contest_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_stddev_samp_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_tally_session_contest" */
export type Sequent_Backend_Tally_Session_Contest_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tally_Session_Contest_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tally_Session_Contest_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  session_id?: InputMaybe<Scalars['Int']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Tally_Session_Contest_Sum_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_sum_fields';
  session_id?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.tally_session_contest" */
export enum Sequent_Backend_Tally_Session_Contest_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  SessionId = 'session_id',
  /** column name */
  TallySessionId = 'tally_session_id',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Tally_Session_Contest_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tally_Session_Contest_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Tally_Session_Contest_Var_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_var_pop_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Tally_Session_Contest_Var_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_var_samp_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Tally_Session_Contest_Variance_Fields = {
  __typename?: 'sequent_backend_tally_session_contest_variance_fields';
  session_id?: Maybe<Scalars['Float']['output']>;
};

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tally_Session_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Session_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Session_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution = {
  __typename?: 'sequent_backend_tally_session_execution';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id: Scalars['Int']['output'];
  document_id: Scalars['uuid']['output'];
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tally_session_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Aggregate = {
  __typename?: 'sequent_backend_tally_session_execution_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tally_Session_Execution_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tally_Session_Execution>;
};

/** aggregate fields of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Aggregate_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Tally_Session_Execution_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tally_Session_Execution_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tally_Session_Execution_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Tally_Session_Execution_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Tally_Session_Execution_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Tally_Session_Execution_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Tally_Session_Execution_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Tally_Session_Execution_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Tally_Session_Execution_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Tally_Session_Execution_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Execution_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Tally_Session_Execution_Avg_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_avg_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tally_session_execution". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tally_Session_Execution_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  current_message_id?: InputMaybe<Int_Comparison_Exp>;
  document_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tally_session_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tally_session_execution" */
export enum Sequent_Backend_Tally_Session_Execution_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  TallySessionExecutionPkey = 'tally_session_execution_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tally_Session_Execution_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Session_Execution_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Session_Execution_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Inc_Input = {
  current_message_id?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  current_message_id?: InputMaybe<Scalars['Int']['input']>;
  document_id?: InputMaybe<Scalars['uuid']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Session_Execution_Max_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id?: Maybe<Scalars['Int']['output']>;
  document_id?: Maybe<Scalars['uuid']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tally_session_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Session_Execution_Min_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id?: Maybe<Scalars['Int']['output']>;
  document_id?: Maybe<Scalars['uuid']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tally_session_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Mutation_Response = {
  __typename?: 'sequent_backend_tally_session_execution_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tally_Session_Execution>;
};

/** on_conflict condition type for table "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_On_Conflict = {
  constraint: Sequent_Backend_Tally_Session_Execution_Constraint;
  update_columns?: Array<Sequent_Backend_Tally_Session_Execution_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tally_session_execution". */
export type Sequent_Backend_Tally_Session_Execution_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  current_message_id?: InputMaybe<Order_By>;
  document_id?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  tally_session_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tally_session_execution */
export type Sequent_Backend_Tally_Session_Execution_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Execution_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tally_session_execution" */
export enum Sequent_Backend_Tally_Session_Execution_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CurrentMessageId = 'current_message_id',
  /** column name */
  DocumentId = 'document_id',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TallySessionId = 'tally_session_id',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  current_message_id?: InputMaybe<Scalars['Int']['input']>;
  document_id?: InputMaybe<Scalars['uuid']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Tally_Session_Execution_Stddev_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_stddev_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Tally_Session_Execution_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_stddev_pop_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Tally_Session_Execution_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_stddev_samp_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tally_Session_Execution_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tally_Session_Execution_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  current_message_id?: InputMaybe<Scalars['Int']['input']>;
  document_id?: InputMaybe<Scalars['uuid']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Tally_Session_Execution_Sum_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_sum_fields';
  current_message_id?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.tally_session_execution" */
export enum Sequent_Backend_Tally_Session_Execution_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CurrentMessageId = 'current_message_id',
  /** column name */
  DocumentId = 'document_id',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TallySessionId = 'tally_session_id',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Tally_Session_Execution_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tally_Session_Execution_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Tally_Session_Execution_Var_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_var_pop_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Tally_Session_Execution_Var_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_var_samp_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Tally_Session_Execution_Variance_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_variance_fields';
  current_message_id?: Maybe<Scalars['Float']['output']>;
};

/** input type for inserting data into table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Session_Max_Fields = {
  __typename?: 'sequent_backend_tally_session_max_fields';
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  trustee_ids?: Maybe<Array<Scalars['uuid']['output']>>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Session_Min_Fields = {
  __typename?: 'sequent_backend_tally_session_min_fields';
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  trustee_ids?: Maybe<Array<Scalars['uuid']['output']>>;
};

/** response of any mutation on the table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Mutation_Response = {
  __typename?: 'sequent_backend_tally_session_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tally_Session>;
};

/** on_conflict condition type for table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_On_Conflict = {
  constraint: Sequent_Backend_Tally_Session_Constraint;
  update_columns?: Array<Sequent_Backend_Tally_Session_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tally_session". */
export type Sequent_Backend_Tally_Session_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_ids?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_ids?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_execution_completed?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  trustee_ids?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tally_session */
export type Sequent_Backend_Tally_Session_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tally_session" */
export enum Sequent_Backend_Tally_Session_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaIds = 'area_ids',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  Id = 'id',
  /** column name */
  IsExecutionCompleted = 'is_execution_completed',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TrusteeIds = 'trustee_ids'
}

/** input type for updating data in table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** Streaming cursor of the table "sequent_backend_tally_session" */
export type Sequent_Backend_Tally_Session_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tally_Session_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tally_Session_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** update columns of table "sequent_backend.tally_session" */
export enum Sequent_Backend_Tally_Session_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaIds = 'area_ids',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  Id = 'id',
  /** column name */
  IsExecutionCompleted = 'is_execution_completed',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TrusteeIds = 'trustee_ids'
}

export type Sequent_Backend_Tally_Session_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tally_Session_Bool_Exp;
};

/** columns and relationships of "sequent_backend.tenant" */
export type Sequent_Backend_Tenant = {
  __typename?: 'sequent_backend_tenant';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  id: Scalars['uuid']['output'];
  is_active: Scalars['Boolean']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  slug: Scalars['String']['output'];
  updated_at: Scalars['timestamptz']['output'];
  voting_channels?: Maybe<Scalars['jsonb']['output']>;
};


/** columns and relationships of "sequent_backend.tenant" */
export type Sequent_Backend_TenantAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tenant" */
export type Sequent_Backend_TenantLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tenant" */
export type Sequent_Backend_TenantVoting_ChannelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Aggregate = {
  __typename?: 'sequent_backend_tenant_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tenant_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tenant>;
};

/** aggregate fields of "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Aggregate_Fields = {
  __typename?: 'sequent_backend_tenant_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tenant_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tenant_Min_Fields>;
};


/** aggregate fields of "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tenant_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tenant_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tenant". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tenant_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tenant_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tenant_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_active?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  slug?: InputMaybe<String_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  voting_channels?: InputMaybe<Jsonb_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tenant" */
export enum Sequent_Backend_Tenant_Constraint {
  /** unique or primary key constraint on columns "id" */
  TenantPkey = 'tenant_pkey',
  /** unique or primary key constraint on columns "slug" */
  TenantSlugKey = 'tenant_slug_key'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tenant_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tenant_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tenant_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tenant_Max_Fields = {
  __typename?: 'sequent_backend_tenant_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  slug?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tenant_Min_Fields = {
  __typename?: 'sequent_backend_tenant_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  slug?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** response of any mutation on the table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Mutation_Response = {
  __typename?: 'sequent_backend_tenant_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tenant>;
};

/** on_conflict condition type for table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_On_Conflict = {
  constraint: Sequent_Backend_Tenant_Constraint;
  update_columns?: Array<Sequent_Backend_Tenant_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tenant". */
export type Sequent_Backend_Tenant_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_active?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  slug?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
  voting_channels?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tenant */
export type Sequent_Backend_Tenant_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tenant_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tenant" */
export enum Sequent_Backend_Tenant_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  Labels = 'labels',
  /** column name */
  Slug = 'slug',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  VotingChannels = 'voting_channels'
}

/** input type for updating data in table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Streaming cursor of the table "sequent_backend_tenant" */
export type Sequent_Backend_Tenant_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tenant_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tenant_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** update columns of table "sequent_backend.tenant" */
export enum Sequent_Backend_Tenant_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  IsActive = 'is_active',
  /** column name */
  Labels = 'labels',
  /** column name */
  Slug = 'slug',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  VotingChannels = 'voting_channels'
}

export type Sequent_Backend_Tenant_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tenant_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tenant_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tenant_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tenant_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tenant_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tenant_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tenant_Bool_Exp;
};

/** columns and relationships of "sequent_backend.trustee" */
export type Sequent_Backend_Trustee = {
  __typename?: 'sequent_backend_trustee';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.trustee" */
export type Sequent_Backend_TrusteeAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.trustee" */
export type Sequent_Backend_TrusteeLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Aggregate = {
  __typename?: 'sequent_backend_trustee_aggregate';
  aggregate?: Maybe<Sequent_Backend_Trustee_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Trustee>;
};

/** aggregate fields of "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Aggregate_Fields = {
  __typename?: 'sequent_backend_trustee_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Trustee_Max_Fields>;
  min?: Maybe<Sequent_Backend_Trustee_Min_Fields>;
};


/** aggregate fields of "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Trustee_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.trustee". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Trustee_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Trustee_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Trustee_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  public_key?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.trustee" */
export enum Sequent_Backend_Trustee_Constraint {
  /** unique or primary key constraint on columns "id" */
  TrusteePkey = 'trustee_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Trustee_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Trustee_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Trustee_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Trustee_Max_Fields = {
  __typename?: 'sequent_backend_trustee_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Trustee_Min_Fields = {
  __typename?: 'sequent_backend_trustee_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  public_key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Mutation_Response = {
  __typename?: 'sequent_backend_trustee_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Trustee>;
};

/** on_conflict condition type for table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_On_Conflict = {
  constraint: Sequent_Backend_Trustee_Constraint;
  update_columns?: Array<Sequent_Backend_Trustee_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.trustee". */
export type Sequent_Backend_Trustee_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  public_key?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.trustee */
export type Sequent_Backend_Trustee_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Trustee_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.trustee" */
export enum Sequent_Backend_Trustee_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  PublicKey = 'public_key',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_trustee" */
export type Sequent_Backend_Trustee_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Trustee_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Trustee_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.trustee" */
export enum Sequent_Backend_Trustee_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  PublicKey = 'public_key',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Trustee_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Trustee_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Trustee_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Trustee_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Trustee_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Trustee_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Trustee_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Trustee_Bool_Exp;
};

export type Subscription_Root = {
  __typename?: 'subscription_root';
  /** fetch data from the table: "sequent_backend.area" */
  sequent_backend_area: Array<Sequent_Backend_Area>;
  /** fetch aggregated fields from the table: "sequent_backend.area" */
  sequent_backend_area_aggregate: Sequent_Backend_Area_Aggregate;
  /** fetch data from the table: "sequent_backend.area" using primary key columns */
  sequent_backend_area_by_pk?: Maybe<Sequent_Backend_Area>;
  /** fetch data from the table: "sequent_backend.area_contest" */
  sequent_backend_area_contest: Array<Sequent_Backend_Area_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.area_contest" */
  sequent_backend_area_contest_aggregate: Sequent_Backend_Area_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.area_contest" using primary key columns */
  sequent_backend_area_contest_by_pk?: Maybe<Sequent_Backend_Area_Contest>;
  /** fetch data from the table in a streaming manner: "sequent_backend.area_contest" */
  sequent_backend_area_contest_stream: Array<Sequent_Backend_Area_Contest>;
  /** fetch data from the table in a streaming manner: "sequent_backend.area" */
  sequent_backend_area_stream: Array<Sequent_Backend_Area>;
  /** fetch data from the table: "sequent_backend.ballot_style" */
  sequent_backend_ballot_style: Array<Sequent_Backend_Ballot_Style>;
  /** fetch aggregated fields from the table: "sequent_backend.ballot_style" */
  sequent_backend_ballot_style_aggregate: Sequent_Backend_Ballot_Style_Aggregate;
  /** fetch data from the table: "sequent_backend.ballot_style" using primary key columns */
  sequent_backend_ballot_style_by_pk?: Maybe<Sequent_Backend_Ballot_Style>;
  /** fetch data from the table in a streaming manner: "sequent_backend.ballot_style" */
  sequent_backend_ballot_style_stream: Array<Sequent_Backend_Ballot_Style>;
  /** fetch data from the table: "sequent_backend.candidate" */
  sequent_backend_candidate: Array<Sequent_Backend_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.candidate" */
  sequent_backend_candidate_aggregate: Sequent_Backend_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.candidate" using primary key columns */
  sequent_backend_candidate_by_pk?: Maybe<Sequent_Backend_Candidate>;
  /** fetch data from the table in a streaming manner: "sequent_backend.candidate" */
  sequent_backend_candidate_stream: Array<Sequent_Backend_Candidate>;
  /** fetch data from the table: "sequent_backend.cast_vote" */
  sequent_backend_cast_vote: Array<Sequent_Backend_Cast_Vote>;
  /** fetch aggregated fields from the table: "sequent_backend.cast_vote" */
  sequent_backend_cast_vote_aggregate: Sequent_Backend_Cast_Vote_Aggregate;
  /** fetch data from the table: "sequent_backend.cast_vote" using primary key columns */
  sequent_backend_cast_vote_by_pk?: Maybe<Sequent_Backend_Cast_Vote>;
  /** fetch data from the table in a streaming manner: "sequent_backend.cast_vote" */
  sequent_backend_cast_vote_stream: Array<Sequent_Backend_Cast_Vote>;
  /** fetch data from the table: "sequent_backend.contest" */
  sequent_backend_contest: Array<Sequent_Backend_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.contest" */
  sequent_backend_contest_aggregate: Sequent_Backend_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.contest" using primary key columns */
  sequent_backend_contest_by_pk?: Maybe<Sequent_Backend_Contest>;
  /** fetch data from the table in a streaming manner: "sequent_backend.contest" */
  sequent_backend_contest_stream: Array<Sequent_Backend_Contest>;
  /** fetch data from the table: "sequent_backend.document" */
  sequent_backend_document: Array<Sequent_Backend_Document>;
  /** fetch aggregated fields from the table: "sequent_backend.document" */
  sequent_backend_document_aggregate: Sequent_Backend_Document_Aggregate;
  /** fetch data from the table: "sequent_backend.document" using primary key columns */
  sequent_backend_document_by_pk?: Maybe<Sequent_Backend_Document>;
  /** fetch data from the table in a streaming manner: "sequent_backend.document" */
  sequent_backend_document_stream: Array<Sequent_Backend_Document>;
  /** fetch data from the table: "sequent_backend.election" */
  sequent_backend_election: Array<Sequent_Backend_Election>;
  /** fetch aggregated fields from the table: "sequent_backend.election" */
  sequent_backend_election_aggregate: Sequent_Backend_Election_Aggregate;
  /** fetch data from the table: "sequent_backend.election" using primary key columns */
  sequent_backend_election_by_pk?: Maybe<Sequent_Backend_Election>;
  /** fetch data from the table: "sequent_backend.election_event" */
  sequent_backend_election_event: Array<Sequent_Backend_Election_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.election_event" */
  sequent_backend_election_event_aggregate: Sequent_Backend_Election_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.election_event" using primary key columns */
  sequent_backend_election_event_by_pk?: Maybe<Sequent_Backend_Election_Event>;
  /** fetch data from the table in a streaming manner: "sequent_backend.election_event" */
  sequent_backend_election_event_stream: Array<Sequent_Backend_Election_Event>;
  /** fetch data from the table: "sequent_backend.election_result" */
  sequent_backend_election_result: Array<Sequent_Backend_Election_Result>;
  /** fetch aggregated fields from the table: "sequent_backend.election_result" */
  sequent_backend_election_result_aggregate: Sequent_Backend_Election_Result_Aggregate;
  /** fetch data from the table: "sequent_backend.election_result" using primary key columns */
  sequent_backend_election_result_by_pk?: Maybe<Sequent_Backend_Election_Result>;
  /** fetch data from the table in a streaming manner: "sequent_backend.election_result" */
  sequent_backend_election_result_stream: Array<Sequent_Backend_Election_Result>;
  /** fetch data from the table in a streaming manner: "sequent_backend.election" */
  sequent_backend_election_stream: Array<Sequent_Backend_Election>;
  /** fetch data from the table: "sequent_backend.election_type" */
  sequent_backend_election_type: Array<Sequent_Backend_Election_Type>;
  /** fetch aggregated fields from the table: "sequent_backend.election_type" */
  sequent_backend_election_type_aggregate: Sequent_Backend_Election_Type_Aggregate;
  /** fetch data from the table: "sequent_backend.election_type" using primary key columns */
  sequent_backend_election_type_by_pk?: Maybe<Sequent_Backend_Election_Type>;
  /** fetch data from the table in a streaming manner: "sequent_backend.election_type" */
  sequent_backend_election_type_stream: Array<Sequent_Backend_Election_Type>;
  /** fetch data from the table: "sequent_backend.event_execution" */
  sequent_backend_event_execution: Array<Sequent_Backend_Event_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.event_execution" */
  sequent_backend_event_execution_aggregate: Sequent_Backend_Event_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.event_execution" using primary key columns */
  sequent_backend_event_execution_by_pk?: Maybe<Sequent_Backend_Event_Execution>;
  /** fetch data from the table in a streaming manner: "sequent_backend.event_execution" */
  sequent_backend_event_execution_stream: Array<Sequent_Backend_Event_Execution>;
  /** fetch data from the table: "sequent_backend.keys_ceremony" */
  sequent_backend_keys_ceremony: Array<Sequent_Backend_Keys_Ceremony>;
  /** fetch aggregated fields from the table: "sequent_backend.keys_ceremony" */
  sequent_backend_keys_ceremony_aggregate: Sequent_Backend_Keys_Ceremony_Aggregate;
  /** fetch data from the table: "sequent_backend.keys_ceremony" using primary key columns */
  sequent_backend_keys_ceremony_by_pk?: Maybe<Sequent_Backend_Keys_Ceremony>;
  /** fetch data from the table in a streaming manner: "sequent_backend.keys_ceremony" */
  sequent_backend_keys_ceremony_stream: Array<Sequent_Backend_Keys_Ceremony>;
  /** fetch data from the table: "sequent_backend.lock" */
  sequent_backend_lock: Array<Sequent_Backend_Lock>;
  /** fetch aggregated fields from the table: "sequent_backend.lock" */
  sequent_backend_lock_aggregate: Sequent_Backend_Lock_Aggregate;
  /** fetch data from the table: "sequent_backend.lock" using primary key columns */
  sequent_backend_lock_by_pk?: Maybe<Sequent_Backend_Lock>;
  /** fetch data from the table in a streaming manner: "sequent_backend.lock" */
  sequent_backend_lock_stream: Array<Sequent_Backend_Lock>;
  /** fetch data from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_aggregate: Sequent_Backend_Scheduled_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.scheduled_event" using primary key columns */
  sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table in a streaming manner: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_stream: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table: "sequent_backend.tally_session" */
  sequent_backend_tally_session: Array<Sequent_Backend_Tally_Session>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session" */
  sequent_backend_tally_session_aggregate: Sequent_Backend_Tally_Session_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session" using primary key columns */
  sequent_backend_tally_session_by_pk?: Maybe<Sequent_Backend_Tally_Session>;
  /** fetch data from the table: "sequent_backend.tally_session_contest" */
  sequent_backend_tally_session_contest: Array<Sequent_Backend_Tally_Session_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session_contest" */
  sequent_backend_tally_session_contest_aggregate: Sequent_Backend_Tally_Session_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session_contest" using primary key columns */
  sequent_backend_tally_session_contest_by_pk?: Maybe<Sequent_Backend_Tally_Session_Contest>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tally_session_contest" */
  sequent_backend_tally_session_contest_stream: Array<Sequent_Backend_Tally_Session_Contest>;
  /** fetch data from the table: "sequent_backend.tally_session_execution" */
  sequent_backend_tally_session_execution: Array<Sequent_Backend_Tally_Session_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_session_execution" */
  sequent_backend_tally_session_execution_aggregate: Sequent_Backend_Tally_Session_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_session_execution" using primary key columns */
  sequent_backend_tally_session_execution_by_pk?: Maybe<Sequent_Backend_Tally_Session_Execution>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tally_session_execution" */
  sequent_backend_tally_session_execution_stream: Array<Sequent_Backend_Tally_Session_Execution>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tally_session" */
  sequent_backend_tally_session_stream: Array<Sequent_Backend_Tally_Session>;
  /** fetch data from the table: "sequent_backend.tenant" */
  sequent_backend_tenant: Array<Sequent_Backend_Tenant>;
  /** fetch aggregated fields from the table: "sequent_backend.tenant" */
  sequent_backend_tenant_aggregate: Sequent_Backend_Tenant_Aggregate;
  /** fetch data from the table: "sequent_backend.tenant" using primary key columns */
  sequent_backend_tenant_by_pk?: Maybe<Sequent_Backend_Tenant>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tenant" */
  sequent_backend_tenant_stream: Array<Sequent_Backend_Tenant>;
  /** fetch data from the table: "sequent_backend.trustee" */
  sequent_backend_trustee: Array<Sequent_Backend_Trustee>;
  /** fetch aggregated fields from the table: "sequent_backend.trustee" */
  sequent_backend_trustee_aggregate: Sequent_Backend_Trustee_Aggregate;
  /** fetch data from the table: "sequent_backend.trustee" using primary key columns */
  sequent_backend_trustee_by_pk?: Maybe<Sequent_Backend_Trustee>;
  /** fetch data from the table in a streaming manner: "sequent_backend.trustee" */
  sequent_backend_trustee_stream: Array<Sequent_Backend_Trustee>;
};


export type Subscription_RootSequent_Backend_AreaArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Area_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Area_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Area_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Area_Contest_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Area_Contest_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Area_Contest_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Area_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Area_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Ballot_StyleArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Ballot_Style_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Style_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Ballot_Style_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Ballot_Style_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Ballot_Style_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Style_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Candidate_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Candidate_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Cast_VoteArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Cast_Vote_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Cast_Vote_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Cast_Vote_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Cast_Vote_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Cast_Vote_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Cast_Vote_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Contest_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Contest_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_DocumentArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Document_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Document_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Document_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Document_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Document_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Document_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Document_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Document_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Document_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_ElectionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Election_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Election_Event_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Election_Event_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Election_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_ResultArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Result_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Result_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Result_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Result_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Result_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Result_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Election_Result_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Election_Result_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Election_Result_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Election_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_TypeArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Type_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Type_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Type_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Election_Type_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Election_Type_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Election_Type_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Election_Type_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Election_Type_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Election_Type_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Event_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Event_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Event_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Event_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Event_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Event_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Event_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Event_Execution_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Event_Execution_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Event_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Keys_CeremonyArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Keys_Ceremony_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Keys_Ceremony_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Keys_Ceremony_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Keys_Ceremony_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Keys_Ceremony_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Keys_Ceremony_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_LockArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Lock_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Lock_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Lock_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Lock_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Lock_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Lock_By_PkArgs = {
  key: Scalars['String']['input'];
};


export type Subscription_RootSequent_Backend_Lock_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Lock_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Lock_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Scheduled_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Scheduled_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Scheduled_Event_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Scheduled_Event_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Scheduled_Event_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_SessionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tally_Session_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tally_Session_Contest_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tally_Session_Contest_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Session_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_Execution_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tally_Session_Execution_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tally_Session_Execution_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Session_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tally_Session_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_TenantArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tenant_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tenant_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tenant_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tenant_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tenant_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tenant_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tenant_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tenant_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_TrusteeArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Trustee_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Trustee_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Trustee_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Trustee_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};

/** Boolean expression to compare columns of type "timestamptz". All fields are combined with logical 'AND'. */
export type Timestamptz_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['timestamptz']['input']>;
  _gt?: InputMaybe<Scalars['timestamptz']['input']>;
  _gte?: InputMaybe<Scalars['timestamptz']['input']>;
  _in?: InputMaybe<Array<Scalars['timestamptz']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['timestamptz']['input']>;
  _lte?: InputMaybe<Scalars['timestamptz']['input']>;
  _neq?: InputMaybe<Scalars['timestamptz']['input']>;
  _nin?: InputMaybe<Array<Scalars['timestamptz']['input']>>;
};

/** Boolean expression to compare columns of type "uuid". All fields are combined with logical 'AND'. */
export type Uuid_Array_Comparison_Exp = {
  /** is the array contained in the given array value */
  _contained_in?: InputMaybe<Array<Scalars['uuid']['input']>>;
  /** does the array contain the given value */
  _contains?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _eq?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _gt?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _gte?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _in?: InputMaybe<Array<Array<Scalars['uuid']['input']>>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _lte?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _neq?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _nin?: InputMaybe<Array<Array<Scalars['uuid']['input']>>>;
};

/** Boolean expression to compare columns of type "uuid". All fields are combined with logical 'AND'. */
export type Uuid_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['uuid']['input']>;
  _gt?: InputMaybe<Scalars['uuid']['input']>;
  _gte?: InputMaybe<Scalars['uuid']['input']>;
  _in?: InputMaybe<Array<Scalars['uuid']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['uuid']['input']>;
  _lte?: InputMaybe<Scalars['uuid']['input']>;
  _neq?: InputMaybe<Scalars['uuid']['input']>;
  _nin?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

export type CreateKeysCeremonyMutationVariables = Exact<{
  electionEventId: Scalars['String']['input'];
  threshold: Scalars['Int']['input'];
  trusteeNames?: InputMaybe<Array<Scalars['String']['input']> | Scalars['String']['input']>;
}>;


export type CreateKeysCeremonyMutation = { __typename?: 'mutation_root', create_keys_ceremony?: { __typename?: 'CreateKeysCeremonyOutput', keys_ceremony_id: string } | null };

export type CreateScheduledEventMutationVariables = Exact<{
  tenantId: Scalars['String']['input'];
  electionEventId: Scalars['String']['input'];
  eventProcessor: Scalars['String']['input'];
  cronConfig?: InputMaybe<Scalars['String']['input']>;
  eventPayload: Scalars['jsonb']['input'];
  createdBy: Scalars['String']['input'];
}>;


export type CreateScheduledEventMutation = { __typename?: 'mutation_root', createScheduledEvent?: { __typename?: 'ScheduledEventOutput3', id?: string | null } | null };

export type CreateUserMutationVariables = Exact<{
  tenantId: Scalars['String']['input'];
  electionEventId?: InputMaybe<Scalars['String']['input']>;
  user: KeycloakUser2;
}>;


export type CreateUserMutation = { __typename?: 'mutation_root', create_user: { __typename?: 'KeycloakUser', id?: string | null, attributes?: any | null, email?: string | null, email_verified?: boolean | null, enabled?: boolean | null, first_name?: string | null, last_name?: string | null, username?: string | null } };

export type Delete_Area_ContestsMutationVariables = Exact<{
  tenantId: Scalars['uuid']['input'];
  area: Scalars['uuid']['input'];
}>;


export type Delete_Area_ContestsMutation = { __typename?: 'mutation_root', delete_sequent_backend_area_contest?: { __typename?: 'sequent_backend_area_contest_mutation_response', returning: Array<{ __typename?: 'sequent_backend_area_contest', id: any }> } | null };

export type EditUserMutationVariables = Exact<{
  body: EditUsersInput;
}>;


export type EditUserMutation = { __typename?: 'mutation_root', edit_user: { __typename?: 'KeycloakUser', attributes?: any | null, email?: string | null, email_verified?: boolean | null, enabled?: boolean | null, first_name?: string | null, groups?: Array<string> | null, id?: string | null, last_name?: string | null, username?: string | null } };

export type FetchDocumentQueryVariables = Exact<{
  tenantId: Scalars['String']['input'];
  electionEventId: Scalars['String']['input'];
  documentId: Scalars['String']['input'];
}>;


export type FetchDocumentQuery = { __typename?: 'query_root', fetchDocument?: { __typename?: 'FetchDocumentOutput', url: string } | null };

export type Get_Area_With_Area_ContestsQueryVariables = Exact<{
  areaId: Scalars['uuid']['input'];
  electionEventId: Scalars['uuid']['input'];
}>;


export type Get_Area_With_Area_ContestsQuery = { __typename?: 'query_root', sequent_backend_area_contest: Array<{ __typename?: 'sequent_backend_area_contest', id: any, contest?: { __typename?: 'sequent_backend_contest', name?: string | null } | null }> };

export type Sequent_Backend_Area_ExtendedQueryVariables = Exact<{
  electionEventId: Scalars['uuid']['input'];
  areaId: Scalars['uuid']['input'];
}>;


export type Sequent_Backend_Area_ExtendedQuery = { __typename?: 'query_root', sequent_backend_area_contest: Array<{ __typename?: 'sequent_backend_area_contest', contest?: { __typename?: 'sequent_backend_contest', id: any, name?: string | null } | null }> };

export type GetBallotStylesQueryVariables = Exact<{ [key: string]: never; }>;


export type GetBallotStylesQuery = { __typename?: 'query_root', sequent_backend_ballot_style: Array<{ __typename?: 'sequent_backend_ballot_style', id: any, election_id: any, election_event_id: any, status?: string | null, tenant_id: any, ballot_eml?: string | null, ballot_signature?: any | null, created_at?: any | null, area_id?: any | null, annotations?: any | null, labels?: any | null, last_updated_at?: any | null }> };

export type GetCastVotesQueryVariables = Exact<{
  electionEventId?: InputMaybe<Scalars['uuid']['input']>;
  tenantId?: InputMaybe<Scalars['uuid']['input']>;
  startDate: Scalars['timestamptz']['input'];
  endDate: Scalars['timestamptz']['input'];
}>;


export type GetCastVotesQuery = { __typename?: 'query_root', sequent_backend_cast_vote: Array<{ __typename?: 'sequent_backend_cast_vote', id: any, tenant_id: any, election_id?: any | null, area_id?: any | null, created_at?: any | null, last_updated_at?: any | null, election_event_id: any }> };

export type GetElectionEventStatsQueryVariables = Exact<{
  electionEventId?: InputMaybe<Scalars['uuid']['input']>;
  tenantId?: InputMaybe<Scalars['uuid']['input']>;
}>;


export type GetElectionEventStatsQuery = { __typename?: 'query_root', castVotes: { __typename?: 'sequent_backend_cast_vote_aggregate', aggregate?: { __typename?: 'sequent_backend_cast_vote_aggregate_fields', count: number } | null }, elections: { __typename?: 'sequent_backend_election_aggregate', aggregate?: { __typename?: 'sequent_backend_election_aggregate_fields', count: number } | null }, areas: { __typename?: 'sequent_backend_area_aggregate', aggregate?: { __typename?: 'sequent_backend_area_aggregate_fields', count: number } | null } };

export type Election_Events_TreeQueryVariables = Exact<{
  tenantId: Scalars['uuid']['input'];
  isArchived: Scalars['Boolean']['input'];
}>;


export type Election_Events_TreeQuery = { __typename?: 'query_root', sequent_backend_election_event: Array<{ __typename?: 'sequent_backend_election_event', id: any, name: string, is_archived: boolean, elections: Array<{ __typename?: 'sequent_backend_election', id: any, name: string, election_event_id: any, image_document_id?: string | null, contests: Array<{ __typename?: 'sequent_backend_contest', id: any, name?: string | null, election_event_id: any, election_id: any, candidates: Array<{ __typename?: 'sequent_backend_candidate', id: any, name?: string | null, contest_id?: any | null, election_event_id: any }> }> }> }> };

export type GetElectionsQueryVariables = Exact<{ [key: string]: never; }>;


export type GetElectionsQuery = { __typename?: 'query_root', sequent_backend_election: Array<{ __typename?: 'sequent_backend_election', annotations?: any | null, created_at?: any | null, dates?: any | null, description?: string | null, election_event_id: any, eml?: string | null, id: any, is_consolidated_ballot_encoding?: boolean | null, labels?: any | null, last_updated_at?: any | null, name: string, num_allowed_revotes?: number | null, presentation?: any | null, spoil_ballot_option?: boolean | null, status?: any | null, tenant_id: any }> };

export type GetEventExecutionQueryVariables = Exact<{
  tenantId: Scalars['uuid']['input'];
  scheduledEventId: Scalars['uuid']['input'];
}>;


export type GetEventExecutionQuery = { __typename?: 'query_root', sequent_backend_event_execution: Array<{ __typename?: 'sequent_backend_event_execution', id: any, tenant_id?: any | null, election_event_id?: any | null, scheduled_event_id: any, labels?: any | null, annotations?: any | null, execution_state?: string | null, execution_payload?: any | null, result_payload?: any | null, started_at?: any | null, ended_at?: any | null }> };

export type GetPermissionsQueryVariables = Exact<{
  tenant_id?: Scalars['String']['input'];
}>;


export type GetPermissionsQuery = { __typename?: 'query_root', get_permissions: { __typename?: 'GetPermissionsOutput', items: Array<{ __typename?: 'KeycloakPermission', id?: string | null, attributes?: any | null, container_id?: string | null, description?: string | null, name?: string | null }>, total: { __typename?: 'TotalAggregate', aggregate: { __typename?: 'Aggregate', count: number } } } };

export type GetRolesQueryVariables = Exact<{
  tenant_id?: Scalars['String']['input'];
}>;


export type GetRolesQuery = { __typename?: 'query_root', get_roles: { __typename?: 'GetRolesOutput', items: Array<{ __typename?: 'KeycloakRole', id?: string | null, name?: string | null, permissions?: any | null, access?: any | null, attributes?: any | null, client_roles?: any | null }>, total: { __typename?: 'TotalAggregate', aggregate: { __typename?: 'Aggregate', count: number } } } };

export type GetUploadUrlMutationVariables = Exact<{
  name: Scalars['String']['input'];
  media_type: Scalars['String']['input'];
  size: Scalars['Int']['input'];
}>;


export type GetUploadUrlMutation = { __typename?: 'mutation_root', get_upload_url?: { __typename?: 'GetUploadUrlOutput', url: string, document_id: string } | null };

export type Insert_Area_ContestsMutationVariables = Exact<{
  areas: Array<Sequent_Backend_Area_Contest_Insert_Input> | Sequent_Backend_Area_Contest_Insert_Input;
}>;


export type Insert_Area_ContestsMutation = { __typename?: 'mutation_root', insert_sequent_backend_area_contest?: { __typename?: 'sequent_backend_area_contest_mutation_response', returning: Array<{ __typename?: 'sequent_backend_area_contest', id: any }> } | null };

export type InsertCastVoteMutationVariables = Exact<{
  id?: InputMaybe<Scalars['uuid']['input']>;
  electionId?: InputMaybe<Scalars['uuid']['input']>;
  electionEventId?: InputMaybe<Scalars['uuid']['input']>;
  tenantId?: InputMaybe<Scalars['uuid']['input']>;
  content: Scalars['String']['input'];
}>;


export type InsertCastVoteMutation = { __typename?: 'mutation_root', insert_sequent_backend_cast_vote?: { __typename?: 'sequent_backend_cast_vote_mutation_response', returning: Array<{ __typename?: 'sequent_backend_cast_vote', id: any, election_id?: any | null, election_event_id: any, tenant_id: any, voter_id_string?: string | null }> } | null };

export type CreateElectionEventMutationVariables = Exact<{
  electionEvent: CreateElectionEventInput;
}>;


export type CreateElectionEventMutation = { __typename?: 'mutation_root', insertElectionEvent?: { __typename?: 'CreateElectionEventOutput', id: string } | null };

export type InsertTenantMutationVariables = Exact<{
  slug: Scalars['String']['input'];
}>;


export type InsertTenantMutation = { __typename?: 'mutation_root', insertTenant?: { __typename?: 'InsertTenantOutput', id: any, slug: string } | null };

export type ListPgauditQueryVariables = Exact<{
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<PgAuditOrderBy>;
}>;


export type ListPgauditQuery = { __typename?: 'query_root', listPgaudit?: { __typename?: 'DataListPgAudit', items: Array<{ __typename?: 'PgAuditRow', id: number, audit_type: string, class: string, command: string, dbname: string, server_timestamp: number, session_id: string, statement: string, user: string } | null>, total: { __typename?: 'TotalAggregate', aggregate: { __typename?: 'Aggregate', count: number } } } | null };


export const CreateKeysCeremonyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateKeysCeremony"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"threshold"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"trusteeNames"}},"type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"create_keys_ceremony"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"object"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"threshold"},"value":{"kind":"Variable","name":{"kind":"Name","value":"threshold"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"trustee_names"},"value":{"kind":"Variable","name":{"kind":"Name","value":"trusteeNames"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"keys_ceremony_id"}}]}}]}}]} as unknown as DocumentNode<CreateKeysCeremonyMutation, CreateKeysCeremonyMutationVariables>;
export const CreateScheduledEventDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateScheduledEvent"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"eventProcessor"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"cronConfig"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"eventPayload"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"jsonb"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"createdBy"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createScheduledEvent"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"Argument","name":{"kind":"Name","value":"event_processor"},"value":{"kind":"Variable","name":{"kind":"Name","value":"eventProcessor"}}},{"kind":"Argument","name":{"kind":"Name","value":"cron_config"},"value":{"kind":"Variable","name":{"kind":"Name","value":"cronConfig"}}},{"kind":"Argument","name":{"kind":"Name","value":"event_payload"},"value":{"kind":"Variable","name":{"kind":"Name","value":"eventPayload"}}},{"kind":"Argument","name":{"kind":"Name","value":"created_by"},"value":{"kind":"Variable","name":{"kind":"Name","value":"createdBy"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}}]}}]} as unknown as DocumentNode<CreateScheduledEventMutation, CreateScheduledEventMutationVariables>;
export const CreateUserDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateUser"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"user"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"KeycloakUser2"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"create_user"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"Argument","name":{"kind":"Name","value":"user"},"value":{"kind":"Variable","name":{"kind":"Name","value":"user"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"attributes"}},{"kind":"Field","name":{"kind":"Name","value":"email"}},{"kind":"Field","name":{"kind":"Name","value":"email_verified"}},{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"first_name"}},{"kind":"Field","name":{"kind":"Name","value":"last_name"}},{"kind":"Field","name":{"kind":"Name","value":"username"}}]}}]}}]} as unknown as DocumentNode<CreateUserMutation, CreateUserMutationVariables>;
export const Delete_Area_ContestsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"delete_area_contests"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"area"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"delete_sequent_backend_area_contest"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"area_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"area"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"returning"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}}]}}]}}]} as unknown as DocumentNode<Delete_Area_ContestsMutation, Delete_Area_ContestsMutationVariables>;
export const EditUserDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EditUser"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"body"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"EditUsersInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edit_user"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"body"},"value":{"kind":"Variable","name":{"kind":"Name","value":"body"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"attributes"}},{"kind":"Field","name":{"kind":"Name","value":"email"}},{"kind":"Field","name":{"kind":"Name","value":"email_verified"}},{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"first_name"}},{"kind":"Field","name":{"kind":"Name","value":"groups"}},{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"last_name"}},{"kind":"Field","name":{"kind":"Name","value":"username"}}]}}]}}]} as unknown as DocumentNode<EditUserMutation, EditUserMutationVariables>;
export const FetchDocumentDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"FetchDocument"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"documentId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fetchDocument"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"Argument","name":{"kind":"Name","value":"document_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"documentId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}}]}}]}}]} as unknown as DocumentNode<FetchDocumentQuery, FetchDocumentQueryVariables>;
export const Get_Area_With_Area_ContestsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"get_area_with_area_contests"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"areaId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_area_contest"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"area_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"areaId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"contest"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"id"}}]}}]}}]} as unknown as DocumentNode<Get_Area_With_Area_ContestsQuery, Get_Area_With_Area_ContestsQueryVariables>;
export const Sequent_Backend_Area_ExtendedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"sequent_backend_area_extended"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"areaId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_area_contest"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"area_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"areaId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"contest"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]} as unknown as DocumentNode<Sequent_Backend_Area_ExtendedQuery, Sequent_Backend_Area_ExtendedQueryVariables>;
export const GetBallotStylesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetBallotStyles"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_ballot_style"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_eml"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_signature"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"area_id"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}}]}}]}}]} as unknown as DocumentNode<GetBallotStylesQuery, GetBallotStylesQueryVariables>;
export const GetCastVotesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetCastVotes"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"startDate"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"timestamptz"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"endDate"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"timestamptz"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_cast_vote"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"created_at"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_gte"},"value":{"kind":"Variable","name":{"kind":"Name","value":"startDate"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"_lte"},"value":{"kind":"Variable","name":{"kind":"Name","value":"endDate"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"area_id"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}}]}}]}}]} as unknown as DocumentNode<GetCastVotesQuery, GetCastVotesQueryVariables>;
export const GetElectionEventStatsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetElectionEventStats"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","alias":{"kind":"Name","value":"castVotes"},"name":{"kind":"Name","value":"sequent_backend_cast_vote_aggregate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"elections"},"name":{"kind":"Name","value":"sequent_backend_election_aggregate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"areas"},"name":{"kind":"Name","value":"sequent_backend_area_aggregate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}}]}}]} as unknown as DocumentNode<GetElectionEventStatsQuery, GetElectionEventStatsQueryVariables>;
export const Election_Events_TreeDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"election_events_tree"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"isArchived"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Boolean"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_election_event"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"is_archived"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"isArchived"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"is_archived"}},{"kind":"Field","name":{"kind":"Name","value":"elections"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"image_document_id"}},{"kind":"Field","name":{"kind":"Name","value":"contests"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"candidates"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"contest_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}}]}}]}}]}}]}}]}}]} as unknown as DocumentNode<Election_Events_TreeQuery, Election_Events_TreeQueryVariables>;
export const GetElectionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetElections"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_election"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"dates"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"eml"}},{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"is_consolidated_ballot_encoding"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"num_allowed_revotes"}},{"kind":"Field","name":{"kind":"Name","value":"presentation"}},{"kind":"Field","name":{"kind":"Name","value":"spoil_ballot_option"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}}]}}]}}]} as unknown as DocumentNode<GetElectionsQuery, GetElectionsQueryVariables>;
export const GetEventExecutionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetEventExecution"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"scheduledEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_event_execution"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"scheduled_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"scheduledEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"scheduled_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"execution_state"}},{"kind":"Field","name":{"kind":"Name","value":"execution_payload"}},{"kind":"Field","name":{"kind":"Name","value":"result_payload"}},{"kind":"Field","name":{"kind":"Name","value":"started_at"}},{"kind":"Field","name":{"kind":"Name","value":"ended_at"}}]}}]}}]} as unknown as DocumentNode<GetEventExecutionQuery, GetEventExecutionQueryVariables>;
export const GetPermissionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"getPermissions"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}},"defaultValue":{"kind":"StringValue","value":"","block":false}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"get_permissions"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"body"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"items"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"attributes"}},{"kind":"Field","name":{"kind":"Name","value":"container_id"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"total"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}}]}}]}}]} as unknown as DocumentNode<GetPermissionsQuery, GetPermissionsQueryVariables>;
export const GetRolesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"getRoles"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}},"defaultValue":{"kind":"StringValue","value":"","block":false}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"get_roles"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"body"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"items"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"permissions"}},{"kind":"Field","name":{"kind":"Name","value":"access"}},{"kind":"Field","name":{"kind":"Name","value":"attributes"}},{"kind":"Field","name":{"kind":"Name","value":"client_roles"}}]}},{"kind":"Field","name":{"kind":"Name","value":"total"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}}]}}]}}]} as unknown as DocumentNode<GetRolesQuery, GetRolesQueryVariables>;
export const GetUploadUrlDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"GetUploadUrl"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"media_type"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"size"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"get_upload_url"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"name"}}},{"kind":"Argument","name":{"kind":"Name","value":"media_type"},"value":{"kind":"Variable","name":{"kind":"Name","value":"media_type"}}},{"kind":"Argument","name":{"kind":"Name","value":"size"},"value":{"kind":"Variable","name":{"kind":"Name","value":"size"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"document_id"}}]}}]}}]} as unknown as DocumentNode<GetUploadUrlMutation, GetUploadUrlMutationVariables>;
export const Insert_Area_ContestsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"insert_area_contests"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"areas"}},"type":{"kind":"NonNullType","type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"sequent_backend_area_contest_insert_input"}}}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"insert_sequent_backend_area_contest"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"objects"},"value":{"kind":"Variable","name":{"kind":"Name","value":"areas"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"returning"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}}]}}]}}]} as unknown as DocumentNode<Insert_Area_ContestsMutation, Insert_Area_ContestsMutationVariables>;
export const InsertCastVoteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"InsertCastVote"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"content"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"insert_sequent_backend_cast_vote"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"objects"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"content"},"value":{"kind":"Variable","name":{"kind":"Name","value":"content"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"returning"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"voter_id_string"}}]}}]}}]}}]} as unknown as DocumentNode<InsertCastVoteMutation, InsertCastVoteMutationVariables>;
export const CreateElectionEventDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateElectionEvent"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEvent"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateElectionEventInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"insertElectionEvent"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"object"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEvent"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}}]}}]} as unknown as DocumentNode<CreateElectionEventMutation, CreateElectionEventMutationVariables>;
export const InsertTenantDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"InsertTenant"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"slug"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"insertTenant"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"slug"},"value":{"kind":"Variable","name":{"kind":"Name","value":"slug"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"slug"}}]}}]}}]} as unknown as DocumentNode<InsertTenantMutation, InsertTenantMutationVariables>;
export const ListPgauditDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"listPgaudit"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"limit"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"offset"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"order_by"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"PgAuditOrderBy"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"listPgaudit"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"limit"},"value":{"kind":"Variable","name":{"kind":"Name","value":"limit"}}},{"kind":"Argument","name":{"kind":"Name","value":"offset"},"value":{"kind":"Variable","name":{"kind":"Name","value":"offset"}}},{"kind":"Argument","name":{"kind":"Name","value":"order_by"},"value":{"kind":"Variable","name":{"kind":"Name","value":"order_by"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"items"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"audit_type"}},{"kind":"Field","name":{"kind":"Name","value":"class"}},{"kind":"Field","name":{"kind":"Name","value":"command"}},{"kind":"Field","name":{"kind":"Name","value":"dbname"}},{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"server_timestamp"}},{"kind":"Field","name":{"kind":"Name","value":"session_id"}},{"kind":"Field","name":{"kind":"Name","value":"statement"}},{"kind":"Field","name":{"kind":"Name","value":"user"}}]}},{"kind":"Field","name":{"kind":"Name","value":"total"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"aggregate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"count"}}]}}]}}]}}]}}]} as unknown as DocumentNode<ListPgauditQuery, ListPgauditQueryVariables>;