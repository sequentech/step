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
  bigint: { input: any; output: any; }
  bytea: { input: any; output: any; }
  date: { input: any; output: any; }
  json: { input: any; output: any; }
  jsonb: { input: any; output: any; }
  numeric: { input: any; output: any; }
  timestamptz: { input: any; output: any; }
  uuid: { input: any; output: any; }
};

export type Aggregate = {
  __typename?: 'Aggregate';
  count: Scalars['Int']['output'];
};

export type ApplicationChangeStatusBody = {
  area_id?: InputMaybe<Scalars['String']['input']>;
  election_event_id: Scalars['String']['input'];
  id: Scalars['String']['input'];
  rejection_message?: InputMaybe<Scalars['String']['input']>;
  rejection_reason?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['String']['input']>;
  user_id: Scalars['String']['input'];
};

export type ApplicationChangeStatusOutput = {
  __typename?: 'ApplicationChangeStatusOutput';
  error?: Maybe<Scalars['String']['output']>;
  message?: Maybe<Scalars['String']['output']>;
};

export type ApplicationVerifyBody = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_id?: InputMaybe<Scalars['String']['input']>;
  area_id?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['String']['input']>;
};

export type BallotPublicationStyles = {
  __typename?: 'BallotPublicationStyles';
  ballot_publication_id: Scalars['String']['output'];
  ballot_styles: Scalars['jsonb']['output'];
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

export type CastVoteEntry = {
  __typename?: 'CastVoteEntry';
  ballot_id: Scalars['String']['output'];
  statement_kind: Scalars['String']['output'];
  statement_timestamp: Scalars['Int']['output'];
  username: Scalars['String']['output'];
};

export type CastVotesByIp = {
  __typename?: 'CastVotesByIp';
  country?: Maybe<Scalars['String']['output']>;
  election_id?: Maybe<Scalars['String']['output']>;
  election_name?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  ip?: Maybe<Scalars['String']['output']>;
  vote_count?: Maybe<Scalars['Int']['output']>;
  voters_id?: Maybe<Array<Maybe<Scalars['String']['output']>>>;
};

export type CastVotesPerDay = {
  __typename?: 'CastVotesPerDay';
  day: Scalars['date']['output'];
  day_count: Scalars['Int']['output'];
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

export type CountUsersInput = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  authorized_to_election_alias?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  email?: InputMaybe<Scalars['jsonb']['input']>;
  email_verified?: InputMaybe<Scalars['Boolean']['input']>;
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  first_name?: InputMaybe<Scalars['jsonb']['input']>;
  has_voted?: InputMaybe<Scalars['Boolean']['input']>;
  last_name?: InputMaybe<Scalars['jsonb']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
  show_votes_info?: InputMaybe<Scalars['Boolean']['input']>;
  sort?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id: Scalars['uuid']['input'];
  username?: InputMaybe<Scalars['jsonb']['input']>;
};

export type CountUsersOutput = {
  __typename?: 'CountUsersOutput';
  count: Scalars['Int']['output'];
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
  error?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  message?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type CreateElectionOutput = {
  __typename?: 'CreateElectionOutput';
  id: Scalars['String']['output'];
};

export type CreateKeysCeremonyInput = {
  election_event_id: Scalars['String']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  is_automatic_ceremony?: InputMaybe<Scalars['Boolean']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  threshold: Scalars['Int']['input'];
  trustee_names?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type CreateKeysCeremonyOutput = {
  __typename?: 'CreateKeysCeremonyOutput';
  error_message?: Maybe<Scalars['String']['output']>;
  keys_ceremony_id: Scalars['String']['output'];
};

export type CreatePermissionInput = {
  permission: KeycloakPermission2;
  tenant_id: Scalars['String']['input'];
};

export type CreateTallyOutput = {
  __typename?: 'CreateTallyOutput';
  tally_session_id: Scalars['uuid']['output'];
};

export type DataListElectoralLog = {
  __typename?: 'DataListElectoralLog';
  items: Array<Maybe<ElectoralLogRow>>;
  total: TotalAggregate;
};

export type DataListPgAudit = {
  __typename?: 'DataListPgAudit';
  items: Array<Maybe<PgAuditRow>>;
  total: TotalAggregate;
};

export type DeleteElectionEvent = {
  __typename?: 'DeleteElectionEvent';
  error_msg?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type DeleteUserOutput = {
  __typename?: 'DeleteUserOutput';
  id?: Maybe<Scalars['String']['output']>;
};

export type DeleteUsersOutput = {
  __typename?: 'DeleteUsersOutput';
  ids?: Maybe<Scalars['String']['output']>;
};

export type EditUsersInput = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  email?: InputMaybe<Scalars['String']['input']>;
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  first_name?: InputMaybe<Scalars['String']['input']>;
  groups?: InputMaybe<Array<Scalars['String']['input']>>;
  last_name?: InputMaybe<Scalars['String']['input']>;
  password?: InputMaybe<Scalars['String']['input']>;
  temporary?: InputMaybe<Scalars['Boolean']['input']>;
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
  username?: InputMaybe<Scalars['String']['input']>;
};

export type ElectionEventStatsInput = {
  election_event_id: Scalars['uuid']['input'];
  end_date: Scalars['String']['input'];
  start_date: Scalars['String']['input'];
  user_timezone: Scalars['String']['input'];
};

export type ElectionEventStatsMonitoringOutput = {
  __typename?: 'ElectionEventStatsMonitoringOutput';
  approval_stats?: Maybe<MonitoringApproval>;
  authentication_stats?: Maybe<MonitoringAuthentication>;
  total_closed_votes?: Maybe<Scalars['Int']['output']>;
  total_elections?: Maybe<Scalars['Int']['output']>;
  total_eligible_voters?: Maybe<Scalars['Int']['output']>;
  total_enrolled_voters?: Maybe<Scalars['Int']['output']>;
  total_genereated_tally?: Maybe<Scalars['Int']['output']>;
  total_initialize?: Maybe<Scalars['Int']['output']>;
  total_not_closed_votes?: Maybe<Scalars['Int']['output']>;
  total_not_genereated_tally?: Maybe<Scalars['Int']['output']>;
  total_not_initialize?: Maybe<Scalars['Int']['output']>;
  total_not_open_votes?: Maybe<Scalars['Int']['output']>;
  total_not_start_counting_votes?: Maybe<Scalars['Int']['output']>;
  total_not_started_votes?: Maybe<Scalars['Int']['output']>;
  total_open_votes?: Maybe<Scalars['Int']['output']>;
  total_start_counting_votes?: Maybe<Scalars['Int']['output']>;
  total_started_votes?: Maybe<Scalars['Int']['output']>;
  transmission_stats?: Maybe<MonitoringTransmissionStatus>;
  voting_stats?: Maybe<MonitoringVotingSatus>;
};

export type ElectionEventStatsOutput = {
  __typename?: 'ElectionEventStatsOutput';
  total_areas: Scalars['Int']['output'];
  total_distinct_voters: Scalars['Int']['output'];
  total_elections: Scalars['Int']['output'];
  total_eligible_voters: Scalars['Int']['output'];
  votes_per_day: Array<Maybe<CastVotesPerDay>>;
};

export type ElectionStatsInput = {
  election_event_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  end_date: Scalars['String']['input'];
  start_date: Scalars['String']['input'];
  user_timezone: Scalars['String']['input'];
};

export type ElectionStatsMonitoringOutput = {
  __typename?: 'ElectionStatsMonitoringOutput';
  approval_stats?: Maybe<MonitoringApproval>;
  authentication_stats?: Maybe<MonitoringAuthentication>;
  total_eligible_voters?: Maybe<Scalars['Int']['output']>;
  total_enrolled_voters?: Maybe<Scalars['Int']['output']>;
  total_voted?: Maybe<Scalars['Int']['output']>;
};

export type ElectionStatsOutput = {
  __typename?: 'ElectionStatsOutput';
  total_areas: Scalars['Int']['output'];
  total_distinct_voters: Scalars['Int']['output'];
  votes_per_day: Array<Maybe<CastVotesPerDay>>;
};

export type ElectoralLogFilter = {
  created?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['String']['input']>;
  statement_kind?: InputMaybe<Scalars['String']['input']>;
  statement_timestamp?: InputMaybe<Scalars['String']['input']>;
  user_id?: InputMaybe<Scalars['String']['input']>;
};

export type ElectoralLogOrderBy = {
  ballot_id?: InputMaybe<OrderDirection>;
  created?: InputMaybe<OrderDirection>;
  id?: InputMaybe<OrderDirection>;
  statement_kind?: InputMaybe<OrderDirection>;
  statement_timestamp?: InputMaybe<OrderDirection>;
  user_id?: InputMaybe<OrderDirection>;
  username?: InputMaybe<OrderDirection>;
};

export type ElectoralLogRow = {
  __typename?: 'ElectoralLogRow';
  created: Scalars['Int']['output'];
  id: Scalars['Int']['output'];
  message: Scalars['String']['output'];
  statement_kind: Scalars['String']['output'];
  statement_timestamp: Scalars['Int']['output'];
  user_id: Scalars['String']['output'];
};

export type EncryptReportOutput = {
  __typename?: 'EncryptReportOutput';
  document_id?: Maybe<Scalars['String']['output']>;
  error_msg?: Maybe<Scalars['String']['output']>;
};

export type ExportApplicationOutput = {
  __typename?: 'ExportApplicationOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type ExportBallotPublicationOutput = {
  __typename?: 'ExportBallotPublicationOutput';
  document_id: Scalars['String']['output'];
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type ExportElectionEventOutput = {
  __typename?: 'ExportElectionEventOutput';
  document_id: Scalars['String']['output'];
  password?: Maybe<Scalars['String']['output']>;
  task_execution: Tasks_Execution_Type;
};

export type ExportLogsOutput = {
  __typename?: 'ExportLogsOutput';
  document_id: Scalars['String']['output'];
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type ExportOptions = {
  activity_logs?: InputMaybe<Scalars['Boolean']['input']>;
  applications?: InputMaybe<Scalars['Boolean']['input']>;
  bulletin_board?: InputMaybe<Scalars['Boolean']['input']>;
  include_voters?: InputMaybe<Scalars['Boolean']['input']>;
  is_encrypted?: InputMaybe<Scalars['Boolean']['input']>;
  password?: InputMaybe<Scalars['String']['input']>;
  publications?: InputMaybe<Scalars['Boolean']['input']>;
  reports?: InputMaybe<Scalars['Boolean']['input']>;
  s3_files?: InputMaybe<Scalars['Boolean']['input']>;
  scheduled_events?: InputMaybe<Scalars['Boolean']['input']>;
  tally?: InputMaybe<Scalars['Boolean']['input']>;
};

export type ExportTallyResultsOutput = {
  __typename?: 'ExportTallyResultsOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution: Tasks_Execution_Type;
};

export type ExportTasksExecutionOutput = {
  __typename?: 'ExportTasksExecutionOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
};

export type ExportTasksOutput = {
  __typename?: 'ExportTasksOutput';
  document_id: Scalars['String']['output'];
  task_id: Scalars['String']['output'];
};

export type ExportTemplateOutput = {
  __typename?: 'ExportTemplateOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
};

export type ExportTenantUsersOutput = {
  __typename?: 'ExportTenantUsersOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type ExportTrusteesOutput = {
  __typename?: 'ExportTrusteesOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution: Tasks_Execution_Type;
};

export type ExportUsersOutput = {
  __typename?: 'ExportUsersOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type FetchDocumentOutput = {
  __typename?: 'FetchDocumentOutput';
  url: Scalars['String']['output'];
};

export type GenerateGoogleMeetOutput = {
  __typename?: 'GenerateGoogleMeetOutput';
  meet_link?: Maybe<Scalars['String']['output']>;
};

export type GenerateTemplateOutput = {
  __typename?: 'GenerateTemplateOutput';
  document_id: Scalars['String']['output'];
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type GetBallotPublicationChangesOutput = {
  __typename?: 'GetBallotPublicationChangesOutput';
  current: BallotPublicationStyles;
  previous?: Maybe<BallotPublicationStyles>;
};

export type GetManualVerificationInput = {
  election_event_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
  voter_id: Scalars['String']['input'];
};

export type GetManualVerificationOutput = {
  __typename?: 'GetManualVerificationOutput';
  document_id?: Maybe<Scalars['String']['output']>;
  status?: Maybe<Scalars['String']['output']>;
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
  keys_ceremony_id: Scalars['String']['input'];
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

export type GetTopCastVotesByIpInput = {
  country?: InputMaybe<Scalars['String']['input']>;
  election_event_id: Scalars['uuid']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  ip?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
};

export type GetTopCastVotesByIpOutput = {
  __typename?: 'GetTopCastVotesByIpOutput';
  items: Array<CastVotesByIp>;
  total: TotalAggregate;
};

export type GetUploadUrlOutput = {
  __typename?: 'GetUploadUrlOutput';
  document_id: Scalars['String']['output'];
  url: Scalars['String']['output'];
};

export type GetUserTemplateOutput = {
  __typename?: 'GetUserTemplateOutput';
  extra_config: Scalars['String']['output'];
  template_hbs: Scalars['String']['output'];
};

export type GetUsersInput = {
  attributes?: InputMaybe<Scalars['jsonb']['input']>;
  authorized_to_election_alias?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  email?: InputMaybe<Scalars['jsonb']['input']>;
  email_verified?: InputMaybe<Scalars['Boolean']['input']>;
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  first_name?: InputMaybe<Scalars['jsonb']['input']>;
  has_voted?: InputMaybe<Scalars['Boolean']['input']>;
  last_name?: InputMaybe<Scalars['jsonb']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
  show_votes_info?: InputMaybe<Scalars['Boolean']['input']>;
  sort?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id: Scalars['uuid']['input'];
  username?: InputMaybe<Scalars['jsonb']['input']>;
};

export type GetUsersOutput = {
  __typename?: 'GetUsersOutput';
  items: Array<KeycloakUser>;
  total: TotalAggregate;
};

export type ImportOptions = {
  include_keycloak?: InputMaybe<Scalars['Boolean']['input']>;
  include_roles?: InputMaybe<Scalars['Boolean']['input']>;
  include_tenant?: InputMaybe<Scalars['Boolean']['input']>;
};

export type ImportTenantOutput = {
  __typename?: 'ImportTenantOutput';
  error?: Maybe<Scalars['String']['output']>;
  message?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type InsertCastVoteOutput = {
  __typename?: 'InsertCastVoteOutput';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id: Scalars['uuid']['output'];
  ballot_id?: Maybe<Scalars['String']['output']>;
  cast_ballot_signature: Scalars['bytea']['output'];
  content?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id: Scalars['uuid']['output'];
  voter_id_string?: Maybe<Scalars['String']['output']>;
};

export type InsertTenantOutput = {
  __typename?: 'InsertTenantOutput';
  id: Scalars['uuid']['output'];
  slug: Scalars['String']['output'];
};

/** Boolean expression to compare columns of type "Int". All fields are combined with logical 'AND'. */
export type Int_Array_Comparison_Exp = {
  /** is the array contained in the given array value */
  _contained_in?: InputMaybe<Array<Scalars['Int']['input']>>;
  /** does the array contain the given value */
  _contains?: InputMaybe<Array<Scalars['Int']['input']>>;
  _eq?: InputMaybe<Array<Scalars['Int']['input']>>;
  _gt?: InputMaybe<Array<Scalars['Int']['input']>>;
  _gte?: InputMaybe<Array<Scalars['Int']['input']>>;
  _in?: InputMaybe<Array<Array<Scalars['Int']['input']>>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Array<Scalars['Int']['input']>>;
  _lte?: InputMaybe<Array<Scalars['Int']['input']>>;
  _neq?: InputMaybe<Array<Scalars['Int']['input']>>;
  _nin?: InputMaybe<Array<Array<Scalars['Int']['input']>>>;
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
  area?: Maybe<KeycloakUserArea>;
  attributes?: Maybe<Scalars['jsonb']['output']>;
  email?: Maybe<Scalars['String']['output']>;
  email_verified?: Maybe<Scalars['Boolean']['output']>;
  enabled?: Maybe<Scalars['Boolean']['output']>;
  first_name?: Maybe<Scalars['String']['output']>;
  groups?: Maybe<Array<Scalars['String']['output']>>;
  id?: Maybe<Scalars['String']['output']>;
  last_name?: Maybe<Scalars['String']['output']>;
  username?: Maybe<Scalars['String']['output']>;
  votes_info?: Maybe<Array<VotesInfo>>;
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

export type KeycloakUserArea = {
  __typename?: 'KeycloakUserArea';
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
};

export type KeysCeremony = {
  __typename?: 'KeysCeremony';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['String']['output'];
  execution_status?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  is_default?: Maybe<Scalars['Boolean']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  permission_label?: Maybe<Array<Maybe<Scalars['String']['output']>>>;
  settings?: Maybe<Scalars['jsonb']['output']>;
  status?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['String']['output'];
  threshold: Scalars['Int']['output'];
  trustee_ids: Array<Scalars['String']['output']>;
};

export type LimitAccessByCountriesOutput = {
  __typename?: 'LimitAccessByCountriesOutput';
  success?: Maybe<Scalars['Boolean']['output']>;
};

export type ListCastVoteMessagesOutput = {
  __typename?: 'ListCastVoteMessagesOutput';
  list: Array<Maybe<CastVoteEntry>>;
  total: Scalars['Int']['output'];
};

export type ListKeysCeremonyOutput = {
  __typename?: 'ListKeysCeremonyOutput';
  items: Array<KeysCeremony>;
  total: TotalAggregate;
};

export type LogEventOutput = {
  __typename?: 'LogEventOutput';
  electionEventId?: Maybe<Scalars['String']['output']>;
};

export type ManageElectionDatesOutput = {
  __typename?: 'ManageElectionDatesOutput';
  error_msg?: Maybe<Scalars['String']['output']>;
};

export type MonitoringApproval = {
  __typename?: 'MonitoringApproval';
  total_approved?: Maybe<Scalars['Int']['output']>;
  total_automated_approved?: Maybe<Scalars['Int']['output']>;
  total_automated_disapproved?: Maybe<Scalars['Int']['output']>;
  total_disapproved?: Maybe<Scalars['Int']['output']>;
  total_manual_approved?: Maybe<Scalars['Int']['output']>;
  total_manual_disapproved?: Maybe<Scalars['Int']['output']>;
};

export type MonitoringAuthentication = {
  __typename?: 'MonitoringAuthentication';
  total_authenticated?: Maybe<Scalars['Int']['output']>;
  total_invalid_password_errors?: Maybe<Scalars['Int']['output']>;
  total_invalid_users_errors?: Maybe<Scalars['Int']['output']>;
  total_not_authenticated?: Maybe<Scalars['Int']['output']>;
};

export type MonitoringTransmissionStatus = {
  __typename?: 'MonitoringTransmissionStatus';
  total_half_transmitted_results?: Maybe<Scalars['Int']['output']>;
  total_not_transmitted_results?: Maybe<Scalars['Int']['output']>;
  total_transmitted_results?: Maybe<Scalars['Int']['output']>;
};

export type MonitoringVotingSatus = {
  __typename?: 'MonitoringVotingSatus';
  total_voted?: Maybe<Scalars['Int']['output']>;
  total_voted_tests_elections?: Maybe<Scalars['Int']['output']>;
};

export type OptionalId = {
  __typename?: 'OptionalId';
  id?: Maybe<Scalars['String']['output']>;
};

export type OptionalImportEvent = {
  __typename?: 'OptionalImportEvent';
  error?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['String']['output']>;
  message?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export enum OrderDirection {
  Asc = 'asc',
  Desc = 'desc'
}

export type PgAuditFilter = {
  audit_type?: InputMaybe<Scalars['String']['input']>;
  class?: InputMaybe<Scalars['String']['input']>;
  command?: InputMaybe<Scalars['String']['input']>;
  dbname?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['String']['input']>;
  session_id?: InputMaybe<Scalars['String']['input']>;
  statement?: InputMaybe<Scalars['String']['input']>;
  user?: InputMaybe<Scalars['String']['input']>;
};

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

export enum PgAuditTable {
  PgauditHasura = 'pgaudit_hasura',
  PgauditKeycloak = 'pgaudit_keycloak'
}

export type PrepareBallotPublicationPreviewOutput = {
  __typename?: 'PrepareBallotPublicationPreviewOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export type PublishBallotOutput = {
  __typename?: 'PublishBallotOutput';
  ballot_publication_id: Scalars['uuid']['output'];
};

export type PublishTallyOutput = {
  __typename?: 'PublishTallyOutput';
  tally_sheet_id?: Maybe<Scalars['uuid']['output']>;
};

export type RenderDocumentPdfOutput = {
  __typename?: 'RenderDocumentPDFOutput';
  document_id?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

export enum ReportEncryptionPolicy {
  ConfiguredPassword = 'configured_password',
  Unencrypted = 'unencrypted'
}

export type RestorePrivateKeyInput = {
  election_event_id: Scalars['String']['input'];
  private_key_base64: Scalars['String']['input'];
  tally_session_id: Scalars['String']['input'];
};

export type RestorePrivateKeyOutput = {
  __typename?: 'RestorePrivateKeyOutput';
  is_valid: Scalars['Boolean']['output'];
};

export type ScheduledEventOutput3 = {
  __typename?: 'ScheduledEventOutput3';
  id?: Maybe<Scalars['String']['output']>;
};

export type SetCustomUrlsOutput = {
  __typename?: 'SetCustomUrlsOutput';
  message?: Maybe<Scalars['String']['output']>;
  success: Scalars['Boolean']['output'];
};

export type SetRolePermissionOutput = {
  __typename?: 'SetRolePermissionOutput';
  id?: Maybe<Scalars['String']['output']>;
};

export type SetUserRoleOutput = {
  __typename?: 'SetUserRoleOutput';
  id?: Maybe<Scalars['String']['output']>;
};

export type SetVoterAuthenticationOutput = {
  __typename?: 'SetVoterAuthenticationOutput';
  message?: Maybe<Scalars['String']['output']>;
  success: Scalars['Boolean']['output'];
};

export type StartTallyOutput = {
  __typename?: 'StartTallyOutput';
  tally_session_id: Scalars['uuid']['output'];
};

/** Boolean expression to compare columns of type "String". All fields are combined with logical 'AND'. */
export type String_Array_Comparison_Exp = {
  /** is the array contained in the given array value */
  _contained_in?: InputMaybe<Array<Scalars['String']['input']>>;
  /** does the array contain the given value */
  _contains?: InputMaybe<Array<Scalars['String']['input']>>;
  _eq?: InputMaybe<Array<Scalars['String']['input']>>;
  _gt?: InputMaybe<Array<Scalars['String']['input']>>;
  _gte?: InputMaybe<Array<Scalars['String']['input']>>;
  _in?: InputMaybe<Array<Array<Scalars['String']['input']>>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Array<Scalars['String']['input']>>;
  _lte?: InputMaybe<Array<Scalars['String']['input']>>;
  _neq?: InputMaybe<Array<Scalars['String']['input']>>;
  _nin?: InputMaybe<Array<Array<Scalars['String']['input']>>>;
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

export type UpdateElectionVotingStatusOutput = {
  __typename?: 'UpdateElectionVotingStatusOutput';
  election_id?: Maybe<Scalars['uuid']['output']>;
};

export type UpdateEventVotingStatusOutput = {
  __typename?: 'UpdateEventVotingStatusOutput';
  election_event_id?: Maybe<Scalars['uuid']['output']>;
};

export type UpsertAreaOutput = {
  __typename?: 'UpsertAreaOutput';
  id: Scalars['String']['output'];
};

export type UserProfileAttribute = {
  __typename?: 'UserProfileAttribute';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  display_name?: Maybe<Scalars['String']['output']>;
  group?: Maybe<Scalars['String']['output']>;
  multivalued?: Maybe<Scalars['Boolean']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  permissions?: Maybe<Scalars['jsonb']['output']>;
  read_only?: Maybe<Scalars['Boolean']['output']>;
  required?: Maybe<Scalars['jsonb']['output']>;
  selector?: Maybe<Scalars['jsonb']['output']>;
  validations?: Maybe<Scalars['jsonb']['output']>;
};

export type VotesInfo = {
  __typename?: 'VotesInfo';
  election_id: Scalars['String']['output'];
  last_voted_at: Scalars['String']['output'];
  num_votes: Scalars['Int']['output'];
};

export enum VotingStatus {
  Closed = 'CLOSED',
  NotStarted = 'NOT_STARTED',
  Open = 'OPEN',
  Paused = 'PAUSED'
}

export enum VotingStatusChannel {
  Kiosk = 'KIOSK',
  Online = 'ONLINE'
}

export type ApplicationOutput = {
  __typename?: 'applicationOutput';
  document_id?: Maybe<Scalars['String']['output']>;
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

/** Boolean expression to compare columns of type "bigint". All fields are combined with logical 'AND'. */
export type Bigint_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['bigint']['input']>;
  _gt?: InputMaybe<Scalars['bigint']['input']>;
  _gte?: InputMaybe<Scalars['bigint']['input']>;
  _in?: InputMaybe<Array<Scalars['bigint']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['bigint']['input']>;
  _lte?: InputMaybe<Scalars['bigint']['input']>;
  _neq?: InputMaybe<Scalars['bigint']['input']>;
  _nin?: InputMaybe<Array<Scalars['bigint']['input']>>;
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

export type CreateBallotReceiptOutput = {
  __typename?: 'createBallotReceiptOutput';
  ballot_id?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  status?: Maybe<Scalars['String']['output']>;
};

export type CreateTransmissionPackageOutput = {
  __typename?: 'createTransmissionPackageOutput';
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

/** ordering argument of a cursor */
export enum Cursor_Ordering {
  /** ascending ordering of the cursor */
  Asc = 'ASC',
  /** descending ordering of the cursor */
  Desc = 'DESC'
}

export type DocumentTaskOutput = {
  __typename?: 'documentTaskOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
  task_execution: Tasks_Execution_Type;
};

export type GenerateReportOutput = {
  __typename?: 'generateReportOutput';
  document_id: Scalars['String']['output'];
  encryption_policy: ReportEncryptionPolicy;
  task_execution?: Maybe<Tasks_Execution_Type>;
};

/** Boolean expression to compare columns of type "json". All fields are combined with logical 'AND'. */
export type Json_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['json']['input']>;
  _gt?: InputMaybe<Scalars['json']['input']>;
  _gte?: InputMaybe<Scalars['json']['input']>;
  _in?: InputMaybe<Array<Scalars['json']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['json']['input']>;
  _lte?: InputMaybe<Scalars['json']['input']>;
  _neq?: InputMaybe<Scalars['json']['input']>;
  _nin?: InputMaybe<Array<Scalars['json']['input']>>;
};

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
  /** Confirm voter application and correlate to a Voter */
  ApplicationChangeStatus?: Maybe<ApplicationChangeStatusOutput>;
  /** Verify User Registration Application */
  VerifyApplication: Scalars['String']['output'];
  /** check private key */
  check_private_key?: Maybe<CheckPrivateKeyOutput>;
  /** create scheduled event */
  createScheduledEvent?: Maybe<ScheduledEventOutput3>;
  /** create_ballot_receipt */
  create_ballot_receipt?: Maybe<CreateBallotReceiptOutput>;
  create_election?: Maybe<CreateElectionOutput>;
  /** create keys ceremony */
  create_keys_ceremony?: Maybe<CreateKeysCeremonyOutput>;
  create_permission?: Maybe<KeycloakPermission>;
  create_role: KeycloakRole;
  create_tally_ceremony?: Maybe<CreateTallyOutput>;
  create_transmission_package?: Maybe<CreateTransmissionPackageOutput>;
  create_user: KeycloakUser;
  delete_election_event?: Maybe<DeleteElectionEvent>;
  delete_permission?: Maybe<SetRolePermissionOutput>;
  delete_role?: Maybe<SetUserRoleOutput>;
  delete_role_permission?: Maybe<SetRolePermissionOutput>;
  /** delete data from the table: "sequent_backend.applications" */
  delete_sequent_backend_applications?: Maybe<Sequent_Backend_Applications_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.applications" */
  delete_sequent_backend_applications_by_pk?: Maybe<Sequent_Backend_Applications>;
  /** delete data from the table: "sequent_backend.area" */
  delete_sequent_backend_area?: Maybe<Sequent_Backend_Area_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.area" */
  delete_sequent_backend_area_by_pk?: Maybe<Sequent_Backend_Area>;
  /** delete data from the table: "sequent_backend.area_contest" */
  delete_sequent_backend_area_contest?: Maybe<Sequent_Backend_Area_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.area_contest" */
  delete_sequent_backend_area_contest_by_pk?: Maybe<Sequent_Backend_Area_Contest>;
  /** delete data from the table: "sequent_backend.ballot_publication" */
  delete_sequent_backend_ballot_publication?: Maybe<Sequent_Backend_Ballot_Publication_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.ballot_publication" */
  delete_sequent_backend_ballot_publication_by_pk?: Maybe<Sequent_Backend_Ballot_Publication>;
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
  /** delete data from the table: "sequent_backend.notification" */
  delete_sequent_backend_notification?: Maybe<Sequent_Backend_Notification_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.notification" */
  delete_sequent_backend_notification_by_pk?: Maybe<Sequent_Backend_Notification>;
  /** delete data from the table: "sequent_backend.report" */
  delete_sequent_backend_report?: Maybe<Sequent_Backend_Report_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.report" */
  delete_sequent_backend_report_by_pk?: Maybe<Sequent_Backend_Report>;
  /** delete data from the table: "sequent_backend.results_area_contest" */
  delete_sequent_backend_results_area_contest?: Maybe<Sequent_Backend_Results_Area_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_area_contest" */
  delete_sequent_backend_results_area_contest_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest>;
  /** delete data from the table: "sequent_backend.results_area_contest_candidate" */
  delete_sequent_backend_results_area_contest_candidate?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_area_contest_candidate" */
  delete_sequent_backend_results_area_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** delete data from the table: "sequent_backend.results_contest" */
  delete_sequent_backend_results_contest?: Maybe<Sequent_Backend_Results_Contest_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_contest" */
  delete_sequent_backend_results_contest_by_pk?: Maybe<Sequent_Backend_Results_Contest>;
  /** delete data from the table: "sequent_backend.results_contest_candidate" */
  delete_sequent_backend_results_contest_candidate?: Maybe<Sequent_Backend_Results_Contest_Candidate_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_contest_candidate" */
  delete_sequent_backend_results_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Contest_Candidate>;
  /** delete data from the table: "sequent_backend.results_election" */
  delete_sequent_backend_results_election?: Maybe<Sequent_Backend_Results_Election_Mutation_Response>;
  /** delete data from the table: "sequent_backend.results_election_area" */
  delete_sequent_backend_results_election_area?: Maybe<Sequent_Backend_Results_Election_Area_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_election_area" */
  delete_sequent_backend_results_election_area_by_pk?: Maybe<Sequent_Backend_Results_Election_Area>;
  /** delete single row from the table: "sequent_backend.results_election" */
  delete_sequent_backend_results_election_by_pk?: Maybe<Sequent_Backend_Results_Election>;
  /** delete data from the table: "sequent_backend.results_event" */
  delete_sequent_backend_results_event?: Maybe<Sequent_Backend_Results_Event_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.results_event" */
  delete_sequent_backend_results_event_by_pk?: Maybe<Sequent_Backend_Results_Event>;
  /** delete data from the table: "sequent_backend.scheduled_event" */
  delete_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.scheduled_event" */
  delete_sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** delete data from the table: "sequent_backend.secret" */
  delete_sequent_backend_secret?: Maybe<Sequent_Backend_Secret_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.secret" */
  delete_sequent_backend_secret_by_pk?: Maybe<Sequent_Backend_Secret>;
  /** delete data from the table: "sequent_backend.support_material" */
  delete_sequent_backend_support_material?: Maybe<Sequent_Backend_Support_Material_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.support_material" */
  delete_sequent_backend_support_material_by_pk?: Maybe<Sequent_Backend_Support_Material>;
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
  /** delete data from the table: "sequent_backend.tally_sheet" */
  delete_sequent_backend_tally_sheet?: Maybe<Sequent_Backend_Tally_Sheet_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tally_sheet" */
  delete_sequent_backend_tally_sheet_by_pk?: Maybe<Sequent_Backend_Tally_Sheet>;
  /** delete data from the table: "sequent_backend.tasks_execution" */
  delete_sequent_backend_tasks_execution?: Maybe<Sequent_Backend_Tasks_Execution_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.tasks_execution" */
  delete_sequent_backend_tasks_execution_by_pk?: Maybe<Sequent_Backend_Tasks_Execution>;
  /** delete data from the table: "sequent_backend.template" */
  delete_sequent_backend_template?: Maybe<Sequent_Backend_Template_Mutation_Response>;
  /** delete single row from the table: "sequent_backend.template" */
  delete_sequent_backend_template_by_pk?: Maybe<Sequent_Backend_Template>;
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
  /** delete users */
  delete_users?: Maybe<DeleteUsersOutput>;
  edit_user: KeycloakUser;
  encrypt_report?: Maybe<EncryptReportOutput>;
  exportTrustees?: Maybe<ExportTrusteesOutput>;
  export_application?: Maybe<ExportApplicationOutput>;
  export_ballot_publication?: Maybe<ExportBallotPublicationOutput>;
  export_election_event?: Maybe<ExportElectionEventOutput>;
  export_election_event_logs?: Maybe<ExportLogsOutput>;
  export_election_event_tasks?: Maybe<ExportTasksOutput>;
  export_tally_results?: Maybe<ExportTallyResultsOutput>;
  export_tasks_execution?: Maybe<ExportTasksExecutionOutput>;
  export_template?: Maybe<ExportTemplateOutput>;
  export_tenant_config?: Maybe<DocumentTaskOutput>;
  export_tenant_users?: Maybe<ExportTenantUsersOutput>;
  export_users?: Maybe<ExportUsersOutput>;
  generate_ballot_publication?: Maybe<PublishBallotOutput>;
  /** generate Google Meet link for election events */
  generate_google_meet?: Maybe<GenerateGoogleMeetOutput>;
  generate_report?: Maybe<GenerateReportOutput>;
  generate_template?: Maybe<GenerateTemplateOutput>;
  generate_transmission_report?: Maybe<GenerateReportOutput>;
  get_ballot_publication_changes?: Maybe<GetBallotPublicationChangesOutput>;
  get_manual_verification_pdf?: Maybe<GetManualVerificationOutput>;
  /** get private key */
  get_private_key?: Maybe<GetPrivateKeyOutput>;
  get_upload_url?: Maybe<GetUploadUrlOutput>;
  get_user: KeycloakUser;
  get_user_template?: Maybe<GetUserTemplateOutput>;
  import_application?: Maybe<ApplicationOutput>;
  import_areas?: Maybe<OptionalId>;
  import_candidates?: Maybe<DocumentTaskOutput>;
  /** import_election_event */
  import_election_event?: Maybe<OptionalImportEvent>;
  import_templates?: Maybe<TemplateOutput>;
  import_tenant_config?: Maybe<ImportTenantOutput>;
  import_users?: Maybe<TaskOutput>;
  insertElectionEvent?: Maybe<CreateElectionEventOutput>;
  /** insertTenant */
  insertTenant?: Maybe<InsertTenantOutput>;
  /** insert_cast_vote */
  insert_cast_vote?: Maybe<InsertCastVoteOutput>;
  /** insert data into the table: "sequent_backend.applications" */
  insert_sequent_backend_applications?: Maybe<Sequent_Backend_Applications_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.applications" */
  insert_sequent_backend_applications_one?: Maybe<Sequent_Backend_Applications>;
  /** insert data into the table: "sequent_backend.area" */
  insert_sequent_backend_area?: Maybe<Sequent_Backend_Area_Mutation_Response>;
  /** insert data into the table: "sequent_backend.area_contest" */
  insert_sequent_backend_area_contest?: Maybe<Sequent_Backend_Area_Contest_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.area_contest" */
  insert_sequent_backend_area_contest_one?: Maybe<Sequent_Backend_Area_Contest>;
  /** insert a single row into the table: "sequent_backend.area" */
  insert_sequent_backend_area_one?: Maybe<Sequent_Backend_Area>;
  /** insert data into the table: "sequent_backend.ballot_publication" */
  insert_sequent_backend_ballot_publication?: Maybe<Sequent_Backend_Ballot_Publication_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.ballot_publication" */
  insert_sequent_backend_ballot_publication_one?: Maybe<Sequent_Backend_Ballot_Publication>;
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
  /** insert data into the table: "sequent_backend.notification" */
  insert_sequent_backend_notification?: Maybe<Sequent_Backend_Notification_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.notification" */
  insert_sequent_backend_notification_one?: Maybe<Sequent_Backend_Notification>;
  /** insert data into the table: "sequent_backend.report" */
  insert_sequent_backend_report?: Maybe<Sequent_Backend_Report_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.report" */
  insert_sequent_backend_report_one?: Maybe<Sequent_Backend_Report>;
  /** insert data into the table: "sequent_backend.results_area_contest" */
  insert_sequent_backend_results_area_contest?: Maybe<Sequent_Backend_Results_Area_Contest_Mutation_Response>;
  /** insert data into the table: "sequent_backend.results_area_contest_candidate" */
  insert_sequent_backend_results_area_contest_candidate?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.results_area_contest_candidate" */
  insert_sequent_backend_results_area_contest_candidate_one?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** insert a single row into the table: "sequent_backend.results_area_contest" */
  insert_sequent_backend_results_area_contest_one?: Maybe<Sequent_Backend_Results_Area_Contest>;
  /** insert data into the table: "sequent_backend.results_contest" */
  insert_sequent_backend_results_contest?: Maybe<Sequent_Backend_Results_Contest_Mutation_Response>;
  /** insert data into the table: "sequent_backend.results_contest_candidate" */
  insert_sequent_backend_results_contest_candidate?: Maybe<Sequent_Backend_Results_Contest_Candidate_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.results_contest_candidate" */
  insert_sequent_backend_results_contest_candidate_one?: Maybe<Sequent_Backend_Results_Contest_Candidate>;
  /** insert a single row into the table: "sequent_backend.results_contest" */
  insert_sequent_backend_results_contest_one?: Maybe<Sequent_Backend_Results_Contest>;
  /** insert data into the table: "sequent_backend.results_election" */
  insert_sequent_backend_results_election?: Maybe<Sequent_Backend_Results_Election_Mutation_Response>;
  /** insert data into the table: "sequent_backend.results_election_area" */
  insert_sequent_backend_results_election_area?: Maybe<Sequent_Backend_Results_Election_Area_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.results_election_area" */
  insert_sequent_backend_results_election_area_one?: Maybe<Sequent_Backend_Results_Election_Area>;
  /** insert a single row into the table: "sequent_backend.results_election" */
  insert_sequent_backend_results_election_one?: Maybe<Sequent_Backend_Results_Election>;
  /** insert data into the table: "sequent_backend.results_event" */
  insert_sequent_backend_results_event?: Maybe<Sequent_Backend_Results_Event_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.results_event" */
  insert_sequent_backend_results_event_one?: Maybe<Sequent_Backend_Results_Event>;
  /** insert data into the table: "sequent_backend.scheduled_event" */
  insert_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.scheduled_event" */
  insert_sequent_backend_scheduled_event_one?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** insert data into the table: "sequent_backend.secret" */
  insert_sequent_backend_secret?: Maybe<Sequent_Backend_Secret_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.secret" */
  insert_sequent_backend_secret_one?: Maybe<Sequent_Backend_Secret>;
  /** insert data into the table: "sequent_backend.support_material" */
  insert_sequent_backend_support_material?: Maybe<Sequent_Backend_Support_Material_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.support_material" */
  insert_sequent_backend_support_material_one?: Maybe<Sequent_Backend_Support_Material>;
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
  /** insert data into the table: "sequent_backend.tally_sheet" */
  insert_sequent_backend_tally_sheet?: Maybe<Sequent_Backend_Tally_Sheet_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tally_sheet" */
  insert_sequent_backend_tally_sheet_one?: Maybe<Sequent_Backend_Tally_Sheet>;
  /** insert data into the table: "sequent_backend.tasks_execution" */
  insert_sequent_backend_tasks_execution?: Maybe<Sequent_Backend_Tasks_Execution_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tasks_execution" */
  insert_sequent_backend_tasks_execution_one?: Maybe<Sequent_Backend_Tasks_Execution>;
  /** insert data into the table: "sequent_backend.template" */
  insert_sequent_backend_template?: Maybe<Sequent_Backend_Template_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.template" */
  insert_sequent_backend_template_one?: Maybe<Sequent_Backend_Template>;
  /** insert data into the table: "sequent_backend.tenant" */
  insert_sequent_backend_tenant?: Maybe<Sequent_Backend_Tenant_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.tenant" */
  insert_sequent_backend_tenant_one?: Maybe<Sequent_Backend_Tenant>;
  /** insert data into the table: "sequent_backend.trustee" */
  insert_sequent_backend_trustee?: Maybe<Sequent_Backend_Trustee_Mutation_Response>;
  /** insert a single row into the table: "sequent_backend.trustee" */
  insert_sequent_backend_trustee_one?: Maybe<Sequent_Backend_Trustee>;
  limit_access_by_countries?: Maybe<LimitAccessByCountriesOutput>;
  manage_election_dates?: Maybe<ManageElectionDatesOutput>;
  prepare_ballot_publication_preview?: Maybe<PrepareBallotPublicationPreviewOutput>;
  publish_ballot?: Maybe<PublishBallotOutput>;
  /** publish_tally_sheet */
  publish_tally_sheet?: Maybe<PublishTallyOutput>;
  render_document_pdf?: Maybe<RenderDocumentPdfOutput>;
  restore_private_key?: Maybe<RestorePrivateKeyOutput>;
  send_transmission_package?: Maybe<OptionalId>;
  set_custom_urls?: Maybe<SetCustomUrlsOutput>;
  set_role_permission?: Maybe<SetRolePermissionOutput>;
  set_user_role?: Maybe<SetUserRoleOutput>;
  set_voter_authentication?: Maybe<SetVoterAuthenticationOutput>;
  update_election_voting_status?: Maybe<UpdateElectionVotingStatusOutput>;
  update_event_voting_status?: Maybe<UpdateEventVotingStatusOutput>;
  /** update data of the table: "sequent_backend.applications" */
  update_sequent_backend_applications?: Maybe<Sequent_Backend_Applications_Mutation_Response>;
  /** update single row of the table: "sequent_backend.applications" */
  update_sequent_backend_applications_by_pk?: Maybe<Sequent_Backend_Applications>;
  /** update multiples rows of table: "sequent_backend.applications" */
  update_sequent_backend_applications_many?: Maybe<Array<Maybe<Sequent_Backend_Applications_Mutation_Response>>>;
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
  /** update data of the table: "sequent_backend.ballot_publication" */
  update_sequent_backend_ballot_publication?: Maybe<Sequent_Backend_Ballot_Publication_Mutation_Response>;
  /** update single row of the table: "sequent_backend.ballot_publication" */
  update_sequent_backend_ballot_publication_by_pk?: Maybe<Sequent_Backend_Ballot_Publication>;
  /** update multiples rows of table: "sequent_backend.ballot_publication" */
  update_sequent_backend_ballot_publication_many?: Maybe<Array<Maybe<Sequent_Backend_Ballot_Publication_Mutation_Response>>>;
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
  /** update data of the table: "sequent_backend.notification" */
  update_sequent_backend_notification?: Maybe<Sequent_Backend_Notification_Mutation_Response>;
  /** update single row of the table: "sequent_backend.notification" */
  update_sequent_backend_notification_by_pk?: Maybe<Sequent_Backend_Notification>;
  /** update multiples rows of table: "sequent_backend.notification" */
  update_sequent_backend_notification_many?: Maybe<Array<Maybe<Sequent_Backend_Notification_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.report" */
  update_sequent_backend_report?: Maybe<Sequent_Backend_Report_Mutation_Response>;
  /** update single row of the table: "sequent_backend.report" */
  update_sequent_backend_report_by_pk?: Maybe<Sequent_Backend_Report>;
  /** update multiples rows of table: "sequent_backend.report" */
  update_sequent_backend_report_many?: Maybe<Array<Maybe<Sequent_Backend_Report_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.results_area_contest" */
  update_sequent_backend_results_area_contest?: Maybe<Sequent_Backend_Results_Area_Contest_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_area_contest" */
  update_sequent_backend_results_area_contest_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest>;
  /** update data of the table: "sequent_backend.results_area_contest_candidate" */
  update_sequent_backend_results_area_contest_candidate?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_area_contest_candidate" */
  update_sequent_backend_results_area_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** update multiples rows of table: "sequent_backend.results_area_contest_candidate" */
  update_sequent_backend_results_area_contest_candidate_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Mutation_Response>>>;
  /** update multiples rows of table: "sequent_backend.results_area_contest" */
  update_sequent_backend_results_area_contest_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Area_Contest_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.results_contest" */
  update_sequent_backend_results_contest?: Maybe<Sequent_Backend_Results_Contest_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_contest" */
  update_sequent_backend_results_contest_by_pk?: Maybe<Sequent_Backend_Results_Contest>;
  /** update data of the table: "sequent_backend.results_contest_candidate" */
  update_sequent_backend_results_contest_candidate?: Maybe<Sequent_Backend_Results_Contest_Candidate_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_contest_candidate" */
  update_sequent_backend_results_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Contest_Candidate>;
  /** update multiples rows of table: "sequent_backend.results_contest_candidate" */
  update_sequent_backend_results_contest_candidate_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Contest_Candidate_Mutation_Response>>>;
  /** update multiples rows of table: "sequent_backend.results_contest" */
  update_sequent_backend_results_contest_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Contest_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.results_election" */
  update_sequent_backend_results_election?: Maybe<Sequent_Backend_Results_Election_Mutation_Response>;
  /** update data of the table: "sequent_backend.results_election_area" */
  update_sequent_backend_results_election_area?: Maybe<Sequent_Backend_Results_Election_Area_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_election_area" */
  update_sequent_backend_results_election_area_by_pk?: Maybe<Sequent_Backend_Results_Election_Area>;
  /** update multiples rows of table: "sequent_backend.results_election_area" */
  update_sequent_backend_results_election_area_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Election_Area_Mutation_Response>>>;
  /** update single row of the table: "sequent_backend.results_election" */
  update_sequent_backend_results_election_by_pk?: Maybe<Sequent_Backend_Results_Election>;
  /** update multiples rows of table: "sequent_backend.results_election" */
  update_sequent_backend_results_election_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Election_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.results_event" */
  update_sequent_backend_results_event?: Maybe<Sequent_Backend_Results_Event_Mutation_Response>;
  /** update single row of the table: "sequent_backend.results_event" */
  update_sequent_backend_results_event_by_pk?: Maybe<Sequent_Backend_Results_Event>;
  /** update multiples rows of table: "sequent_backend.results_event" */
  update_sequent_backend_results_event_many?: Maybe<Array<Maybe<Sequent_Backend_Results_Event_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event?: Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>;
  /** update single row of the table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** update multiples rows of table: "sequent_backend.scheduled_event" */
  update_sequent_backend_scheduled_event_many?: Maybe<Array<Maybe<Sequent_Backend_Scheduled_Event_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.secret" */
  update_sequent_backend_secret?: Maybe<Sequent_Backend_Secret_Mutation_Response>;
  /** update single row of the table: "sequent_backend.secret" */
  update_sequent_backend_secret_by_pk?: Maybe<Sequent_Backend_Secret>;
  /** update multiples rows of table: "sequent_backend.secret" */
  update_sequent_backend_secret_many?: Maybe<Array<Maybe<Sequent_Backend_Secret_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.support_material" */
  update_sequent_backend_support_material?: Maybe<Sequent_Backend_Support_Material_Mutation_Response>;
  /** update single row of the table: "sequent_backend.support_material" */
  update_sequent_backend_support_material_by_pk?: Maybe<Sequent_Backend_Support_Material>;
  /** update multiples rows of table: "sequent_backend.support_material" */
  update_sequent_backend_support_material_many?: Maybe<Array<Maybe<Sequent_Backend_Support_Material_Mutation_Response>>>;
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
  /** update data of the table: "sequent_backend.tally_sheet" */
  update_sequent_backend_tally_sheet?: Maybe<Sequent_Backend_Tally_Sheet_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tally_sheet" */
  update_sequent_backend_tally_sheet_by_pk?: Maybe<Sequent_Backend_Tally_Sheet>;
  /** update multiples rows of table: "sequent_backend.tally_sheet" */
  update_sequent_backend_tally_sheet_many?: Maybe<Array<Maybe<Sequent_Backend_Tally_Sheet_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.tasks_execution" */
  update_sequent_backend_tasks_execution?: Maybe<Sequent_Backend_Tasks_Execution_Mutation_Response>;
  /** update single row of the table: "sequent_backend.tasks_execution" */
  update_sequent_backend_tasks_execution_by_pk?: Maybe<Sequent_Backend_Tasks_Execution>;
  /** update multiples rows of table: "sequent_backend.tasks_execution" */
  update_sequent_backend_tasks_execution_many?: Maybe<Array<Maybe<Sequent_Backend_Tasks_Execution_Mutation_Response>>>;
  /** update data of the table: "sequent_backend.template" */
  update_sequent_backend_template?: Maybe<Sequent_Backend_Template_Mutation_Response>;
  /** update single row of the table: "sequent_backend.template" */
  update_sequent_backend_template_by_pk?: Maybe<Sequent_Backend_Template>;
  /** update multiples rows of table: "sequent_backend.template" */
  update_sequent_backend_template_many?: Maybe<Array<Maybe<Sequent_Backend_Template_Mutation_Response>>>;
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
  update_tally_ceremony?: Maybe<StartTallyOutput>;
  upload_signature?: Maybe<OptionalId>;
  upsert_area?: Maybe<UpsertAreaOutput>;
  /** upsert_areas */
  upsert_areas?: Maybe<OptionalId>;
};


/** mutation root */
export type Mutation_RootApplicationChangeStatusArgs = {
  body: ApplicationChangeStatusBody;
};


/** mutation root */
export type Mutation_RootVerifyApplicationArgs = {
  body: ApplicationVerifyBody;
};


/** mutation root */
export type Mutation_RootCheck_Private_KeyArgs = {
  object: CheckPrivateKeyInput;
};


/** mutation root */
export type Mutation_RootCreateScheduledEventArgs = {
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  event_payload: Scalars['jsonb']['input'];
  event_processor: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootCreate_Ballot_ReceiptArgs = {
  ballot_id: Scalars['String']['input'];
  ballot_tracker_url: Scalars['String']['input'];
  election_event_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootCreate_ElectionArgs = {
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id: Scalars['String']['input'];
  name: Scalars['String']['input'];
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
export type Mutation_RootCreate_Tally_CeremonyArgs = {
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id: Scalars['uuid']['input'];
  election_ids: Array<Scalars['uuid']['input']>;
  tally_type?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootCreate_Transmission_PackageArgs = {
  area_id: Scalars['uuid']['input'];
  election_event_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  force: Scalars['Boolean']['input'];
  tally_session_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootCreate_UserArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  user: KeycloakUser2;
  user_roles_ids?: InputMaybe<Array<Scalars['String']['input']>>;
};


/** mutation root */
export type Mutation_RootDelete_Election_EventArgs = {
  election_event_id: Scalars['String']['input'];
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
export type Mutation_RootDelete_Sequent_Backend_ApplicationsArgs = {
  where: Sequent_Backend_Applications_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Applications_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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
export type Mutation_RootDelete_Sequent_Backend_Ballot_PublicationArgs = {
  where: Sequent_Backend_Ballot_Publication_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Ballot_Publication_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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
export type Mutation_RootDelete_Sequent_Backend_NotificationArgs = {
  where: Sequent_Backend_Notification_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Notification_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_ReportArgs = {
  where: Sequent_Backend_Report_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Report_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Area_ContestArgs = {
  where: Sequent_Backend_Results_Area_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Area_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Area_Contest_CandidateArgs = {
  where: Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Area_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_ContestArgs = {
  where: Sequent_Backend_Results_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Contest_CandidateArgs = {
  where: Sequent_Backend_Results_Contest_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_ElectionArgs = {
  where: Sequent_Backend_Results_Election_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Election_AreaArgs = {
  where: Sequent_Backend_Results_Election_Area_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Election_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_EventArgs = {
  where: Sequent_Backend_Results_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Results_Event_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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
export type Mutation_RootDelete_Sequent_Backend_SecretArgs = {
  where: Sequent_Backend_Secret_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Secret_By_PkArgs = {
  id: Scalars['uuid']['input'];
  key: Scalars['String']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Support_MaterialArgs = {
  where: Sequent_Backend_Support_Material_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Support_Material_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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
export type Mutation_RootDelete_Sequent_Backend_Tally_SheetArgs = {
  where: Sequent_Backend_Tally_Sheet_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tally_Sheet_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tasks_ExecutionArgs = {
  where: Sequent_Backend_Tasks_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Tasks_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_TemplateArgs = {
  where: Sequent_Backend_Template_Bool_Exp;
};


/** mutation root */
export type Mutation_RootDelete_Sequent_Backend_Template_By_PkArgs = {
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
export type Mutation_RootDelete_UsersArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  users_id: Array<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootEdit_UserArgs = {
  body: EditUsersInput;
};


/** mutation root */
export type Mutation_RootEncrypt_ReportArgs = {
  election_event_id: Scalars['String']['input'];
  password: Scalars['String']['input'];
  report_id?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootExportTrusteesArgs = {
  password: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_ApplicationArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  election_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_Ballot_PublicationArgs = {
  ballot_publication_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_Election_EventArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  export_configurations?: InputMaybe<ExportOptions>;
};


/** mutation root */
export type Mutation_RootExport_Election_Event_LogsArgs = {
  election_event_id: Scalars['String']['input'];
  format: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_Election_Event_TasksArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootExport_Tally_ResultsArgs = {
  election_event_id: Scalars['String']['input'];
  tally_session_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_Tasks_ExecutionArgs = {
  election_event_id: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_TemplateArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  election_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_Tenant_ConfigArgs = {
  tenant_id?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootExport_Tenant_UsersArgs = {
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootExport_UsersArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  election_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGenerate_Ballot_PublicationArgs = {
  election_event_id: Scalars['uuid']['input'];
  election_id?: InputMaybe<Scalars['uuid']['input']>;
};


/** mutation root */
export type Mutation_RootGenerate_Google_MeetArgs = {
  attendee_emails: Array<Scalars['String']['input']>;
  description: Scalars['String']['input'];
  end_date_time: Scalars['String']['input'];
  start_date_time: Scalars['String']['input'];
  summary: Scalars['String']['input'];
  time_zone: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGenerate_ReportArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  report_id: Scalars['String']['input'];
  report_mode: Scalars['String']['input'];
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGenerate_TemplateArgs = {
  election_event_id: Scalars['String']['input'];
  election_id: Scalars['String']['input'];
  tally_session_id: Scalars['String']['input'];
  type: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGenerate_Transmission_ReportArgs = {
  election_event_id: Scalars['String']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  tally_session_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGet_Ballot_Publication_ChangesArgs = {
  ballot_publication_id: Scalars['uuid']['input'];
  election_event_id: Scalars['uuid']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


/** mutation root */
export type Mutation_RootGet_Manual_Verification_PdfArgs = {
  body: GetManualVerificationInput;
};


/** mutation root */
export type Mutation_RootGet_Private_KeyArgs = {
  object: GetPrivateKeyInput;
};


/** mutation root */
export type Mutation_RootGet_Upload_UrlArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  is_local?: InputMaybe<Scalars['Boolean']['input']>;
  is_public: Scalars['Boolean']['input'];
  media_type: Scalars['String']['input'];
  name: Scalars['String']['input'];
  size: Scalars['Int']['input'];
};


/** mutation root */
export type Mutation_RootGet_UserArgs = {
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id: Scalars['uuid']['input'];
  user_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootGet_User_TemplateArgs = {
  template_type: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootImport_ApplicationArgs = {
  document_id: Scalars['String']['input'];
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  election_id?: InputMaybe<Scalars['String']['input']>;
  sha256?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootImport_AreasArgs = {
  document_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  sha256?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootImport_CandidatesArgs = {
  document_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  sha256?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootImport_Election_EventArgs = {
  check_only?: InputMaybe<Scalars['Boolean']['input']>;
  document_id: Scalars['String']['input'];
  password?: InputMaybe<Scalars['String']['input']>;
  sha256?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootImport_TemplatesArgs = {
  document_id: Scalars['String']['input'];
  sha256?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootImport_Tenant_ConfigArgs = {
  document_id: Scalars['String']['input'];
  import_configurations?: InputMaybe<ImportOptions>;
  sha256?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootImport_UsersArgs = {
  document_id: Scalars['String']['input'];
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  sha256?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
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
export type Mutation_RootInsert_Cast_VoteArgs = {
  ballot_id: Scalars['String']['input'];
  content: Scalars['String']['input'];
  election_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_ApplicationsArgs = {
  objects: Array<Sequent_Backend_Applications_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Applications_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Applications_OneArgs = {
  object: Sequent_Backend_Applications_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Applications_On_Conflict>;
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
export type Mutation_RootInsert_Sequent_Backend_Ballot_PublicationArgs = {
  objects: Array<Sequent_Backend_Ballot_Publication_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Ballot_Publication_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Ballot_Publication_OneArgs = {
  object: Sequent_Backend_Ballot_Publication_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Ballot_Publication_On_Conflict>;
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
export type Mutation_RootInsert_Sequent_Backend_NotificationArgs = {
  objects: Array<Sequent_Backend_Notification_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Notification_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Notification_OneArgs = {
  object: Sequent_Backend_Notification_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Notification_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_ReportArgs = {
  objects: Array<Sequent_Backend_Report_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Report_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Report_OneArgs = {
  object: Sequent_Backend_Report_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Report_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Area_ContestArgs = {
  objects: Array<Sequent_Backend_Results_Area_Contest_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Area_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Area_Contest_CandidateArgs = {
  objects: Array<Sequent_Backend_Results_Area_Contest_Candidate_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Area_Contest_Candidate_OneArgs = {
  object: Sequent_Backend_Results_Area_Contest_Candidate_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Area_Contest_OneArgs = {
  object: Sequent_Backend_Results_Area_Contest_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Area_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_ContestArgs = {
  objects: Array<Sequent_Backend_Results_Contest_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Contest_CandidateArgs = {
  objects: Array<Sequent_Backend_Results_Contest_Candidate_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Contest_Candidate_OneArgs = {
  object: Sequent_Backend_Results_Contest_Candidate_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Contest_OneArgs = {
  object: Sequent_Backend_Results_Contest_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Contest_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_ElectionArgs = {
  objects: Array<Sequent_Backend_Results_Election_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Election_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Election_AreaArgs = {
  objects: Array<Sequent_Backend_Results_Election_Area_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Election_Area_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Election_Area_OneArgs = {
  object: Sequent_Backend_Results_Election_Area_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Election_Area_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Election_OneArgs = {
  object: Sequent_Backend_Results_Election_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Election_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_EventArgs = {
  objects: Array<Sequent_Backend_Results_Event_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Event_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Results_Event_OneArgs = {
  object: Sequent_Backend_Results_Event_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Results_Event_On_Conflict>;
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
export type Mutation_RootInsert_Sequent_Backend_SecretArgs = {
  objects: Array<Sequent_Backend_Secret_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Secret_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Secret_OneArgs = {
  object: Sequent_Backend_Secret_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Secret_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Support_MaterialArgs = {
  objects: Array<Sequent_Backend_Support_Material_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Support_Material_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Support_Material_OneArgs = {
  object: Sequent_Backend_Support_Material_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Support_Material_On_Conflict>;
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
export type Mutation_RootInsert_Sequent_Backend_Tally_SheetArgs = {
  objects: Array<Sequent_Backend_Tally_Sheet_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Sheet_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tally_Sheet_OneArgs = {
  object: Sequent_Backend_Tally_Sheet_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tally_Sheet_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tasks_ExecutionArgs = {
  objects: Array<Sequent_Backend_Tasks_Execution_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Tasks_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Tasks_Execution_OneArgs = {
  object: Sequent_Backend_Tasks_Execution_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Tasks_Execution_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_TemplateArgs = {
  objects: Array<Sequent_Backend_Template_Insert_Input>;
  on_conflict?: InputMaybe<Sequent_Backend_Template_On_Conflict>;
};


/** mutation root */
export type Mutation_RootInsert_Sequent_Backend_Template_OneArgs = {
  object: Sequent_Backend_Template_Insert_Input;
  on_conflict?: InputMaybe<Sequent_Backend_Template_On_Conflict>;
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
export type Mutation_RootLimit_Access_By_CountriesArgs = {
  enroll_countries: Array<Scalars['String']['input']>;
  voting_countries: Array<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootManage_Election_DatesArgs = {
  election_event_id: Scalars['String']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  event_processor: Scalars['String']['input'];
  scheduled_date?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootPrepare_Ballot_Publication_PreviewArgs = {
  ballot_publication_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootPublish_BallotArgs = {
  ballot_publication_id: Scalars['uuid']['input'];
  election_event_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootPublish_Tally_SheetArgs = {
  election_event_id: Scalars['uuid']['input'];
  publish?: InputMaybe<Scalars['Boolean']['input']>;
  tally_sheet_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootRender_Document_PdfArgs = {
  document_id: Scalars['uuid']['input'];
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
};


/** mutation root */
export type Mutation_RootRestore_Private_KeyArgs = {
  object: RestorePrivateKeyInput;
};


/** mutation root */
export type Mutation_RootSend_Transmission_PackageArgs = {
  area_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  tally_session_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootSet_Custom_UrlsArgs = {
  dns_prefix: Scalars['String']['input'];
  election_id: Scalars['String']['input'];
  key: Scalars['String']['input'];
  origin: Scalars['String']['input'];
  redirect_to: Scalars['String']['input'];
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
export type Mutation_RootSet_Voter_AuthenticationArgs = {
  election_event_id: Scalars['String']['input'];
  enrollment: Scalars['String']['input'];
  otp: Scalars['String']['input'];
};


/** mutation root */
export type Mutation_RootUpdate_Election_Voting_StatusArgs = {
  election_event_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  voting_channels?: InputMaybe<Array<InputMaybe<VotingStatusChannel>>>;
  voting_status: VotingStatus;
};


/** mutation root */
export type Mutation_RootUpdate_Event_Voting_StatusArgs = {
  election_event_id: Scalars['uuid']['input'];
  voting_channels?: InputMaybe<Array<InputMaybe<VotingStatusChannel>>>;
  voting_status: VotingStatus;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_ApplicationsArgs = {
  _append?: InputMaybe<Sequent_Backend_Applications_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Applications_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Applications_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Applications_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Applications_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Applications_Set_Input>;
  where: Sequent_Backend_Applications_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Applications_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Applications_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Applications_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Applications_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Applications_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Applications_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Applications_Set_Input>;
  pk_columns: Sequent_Backend_Applications_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Applications_ManyArgs = {
  updates: Array<Sequent_Backend_Applications_Updates>;
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
export type Mutation_RootUpdate_Sequent_Backend_Ballot_PublicationArgs = {
  _append?: InputMaybe<Sequent_Backend_Ballot_Publication_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Publication_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Ballot_Publication_Set_Input>;
  where: Sequent_Backend_Ballot_Publication_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Ballot_Publication_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Ballot_Publication_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Publication_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Ballot_Publication_Set_Input>;
  pk_columns: Sequent_Backend_Ballot_Publication_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Ballot_Publication_ManyArgs = {
  updates: Array<Sequent_Backend_Ballot_Publication_Updates>;
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
  _inc?: InputMaybe<Sequent_Backend_Keys_Ceremony_Inc_Input>;
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
  _inc?: InputMaybe<Sequent_Backend_Keys_Ceremony_Inc_Input>;
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
export type Mutation_RootUpdate_Sequent_Backend_NotificationArgs = {
  _append?: InputMaybe<Sequent_Backend_Notification_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Notification_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Notification_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Notification_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Notification_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Notification_Set_Input>;
  where: Sequent_Backend_Notification_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Notification_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Notification_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Notification_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Notification_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Notification_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Notification_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Notification_Set_Input>;
  pk_columns: Sequent_Backend_Notification_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Notification_ManyArgs = {
  updates: Array<Sequent_Backend_Notification_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_ReportArgs = {
  _append?: InputMaybe<Sequent_Backend_Report_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Report_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Report_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Report_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Report_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Report_Set_Input>;
  where: Sequent_Backend_Report_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Report_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Report_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Report_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Report_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Report_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Report_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Report_Set_Input>;
  pk_columns: Sequent_Backend_Report_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Report_ManyArgs = {
  updates: Array<Sequent_Backend_Report_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_ContestArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Set_Input>;
  where: Sequent_Backend_Results_Area_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_Contest_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Set_Input>;
  pk_columns: Sequent_Backend_Results_Area_Contest_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_Contest_CandidateArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Set_Input>;
  where: Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_Contest_Candidate_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Set_Input>;
  pk_columns: Sequent_Backend_Results_Area_Contest_Candidate_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_Contest_Candidate_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Area_Contest_Candidate_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Area_Contest_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Area_Contest_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_ContestArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Set_Input>;
  where: Sequent_Backend_Results_Contest_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Contest_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Set_Input>;
  pk_columns: Sequent_Backend_Results_Contest_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Contest_CandidateArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Set_Input>;
  where: Sequent_Backend_Results_Contest_Candidate_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Contest_Candidate_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Set_Input>;
  pk_columns: Sequent_Backend_Results_Contest_Candidate_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Contest_Candidate_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Contest_Candidate_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Contest_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Contest_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_ElectionArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Election_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Election_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Election_Set_Input>;
  where: Sequent_Backend_Results_Election_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Election_AreaArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Election_Area_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Area_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Election_Area_Set_Input>;
  where: Sequent_Backend_Results_Election_Area_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Election_Area_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Election_Area_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Area_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Election_Area_Set_Input>;
  pk_columns: Sequent_Backend_Results_Election_Area_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Election_Area_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Election_Area_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Election_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Election_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Results_Election_Inc_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Election_Set_Input>;
  pk_columns: Sequent_Backend_Results_Election_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Election_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Election_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_EventArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Event_Set_Input>;
  where: Sequent_Backend_Results_Event_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Event_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Results_Event_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Event_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Event_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Results_Event_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Results_Event_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Results_Event_Set_Input>;
  pk_columns: Sequent_Backend_Results_Event_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Results_Event_ManyArgs = {
  updates: Array<Sequent_Backend_Results_Event_Updates>;
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
export type Mutation_RootUpdate_Sequent_Backend_SecretArgs = {
  _append?: InputMaybe<Sequent_Backend_Secret_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Secret_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Secret_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Secret_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Secret_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Secret_Set_Input>;
  where: Sequent_Backend_Secret_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Secret_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Secret_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Secret_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Secret_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Secret_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Secret_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Secret_Set_Input>;
  pk_columns: Sequent_Backend_Secret_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Secret_ManyArgs = {
  updates: Array<Sequent_Backend_Secret_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Support_MaterialArgs = {
  _append?: InputMaybe<Sequent_Backend_Support_Material_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Support_Material_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Support_Material_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Support_Material_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Support_Material_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Support_Material_Set_Input>;
  where: Sequent_Backend_Support_Material_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Support_Material_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Support_Material_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Support_Material_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Support_Material_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Support_Material_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Support_Material_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Support_Material_Set_Input>;
  pk_columns: Sequent_Backend_Support_Material_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Support_Material_ManyArgs = {
  updates: Array<Sequent_Backend_Support_Material_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_SessionArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Session_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Session_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Session_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Inc_Input>;
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
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Inc_Input>;
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
export type Mutation_RootUpdate_Sequent_Backend_Tally_SheetArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Sheet_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Sheet_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Sheet_Set_Input>;
  where: Sequent_Backend_Tally_Sheet_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Sheet_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tally_Sheet_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tally_Sheet_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tally_Sheet_Set_Input>;
  pk_columns: Sequent_Backend_Tally_Sheet_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tally_Sheet_ManyArgs = {
  updates: Array<Sequent_Backend_Tally_Sheet_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tasks_ExecutionArgs = {
  _append?: InputMaybe<Sequent_Backend_Tasks_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tasks_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tasks_Execution_Set_Input>;
  where: Sequent_Backend_Tasks_Execution_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tasks_Execution_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Tasks_Execution_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Tasks_Execution_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Tasks_Execution_Set_Input>;
  pk_columns: Sequent_Backend_Tasks_Execution_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Tasks_Execution_ManyArgs = {
  updates: Array<Sequent_Backend_Tasks_Execution_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_TemplateArgs = {
  _append?: InputMaybe<Sequent_Backend_Template_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Template_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Template_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Template_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Template_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Template_Set_Input>;
  where: Sequent_Backend_Template_Bool_Exp;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Template_By_PkArgs = {
  _append?: InputMaybe<Sequent_Backend_Template_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Template_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Template_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Template_Delete_Key_Input>;
  _prepend?: InputMaybe<Sequent_Backend_Template_Prepend_Input>;
  _set?: InputMaybe<Sequent_Backend_Template_Set_Input>;
  pk_columns: Sequent_Backend_Template_Pk_Columns_Input;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_Template_ManyArgs = {
  updates: Array<Sequent_Backend_Template_Updates>;
};


/** mutation root */
export type Mutation_RootUpdate_Sequent_Backend_TenantArgs = {
  _append?: InputMaybe<Sequent_Backend_Tenant_Append_Input>;
  _delete_at_path?: InputMaybe<Sequent_Backend_Tenant_Delete_At_Path_Input>;
  _delete_elem?: InputMaybe<Sequent_Backend_Tenant_Delete_Elem_Input>;
  _delete_key?: InputMaybe<Sequent_Backend_Tenant_Delete_Key_Input>;
  _inc?: InputMaybe<Sequent_Backend_Tenant_Inc_Input>;
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
  _inc?: InputMaybe<Sequent_Backend_Tenant_Inc_Input>;
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


/** mutation root */
export type Mutation_RootUpdate_Tally_CeremonyArgs = {
  election_event_id: Scalars['uuid']['input'];
  status: Scalars['String']['input'];
  tally_session_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootUpload_SignatureArgs = {
  area_id: Scalars['uuid']['input'];
  document_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
  password: Scalars['String']['input'];
  tally_session_id: Scalars['uuid']['input'];
};


/** mutation root */
export type Mutation_RootUpsert_AreaArgs = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_contest_ids?: InputMaybe<Array<InputMaybe<Scalars['String']['input']>>>;
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id: Scalars['String']['input'];
  id?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name: Scalars['String']['input'];
  parent_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  type?: InputMaybe<Scalars['String']['input']>;
};


/** mutation root */
export type Mutation_RootUpsert_AreasArgs = {
  document_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
};

/** Boolean expression to compare columns of type "numeric". All fields are combined with logical 'AND'. */
export type Numeric_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['numeric']['input']>;
  _gt?: InputMaybe<Scalars['numeric']['input']>;
  _gte?: InputMaybe<Scalars['numeric']['input']>;
  _in?: InputMaybe<Array<Scalars['numeric']['input']>>;
  _is_null?: InputMaybe<Scalars['Boolean']['input']>;
  _lt?: InputMaybe<Scalars['numeric']['input']>;
  _lte?: InputMaybe<Scalars['numeric']['input']>;
  _neq?: InputMaybe<Scalars['numeric']['input']>;
  _nin?: InputMaybe<Array<Scalars['numeric']['input']>>;
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
  count_users: CountUsersOutput;
  /** fetch document */
  fetchDocument?: Maybe<FetchDocumentOutput>;
  /** get election event stats */
  getElectionEventStats?: Maybe<ElectionEventStatsOutput>;
  /** get election event stats */
  getElectionStats?: Maybe<ElectionStatsOutput>;
  get_election_event_monitoring?: Maybe<ElectionEventStatsMonitoringOutput>;
  get_election_monitoring?: Maybe<ElectionStatsMonitoringOutput>;
  /** list permissions */
  get_permissions: GetPermissionsOutput;
  get_roles: GetRolesOutput;
  get_top_votes_by_ip?: Maybe<GetTopCastVotesByIpOutput>;
  get_user_profile_attributes: Array<UserProfileAttribute>;
  get_users: GetUsersOutput;
  /** List Electoral Log */
  listElectoralLog?: Maybe<DataListElectoralLog>;
  /** List PostgreSQL audit logs */
  listPgaudit?: Maybe<DataListPgAudit>;
  /** List electoral log entries of statement_kind CastVote */
  list_cast_vote_messages?: Maybe<ListCastVoteMessagesOutput>;
  list_keys_ceremony?: Maybe<ListKeysCeremonyOutput>;
  list_user_roles: Array<KeycloakRole>;
  /** log an event in immudb */
  logEvent?: Maybe<LogEventOutput>;
  /** fetch data from the table: "sequent_backend.applications" */
  sequent_backend_applications: Array<Sequent_Backend_Applications>;
  /** fetch aggregated fields from the table: "sequent_backend.applications" */
  sequent_backend_applications_aggregate: Sequent_Backend_Applications_Aggregate;
  /** fetch data from the table: "sequent_backend.applications" using primary key columns */
  sequent_backend_applications_by_pk?: Maybe<Sequent_Backend_Applications>;
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
  /** fetch data from the table: "sequent_backend.ballot_publication" */
  sequent_backend_ballot_publication: Array<Sequent_Backend_Ballot_Publication>;
  /** fetch aggregated fields from the table: "sequent_backend.ballot_publication" */
  sequent_backend_ballot_publication_aggregate: Sequent_Backend_Ballot_Publication_Aggregate;
  /** fetch data from the table: "sequent_backend.ballot_publication" using primary key columns */
  sequent_backend_ballot_publication_by_pk?: Maybe<Sequent_Backend_Ballot_Publication>;
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
  /** fetch data from the table: "sequent_backend.notification" */
  sequent_backend_notification: Array<Sequent_Backend_Notification>;
  /** fetch aggregated fields from the table: "sequent_backend.notification" */
  sequent_backend_notification_aggregate: Sequent_Backend_Notification_Aggregate;
  /** fetch data from the table: "sequent_backend.notification" using primary key columns */
  sequent_backend_notification_by_pk?: Maybe<Sequent_Backend_Notification>;
  /** fetch data from the table: "sequent_backend.report" */
  sequent_backend_report: Array<Sequent_Backend_Report>;
  /** fetch aggregated fields from the table: "sequent_backend.report" */
  sequent_backend_report_aggregate: Sequent_Backend_Report_Aggregate;
  /** fetch data from the table: "sequent_backend.report" using primary key columns */
  sequent_backend_report_by_pk?: Maybe<Sequent_Backend_Report>;
  /** fetch data from the table: "sequent_backend.results_area_contest" */
  sequent_backend_results_area_contest: Array<Sequent_Backend_Results_Area_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.results_area_contest" */
  sequent_backend_results_area_contest_aggregate: Sequent_Backend_Results_Area_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.results_area_contest" using primary key columns */
  sequent_backend_results_area_contest_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest>;
  /** fetch data from the table: "sequent_backend.results_area_contest_candidate" */
  sequent_backend_results_area_contest_candidate: Array<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.results_area_contest_candidate" */
  sequent_backend_results_area_contest_candidate_aggregate: Sequent_Backend_Results_Area_Contest_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.results_area_contest_candidate" using primary key columns */
  sequent_backend_results_area_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** fetch data from the table: "sequent_backend.results_contest" */
  sequent_backend_results_contest: Array<Sequent_Backend_Results_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.results_contest" */
  sequent_backend_results_contest_aggregate: Sequent_Backend_Results_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.results_contest" using primary key columns */
  sequent_backend_results_contest_by_pk?: Maybe<Sequent_Backend_Results_Contest>;
  /** fetch data from the table: "sequent_backend.results_contest_candidate" */
  sequent_backend_results_contest_candidate: Array<Sequent_Backend_Results_Contest_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.results_contest_candidate" */
  sequent_backend_results_contest_candidate_aggregate: Sequent_Backend_Results_Contest_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.results_contest_candidate" using primary key columns */
  sequent_backend_results_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Contest_Candidate>;
  /** fetch data from the table: "sequent_backend.results_election" */
  sequent_backend_results_election: Array<Sequent_Backend_Results_Election>;
  /** fetch aggregated fields from the table: "sequent_backend.results_election" */
  sequent_backend_results_election_aggregate: Sequent_Backend_Results_Election_Aggregate;
  /** fetch data from the table: "sequent_backend.results_election_area" */
  sequent_backend_results_election_area: Array<Sequent_Backend_Results_Election_Area>;
  /** fetch aggregated fields from the table: "sequent_backend.results_election_area" */
  sequent_backend_results_election_area_aggregate: Sequent_Backend_Results_Election_Area_Aggregate;
  /** fetch data from the table: "sequent_backend.results_election_area" using primary key columns */
  sequent_backend_results_election_area_by_pk?: Maybe<Sequent_Backend_Results_Election_Area>;
  /** fetch data from the table: "sequent_backend.results_election" using primary key columns */
  sequent_backend_results_election_by_pk?: Maybe<Sequent_Backend_Results_Election>;
  /** fetch data from the table: "sequent_backend.results_event" */
  sequent_backend_results_event: Array<Sequent_Backend_Results_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.results_event" */
  sequent_backend_results_event_aggregate: Sequent_Backend_Results_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.results_event" using primary key columns */
  sequent_backend_results_event_by_pk?: Maybe<Sequent_Backend_Results_Event>;
  /** fetch data from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_aggregate: Sequent_Backend_Scheduled_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.scheduled_event" using primary key columns */
  sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table: "sequent_backend.secret" */
  sequent_backend_secret: Array<Sequent_Backend_Secret>;
  /** fetch aggregated fields from the table: "sequent_backend.secret" */
  sequent_backend_secret_aggregate: Sequent_Backend_Secret_Aggregate;
  /** fetch data from the table: "sequent_backend.secret" using primary key columns */
  sequent_backend_secret_by_pk?: Maybe<Sequent_Backend_Secret>;
  /** fetch data from the table: "sequent_backend.support_material" */
  sequent_backend_support_material: Array<Sequent_Backend_Support_Material>;
  /** fetch aggregated fields from the table: "sequent_backend.support_material" */
  sequent_backend_support_material_aggregate: Sequent_Backend_Support_Material_Aggregate;
  /** fetch data from the table: "sequent_backend.support_material" using primary key columns */
  sequent_backend_support_material_by_pk?: Maybe<Sequent_Backend_Support_Material>;
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
  /** fetch data from the table: "sequent_backend.tally_sheet" */
  sequent_backend_tally_sheet: Array<Sequent_Backend_Tally_Sheet>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_sheet" */
  sequent_backend_tally_sheet_aggregate: Sequent_Backend_Tally_Sheet_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_sheet" using primary key columns */
  sequent_backend_tally_sheet_by_pk?: Maybe<Sequent_Backend_Tally_Sheet>;
  /** fetch data from the table: "sequent_backend.tasks_execution" */
  sequent_backend_tasks_execution: Array<Sequent_Backend_Tasks_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.tasks_execution" */
  sequent_backend_tasks_execution_aggregate: Sequent_Backend_Tasks_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.tasks_execution" using primary key columns */
  sequent_backend_tasks_execution_by_pk?: Maybe<Sequent_Backend_Tasks_Execution>;
  /** fetch data from the table: "sequent_backend.template" */
  sequent_backend_template: Array<Sequent_Backend_Template>;
  /** fetch aggregated fields from the table: "sequent_backend.template" */
  sequent_backend_template_aggregate: Sequent_Backend_Template_Aggregate;
  /** fetch data from the table: "sequent_backend.template" using primary key columns */
  sequent_backend_template_by_pk?: Maybe<Sequent_Backend_Template>;
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


export type Query_RootCount_UsersArgs = {
  body: CountUsersInput;
};


export type Query_RootFetchDocumentArgs = {
  document_id: Scalars['String']['input'];
  election_event_id?: InputMaybe<Scalars['String']['input']>;
};


export type Query_RootGetElectionEventStatsArgs = {
  object: ElectionEventStatsInput;
};


export type Query_RootGetElectionStatsArgs = {
  object: ElectionStatsInput;
};


export type Query_RootGet_Election_Event_MonitoringArgs = {
  election_event_id: Scalars['uuid']['input'];
};


export type Query_RootGet_Election_MonitoringArgs = {
  election_event_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
};


export type Query_RootGet_PermissionsArgs = {
  body: GetPermissionsInput;
};


export type Query_RootGet_RolesArgs = {
  body: GetRolesInput;
};


export type Query_RootGet_Top_Votes_By_IpArgs = {
  body: GetTopCastVotesByIpInput;
};


export type Query_RootGet_User_Profile_AttributesArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
};


export type Query_RootGet_UsersArgs = {
  body: GetUsersInput;
};


export type Query_RootListElectoralLogArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  filter?: InputMaybe<ElectoralLogFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<ElectoralLogOrderBy>;
};


export type Query_RootListPgauditArgs = {
  audit_table?: InputMaybe<PgAuditTable>;
  filter?: InputMaybe<PgAuditFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<PgAuditOrderBy>;
};


export type Query_RootList_Cast_Vote_MessagesArgs = {
  ballot_id: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  election_id?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<ElectoralLogOrderBy>;
  tenant_id: Scalars['String']['input'];
};


export type Query_RootList_Keys_CeremonyArgs = {
  election_event_id: Scalars['String']['input'];
};


export type Query_RootList_User_RolesArgs = {
  election_event_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id: Scalars['String']['input'];
  user_id: Scalars['String']['input'];
};


export type Query_RootLogEventArgs = {
  body: Scalars['String']['input'];
  election_event_id: Scalars['String']['input'];
  message_type: Scalars['String']['input'];
  user_id?: InputMaybe<Scalars['String']['input']>;
};


export type Query_RootSequent_Backend_ApplicationsArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Applications_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Applications_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
};


export type Query_RootSequent_Backend_Applications_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Applications_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Applications_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
};


export type Query_RootSequent_Backend_Applications_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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


export type Query_RootSequent_Backend_Ballot_PublicationArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
};


export type Query_RootSequent_Backend_Ballot_Publication_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
};


export type Query_RootSequent_Backend_Ballot_Publication_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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


export type Query_RootSequent_Backend_NotificationArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Notification_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Notification_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};


export type Query_RootSequent_Backend_Notification_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Notification_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Notification_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};


export type Query_RootSequent_Backend_Notification_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_ReportArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Report_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Report_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};


export type Query_RootSequent_Backend_Report_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Report_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Report_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};


export type Query_RootSequent_Backend_Report_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_Area_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Area_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Area_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_Area_Contest_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Area_Contest_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Area_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_Contest_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Contest_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_ElectionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Election_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Election_AreaArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Election_Area_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Election_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Results_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
};


export type Query_RootSequent_Backend_Results_Event_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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


export type Query_RootSequent_Backend_SecretArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Secret_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Secret_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};


export type Query_RootSequent_Backend_Secret_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Secret_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Secret_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};


export type Query_RootSequent_Backend_Secret_By_PkArgs = {
  id: Scalars['uuid']['input'];
  key: Scalars['String']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Support_MaterialArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Support_Material_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Support_Material_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
};


export type Query_RootSequent_Backend_Support_Material_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Support_Material_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Support_Material_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
};


export type Query_RootSequent_Backend_Support_Material_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
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


export type Query_RootSequent_Backend_Tally_SheetArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Sheet_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tally_Sheet_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_Tasks_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tasks_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};


export type Query_RootSequent_Backend_Tasks_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Query_RootSequent_Backend_TemplateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Template_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Template_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
};


export type Query_RootSequent_Backend_Template_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Template_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Template_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
};


export type Query_RootSequent_Backend_Template_By_PkArgs = {
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

/** columns and relationships of "sequent_backend.applications" */
export type Sequent_Backend_Applications = {
  __typename?: 'sequent_backend_applications';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  applicant_data: Scalars['jsonb']['output'];
  applicant_id: Scalars['String']['output'];
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at: Scalars['timestamptz']['output'];
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
  status: Scalars['String']['output'];
  tenant_id: Scalars['uuid']['output'];
  updated_at: Scalars['timestamptz']['output'];
  verification_type: Scalars['String']['output'];
};


/** columns and relationships of "sequent_backend.applications" */
export type Sequent_Backend_ApplicationsAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.applications" */
export type Sequent_Backend_ApplicationsApplicant_DataArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.applications" */
export type Sequent_Backend_ApplicationsLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.applications" */
export type Sequent_Backend_Applications_Aggregate = {
  __typename?: 'sequent_backend_applications_aggregate';
  aggregate?: Maybe<Sequent_Backend_Applications_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Applications>;
};

/** aggregate fields of "sequent_backend.applications" */
export type Sequent_Backend_Applications_Aggregate_Fields = {
  __typename?: 'sequent_backend_applications_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Applications_Max_Fields>;
  min?: Maybe<Sequent_Backend_Applications_Min_Fields>;
};


/** aggregate fields of "sequent_backend.applications" */
export type Sequent_Backend_Applications_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Applications_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Applications_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.applications". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Applications_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Applications_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Applications_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  applicant_data?: InputMaybe<Jsonb_Comparison_Exp>;
  applicant_id?: InputMaybe<String_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  permission_label?: InputMaybe<String_Comparison_Exp>;
  status?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  verification_type?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.applications" */
export enum Sequent_Backend_Applications_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  ApplicationsPkey = 'applications_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Applications_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  applicant_data?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Applications_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  applicant_data?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Applications_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  applicant_data?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.applications" */
export type Sequent_Backend_Applications_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_id?: InputMaybe<Scalars['String']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  verification_type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Applications_Max_Fields = {
  __typename?: 'sequent_backend_applications_max_fields';
  applicant_id?: Maybe<Scalars['String']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
  status?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
  verification_type?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Applications_Min_Fields = {
  __typename?: 'sequent_backend_applications_min_fields';
  applicant_id?: Maybe<Scalars['String']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
  status?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
  verification_type?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.applications" */
export type Sequent_Backend_Applications_Mutation_Response = {
  __typename?: 'sequent_backend_applications_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Applications>;
};

/** on_conflict condition type for table "sequent_backend.applications" */
export type Sequent_Backend_Applications_On_Conflict = {
  constraint: Sequent_Backend_Applications_Constraint;
  update_columns?: Array<Sequent_Backend_Applications_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.applications". */
export type Sequent_Backend_Applications_Order_By = {
  annotations?: InputMaybe<Order_By>;
  applicant_data?: InputMaybe<Order_By>;
  applicant_id?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
  verification_type?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.applications */
export type Sequent_Backend_Applications_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Applications_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.applications" */
export enum Sequent_Backend_Applications_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ApplicantData = 'applicant_data',
  /** column name */
  ApplicantId = 'applicant_id',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  VerificationType = 'verification_type'
}

/** input type for updating data in table "sequent_backend.applications" */
export type Sequent_Backend_Applications_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_id?: InputMaybe<Scalars['String']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  verification_type?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_applications" */
export type Sequent_Backend_Applications_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Applications_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Applications_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_data?: InputMaybe<Scalars['jsonb']['input']>;
  applicant_id?: InputMaybe<Scalars['String']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  verification_type?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.applications" */
export enum Sequent_Backend_Applications_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ApplicantData = 'applicant_data',
  /** column name */
  ApplicantId = 'applicant_id',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  UpdatedAt = 'updated_at',
  /** column name */
  VerificationType = 'verification_type'
}

export type Sequent_Backend_Applications_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Applications_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Applications_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Applications_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Applications_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Applications_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Applications_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Applications_Bool_Exp;
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
  parent_id?: Maybe<Scalars['uuid']['output']>;
  presentation?: Maybe<Scalars['jsonb']['output']>;
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


/** columns and relationships of "sequent_backend.area" */
export type Sequent_Backend_AreaPresentationArgs = {
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
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
  parent_id?: InputMaybe<Uuid_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
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
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Area_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Area_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
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
  parent_id?: InputMaybe<Scalars['uuid']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
  parent_id?: Maybe<Scalars['uuid']['output']>;
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
  parent_id?: Maybe<Scalars['uuid']['output']>;
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
  parent_id?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
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
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
  ParentId = 'parent_id',
  /** column name */
  Presentation = 'presentation',
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
  parent_id?: InputMaybe<Scalars['uuid']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
  parent_id?: InputMaybe<Scalars['uuid']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
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
  ParentId = 'parent_id',
  /** column name */
  Presentation = 'presentation',
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

/** columns and relationships of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication = {
  __typename?: 'sequent_backend_ballot_publication';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  created_by_user_id?: Maybe<Scalars['String']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id: Scalars['uuid']['output'];
  is_generated: Scalars['Boolean']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_PublicationAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_PublicationLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Aggregate = {
  __typename?: 'sequent_backend_ballot_publication_aggregate';
  aggregate?: Maybe<Sequent_Backend_Ballot_Publication_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Ballot_Publication>;
};

/** aggregate fields of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Aggregate_Fields = {
  __typename?: 'sequent_backend_ballot_publication_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Ballot_Publication_Max_Fields>;
  min?: Maybe<Sequent_Backend_Ballot_Publication_Min_Fields>;
};


/** aggregate fields of "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Ballot_Publication_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.ballot_publication". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Ballot_Publication_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_by_user_id?: InputMaybe<String_Comparison_Exp>;
  deleted_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_generated?: InputMaybe<Boolean_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  published_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.ballot_publication" */
export enum Sequent_Backend_Ballot_Publication_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  BallotPublicationPkey = 'ballot_publication_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Ballot_Publication_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Ballot_Publication_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Ballot_Publication_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_generated?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Ballot_Publication_Max_Fields = {
  __typename?: 'sequent_backend_ballot_publication_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by_user_id?: Maybe<Scalars['String']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id?: Maybe<Scalars['uuid']['output']>;
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Ballot_Publication_Min_Fields = {
  __typename?: 'sequent_backend_ballot_publication_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by_user_id?: Maybe<Scalars['String']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  id?: Maybe<Scalars['uuid']['output']>;
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Mutation_Response = {
  __typename?: 'sequent_backend_ballot_publication_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Ballot_Publication>;
};

/** on_conflict condition type for table "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_On_Conflict = {
  constraint: Sequent_Backend_Ballot_Publication_Constraint;
  update_columns?: Array<Sequent_Backend_Ballot_Publication_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.ballot_publication". */
export type Sequent_Backend_Ballot_Publication_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  created_by_user_id?: InputMaybe<Order_By>;
  deleted_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  election_ids?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_generated?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  published_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.ballot_publication */
export type Sequent_Backend_Ballot_Publication_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Ballot_Publication_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.ballot_publication" */
export enum Sequent_Backend_Ballot_Publication_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedByUserId = 'created_by_user_id',
  /** column name */
  DeletedAt = 'deleted_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  Id = 'id',
  /** column name */
  IsGenerated = 'is_generated',
  /** column name */
  Labels = 'labels',
  /** column name */
  PublishedAt = 'published_at',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_generated?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_ballot_publication" */
export type Sequent_Backend_Ballot_Publication_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Ballot_Publication_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Ballot_Publication_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_generated?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.ballot_publication" */
export enum Sequent_Backend_Ballot_Publication_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedByUserId = 'created_by_user_id',
  /** column name */
  DeletedAt = 'deleted_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  Id = 'id',
  /** column name */
  IsGenerated = 'is_generated',
  /** column name */
  Labels = 'labels',
  /** column name */
  PublishedAt = 'published_at',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Ballot_Publication_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Ballot_Publication_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Ballot_Publication_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Ballot_Publication_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Ballot_Publication_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Ballot_Publication_Bool_Exp;
};

/** columns and relationships of "sequent_backend.ballot_style" */
export type Sequent_Backend_Ballot_Style = {
  __typename?: 'sequent_backend_ballot_style';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id?: Maybe<Scalars['uuid']['output']>;
  ballot_eml?: Maybe<Scalars['String']['output']>;
  ballot_publication_id: Scalars['uuid']['output'];
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
  ballot_publication_id?: InputMaybe<Uuid_Comparison_Exp>;
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
  ballot_publication_id?: InputMaybe<Scalars['uuid']['input']>;
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
  ballot_publication_id?: Maybe<Scalars['uuid']['output']>;
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
  ballot_publication_id?: Maybe<Scalars['uuid']['output']>;
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
  ballot_publication_id?: InputMaybe<Order_By>;
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
  BallotPublicationId = 'ballot_publication_id',
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
  ballot_publication_id?: InputMaybe<Scalars['uuid']['input']>;
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
  ballot_publication_id?: InputMaybe<Scalars['uuid']['input']>;
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
  BallotPublicationId = 'ballot_publication_id',
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
  ballot_id?: Maybe<Scalars['String']['output']>;
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
  ballot_id?: InputMaybe<String_Comparison_Exp>;
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
  ballot_id?: InputMaybe<Scalars['String']['input']>;
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
  ballot_id?: Maybe<Scalars['String']['output']>;
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
  ballot_id?: Maybe<Scalars['String']['output']>;
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
  ballot_id?: InputMaybe<Order_By>;
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
  BallotId = 'ballot_id',
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
  ballot_id?: InputMaybe<Scalars['String']['input']>;
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
  ballot_id?: InputMaybe<Scalars['String']['input']>;
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
  BallotId = 'ballot_id',
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
  alias?: Maybe<Scalars['String']['output']>;
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
  alias?: InputMaybe<String_Comparison_Exp>;
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
  alias?: InputMaybe<Scalars['String']['input']>;
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
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  tally_configuration?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
  winning_candidates_num?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Contest_Max_Fields = {
  __typename?: 'sequent_backend_contest_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
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
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};

/** order by max() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Max_Order_By = {
  alias?: InputMaybe<Order_By>;
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
  tenant_id?: InputMaybe<Order_By>;
  voting_type?: InputMaybe<Order_By>;
  winning_candidates_num?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Sequent_Backend_Contest_Min_Fields = {
  __typename?: 'sequent_backend_contest_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
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
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
  winning_candidates_num?: Maybe<Scalars['Int']['output']>;
};

/** order by min() on columns of table "sequent_backend.contest" */
export type Sequent_Backend_Contest_Min_Order_By = {
  alias?: InputMaybe<Order_By>;
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
  alias?: InputMaybe<Order_By>;
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
  Alias = 'alias',
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
  alias?: InputMaybe<Scalars['String']['input']>;
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
  alias?: InputMaybe<Scalars['String']['input']>;
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
  Alias = 'alias',
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
  size?: Maybe<Scalars['bigint']['output']>;
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
  size?: InputMaybe<Bigint_Comparison_Exp>;
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
  size?: InputMaybe<Scalars['bigint']['input']>;
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
  size?: InputMaybe<Scalars['bigint']['input']>;
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
  size?: Maybe<Scalars['bigint']['output']>;
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
  size?: Maybe<Scalars['bigint']['output']>;
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
  size?: InputMaybe<Scalars['bigint']['input']>;
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
  size?: InputMaybe<Scalars['bigint']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Document_Sum_Fields = {
  __typename?: 'sequent_backend_document_sum_fields';
  size?: Maybe<Scalars['bigint']['output']>;
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
  description?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  eml?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  image_document_id?: Maybe<Scalars['String']['output']>;
  initialization_report_generated?: Maybe<Scalars['Boolean']['output']>;
  is_consolidated_ballot_encoding?: Maybe<Scalars['Boolean']['output']>;
  is_kiosk?: Maybe<Scalars['Boolean']['output']>;
  keys_ceremony_id?: Maybe<Scalars['uuid']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name: Scalars['String']['output'];
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
  presentation?: Maybe<Scalars['jsonb']['output']>;
  receipts?: Maybe<Scalars['jsonb']['output']>;
  spoil_ballot_option?: Maybe<Scalars['Boolean']['output']>;
  statistics?: Maybe<Scalars['jsonb']['output']>;
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
export type Sequent_Backend_ElectionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionPresentationArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionReceiptsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.election" */
export type Sequent_Backend_ElectionStatisticsArgs = {
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
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  receipts?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  description?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  eml?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  image_document_id?: InputMaybe<String_Comparison_Exp>;
  initialization_report_generated?: InputMaybe<Boolean_Comparison_Exp>;
  is_consolidated_ballot_encoding?: InputMaybe<Boolean_Comparison_Exp>;
  is_kiosk?: InputMaybe<Boolean_Comparison_Exp>;
  keys_ceremony_id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  num_allowed_revotes?: InputMaybe<Int_Comparison_Exp>;
  permission_label?: InputMaybe<String_Comparison_Exp>;
  presentation?: InputMaybe<Jsonb_Comparison_Exp>;
  receipts?: InputMaybe<Jsonb_Comparison_Exp>;
  spoil_ballot_option?: InputMaybe<Boolean_Comparison_Exp>;
  statistics?: InputMaybe<Jsonb_Comparison_Exp>;
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
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
  receipts?: InputMaybe<Array<Scalars['String']['input']>>;
  statistics?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
  receipts?: InputMaybe<Scalars['Int']['input']>;
  statistics?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
  receipts?: InputMaybe<Scalars['String']['input']>;
  statistics?: InputMaybe<Scalars['String']['input']>;
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
  statistics?: Maybe<Scalars['jsonb']['output']>;
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
export type Sequent_Backend_Election_EventStatisticsArgs = {
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
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  statistics?: InputMaybe<Jsonb_Comparison_Exp>;
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
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  presentation?: InputMaybe<Array<Scalars['String']['input']>>;
  statistics?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Election_Event_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  presentation?: InputMaybe<Scalars['Int']['input']>;
  statistics?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Election_Event_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  bulletin_board_reference?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['String']['input']>;
  statistics?: InputMaybe<Scalars['String']['input']>;
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
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  statistics?: InputMaybe<Order_By>;
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
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  Statistics = 'statistics',
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
  description?: InputMaybe<Scalars['String']['input']>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  description?: InputMaybe<Scalars['String']['input']>;
  encryption_protocol?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_archived?: InputMaybe<Scalars['Boolean']['input']>;
  is_audit?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  public_key?: InputMaybe<Scalars['String']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  Statistics = 'statistics',
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
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  initialization_report_generated?: InputMaybe<Scalars['Boolean']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  receipts?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  keys_ceremony_id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
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
  keys_ceremony_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
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
  keys_ceremony_id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  num_allowed_revotes?: Maybe<Scalars['Int']['output']>;
  permission_label?: Maybe<Scalars['String']['output']>;
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
  keys_ceremony_id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
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
  description?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  eml?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  image_document_id?: InputMaybe<Order_By>;
  initialization_report_generated?: InputMaybe<Order_By>;
  is_consolidated_ballot_encoding?: InputMaybe<Order_By>;
  is_kiosk?: InputMaybe<Order_By>;
  keys_ceremony_id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  num_allowed_revotes?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
  presentation?: InputMaybe<Order_By>;
  receipts?: InputMaybe<Order_By>;
  spoil_ballot_option?: InputMaybe<Order_By>;
  statistics?: InputMaybe<Order_By>;
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
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  receipts?: InputMaybe<Scalars['jsonb']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  InitializationReportGenerated = 'initialization_report_generated',
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  KeysCeremonyId = 'keys_ceremony_id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  NumAllowedRevotes = 'num_allowed_revotes',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  Receipts = 'receipts',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option',
  /** column name */
  Statistics = 'statistics',
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
  InitializationReportGenerated = 'initialization_report_generated',
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
  InitializationReportGenerated = 'initialization_report_generated',
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
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  initialization_report_generated?: InputMaybe<Scalars['Boolean']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  receipts?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  description?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  eml?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  image_document_id?: InputMaybe<Scalars['String']['input']>;
  initialization_report_generated?: InputMaybe<Scalars['Boolean']['input']>;
  is_consolidated_ballot_encoding?: InputMaybe<Scalars['Boolean']['input']>;
  is_kiosk?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  num_allowed_revotes?: InputMaybe<Scalars['Int']['input']>;
  permission_label?: InputMaybe<Scalars['String']['input']>;
  presentation?: InputMaybe<Scalars['jsonb']['input']>;
  receipts?: InputMaybe<Scalars['jsonb']['input']>;
  spoil_ballot_option?: InputMaybe<Scalars['Boolean']['input']>;
  statistics?: InputMaybe<Scalars['jsonb']['input']>;
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
  InitializationReportGenerated = 'initialization_report_generated',
  /** column name */
  IsConsolidatedBallotEncoding = 'is_consolidated_ballot_encoding',
  /** column name */
  IsKiosk = 'is_kiosk',
  /** column name */
  KeysCeremonyId = 'keys_ceremony_id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  NumAllowedRevotes = 'num_allowed_revotes',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Presentation = 'presentation',
  /** column name */
  Receipts = 'receipts',
  /** column name */
  SpoilBallotOption = 'spoil_ballot_option',
  /** column name */
  Statistics = 'statistics',
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
  is_default?: Maybe<Scalars['Boolean']['output']>;
  /** An array relationship */
  keys_ceremony_trustee_ids: Array<Sequent_Backend_Trustee>;
  /** An aggregate relationship */
  keys_ceremony_trustee_ids_aggregate: Sequent_Backend_Trustee_Aggregate;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at: Scalars['timestamptz']['output'];
  name?: Maybe<Scalars['String']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  settings?: Maybe<Scalars['jsonb']['output']>;
  status?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  threshold: Scalars['Int']['output'];
  trustee_ids: Array<Scalars['uuid']['output']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyKeys_Ceremony_Trustee_IdsArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyKeys_Ceremony_Trustee_Ids_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Trustee_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonyLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_CeremonySettingsArgs = {
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
  avg?: Maybe<Sequent_Backend_Keys_Ceremony_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Keys_Ceremony_Max_Fields>;
  min?: Maybe<Sequent_Backend_Keys_Ceremony_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Keys_Ceremony_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Keys_Ceremony_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Keys_Ceremony_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Keys_Ceremony_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Keys_Ceremony_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Keys_Ceremony_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Keys_Ceremony_Variance_Fields>;
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Keys_Ceremony_Avg_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_avg_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
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
  is_default?: InputMaybe<Boolean_Comparison_Exp>;
  keys_ceremony_trustee_ids?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
  keys_ceremony_trustee_ids_aggregate?: InputMaybe<Sequent_Backend_Trustee_Aggregate_Bool_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  permission_label?: InputMaybe<String_Array_Comparison_Exp>;
  settings?: InputMaybe<Jsonb_Comparison_Exp>;
  status?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  threshold?: InputMaybe<Int_Comparison_Exp>;
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
  settings?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Keys_Ceremony_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  settings?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Keys_Ceremony_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  settings?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Inc_Input = {
  threshold?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.keys_ceremony" */
export type Sequent_Backend_Keys_Ceremony_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_default?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_trustee_ids?: InputMaybe<Sequent_Backend_Trustee_Arr_Rel_Insert_Input>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
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
  name?: Maybe<Scalars['String']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  threshold?: Maybe<Scalars['Int']['output']>;
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
  name?: Maybe<Scalars['String']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  threshold?: Maybe<Scalars['Int']['output']>;
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
  is_default?: InputMaybe<Order_By>;
  keys_ceremony_trustee_ids_aggregate?: InputMaybe<Sequent_Backend_Trustee_Aggregate_Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
  settings?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  threshold?: InputMaybe<Order_By>;
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
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
  IsDefault = 'is_default',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Settings = 'settings',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Threshold = 'threshold',
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
  is_default?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Keys_Ceremony_Stddev_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_stddev_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Keys_Ceremony_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_stddev_pop_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Keys_Ceremony_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_stddev_samp_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
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
  is_default?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
  trustee_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Keys_Ceremony_Sum_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_sum_fields';
  threshold?: Maybe<Scalars['Int']['output']>;
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
  IsDefault = 'is_default',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  Settings = 'settings',
  /** column name */
  Status = 'status',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Threshold = 'threshold',
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
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Keys_Ceremony_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Keys_Ceremony_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Keys_Ceremony_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Keys_Ceremony_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Keys_Ceremony_Var_Pop_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_var_pop_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Keys_Ceremony_Var_Samp_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_var_samp_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Keys_Ceremony_Variance_Fields = {
  __typename?: 'sequent_backend_keys_ceremony_variance_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
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

/** columns and relationships of "sequent_backend.notification" */
export type Sequent_Backend_Notification = {
  __typename?: 'sequent_backend_notification';
  alias?: Maybe<Scalars['String']['output']>;
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  election_event_id: Scalars['uuid']['output'];
  election_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  template_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id: Scalars['uuid']['output'];
  type?: Maybe<Scalars['String']['output']>;
  updated_at: Scalars['timestamptz']['output'];
};


/** columns and relationships of "sequent_backend.notification" */
export type Sequent_Backend_NotificationAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.notification" */
export type Sequent_Backend_NotificationLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.notification" */
export type Sequent_Backend_Notification_Aggregate = {
  __typename?: 'sequent_backend_notification_aggregate';
  aggregate?: Maybe<Sequent_Backend_Notification_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Notification>;
};

/** aggregate fields of "sequent_backend.notification" */
export type Sequent_Backend_Notification_Aggregate_Fields = {
  __typename?: 'sequent_backend_notification_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Notification_Max_Fields>;
  min?: Maybe<Sequent_Backend_Notification_Min_Fields>;
};


/** aggregate fields of "sequent_backend.notification" */
export type Sequent_Backend_Notification_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Notification_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Notification_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.notification". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Notification_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Notification_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Notification_Bool_Exp>>;
  alias?: InputMaybe<String_Comparison_Exp>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  template_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.notification" */
export enum Sequent_Backend_Notification_Constraint {
  /** unique or primary key constraint on columns "id" */
  NotificationPkey = 'notification_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Notification_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Notification_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Notification_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.notification" */
export type Sequent_Backend_Notification_Insert_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  template_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Notification_Max_Fields = {
  __typename?: 'sequent_backend_notification_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  template_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Notification_Min_Fields = {
  __typename?: 'sequent_backend_notification_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  template_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** response of any mutation on the table "sequent_backend.notification" */
export type Sequent_Backend_Notification_Mutation_Response = {
  __typename?: 'sequent_backend_notification_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Notification>;
};

/** on_conflict condition type for table "sequent_backend.notification" */
export type Sequent_Backend_Notification_On_Conflict = {
  constraint: Sequent_Backend_Notification_Constraint;
  update_columns?: Array<Sequent_Backend_Notification_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.notification". */
export type Sequent_Backend_Notification_Order_By = {
  alias?: InputMaybe<Order_By>;
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  template_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.notification */
export type Sequent_Backend_Notification_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Notification_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.notification" */
export enum Sequent_Backend_Notification_Select_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
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
  Name = 'name',
  /** column name */
  TemplateId = 'template_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type',
  /** column name */
  UpdatedAt = 'updated_at'
}

/** input type for updating data in table "sequent_backend.notification" */
export type Sequent_Backend_Notification_Set_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  template_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** Streaming cursor of the table "sequent_backend_notification" */
export type Sequent_Backend_Notification_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Notification_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Notification_Stream_Cursor_Value_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  template_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** update columns of table "sequent_backend.notification" */
export enum Sequent_Backend_Notification_Update_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
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
  Name = 'name',
  /** column name */
  TemplateId = 'template_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type',
  /** column name */
  UpdatedAt = 'updated_at'
}

export type Sequent_Backend_Notification_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Notification_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Notification_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Notification_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Notification_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Notification_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Notification_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Notification_Bool_Exp;
};

/** columns and relationships of "sequent_backend.report" */
export type Sequent_Backend_Report = {
  __typename?: 'sequent_backend_report';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  cron_config?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id?: Maybe<Scalars['uuid']['output']>;
  encryption_policy: Scalars['String']['output'];
  id: Scalars['uuid']['output'];
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  report_type: Scalars['String']['output'];
  template_alias?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.report" */
export type Sequent_Backend_ReportCron_ConfigArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.report" */
export type Sequent_Backend_Report_Aggregate = {
  __typename?: 'sequent_backend_report_aggregate';
  aggregate?: Maybe<Sequent_Backend_Report_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Report>;
};

/** aggregate fields of "sequent_backend.report" */
export type Sequent_Backend_Report_Aggregate_Fields = {
  __typename?: 'sequent_backend_report_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Report_Max_Fields>;
  min?: Maybe<Sequent_Backend_Report_Min_Fields>;
};


/** aggregate fields of "sequent_backend.report" */
export type Sequent_Backend_Report_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Report_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Report_Append_Input = {
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.report". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Report_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Report_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Report_Bool_Exp>>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  cron_config?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  encryption_policy?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  permission_label?: InputMaybe<String_Array_Comparison_Exp>;
  report_type?: InputMaybe<String_Comparison_Exp>;
  template_alias?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.report" */
export enum Sequent_Backend_Report_Constraint {
  /** unique or primary key constraint on columns "id" */
  ReportPkey = 'report_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Report_Delete_At_Path_Input = {
  cron_config?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Report_Delete_Elem_Input = {
  cron_config?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Report_Delete_Key_Input = {
  cron_config?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.report" */
export type Sequent_Backend_Report_Insert_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  encryption_policy?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  report_type?: InputMaybe<Scalars['String']['input']>;
  template_alias?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Report_Max_Fields = {
  __typename?: 'sequent_backend_report_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  encryption_policy?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  report_type?: Maybe<Scalars['String']['output']>;
  template_alias?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Report_Min_Fields = {
  __typename?: 'sequent_backend_report_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  encryption_policy?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  report_type?: Maybe<Scalars['String']['output']>;
  template_alias?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.report" */
export type Sequent_Backend_Report_Mutation_Response = {
  __typename?: 'sequent_backend_report_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Report>;
};

/** on_conflict condition type for table "sequent_backend.report" */
export type Sequent_Backend_Report_On_Conflict = {
  constraint: Sequent_Backend_Report_Constraint;
  update_columns?: Array<Sequent_Backend_Report_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.report". */
export type Sequent_Backend_Report_Order_By = {
  created_at?: InputMaybe<Order_By>;
  cron_config?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  encryption_policy?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
  report_type?: InputMaybe<Order_By>;
  template_alias?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.report */
export type Sequent_Backend_Report_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Report_Prepend_Input = {
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.report" */
export enum Sequent_Backend_Report_Select_Column {
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CronConfig = 'cron_config',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  EncryptionPolicy = 'encryption_policy',
  /** column name */
  Id = 'id',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  ReportType = 'report_type',
  /** column name */
  TemplateAlias = 'template_alias',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.report" */
export type Sequent_Backend_Report_Set_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  encryption_policy?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  report_type?: InputMaybe<Scalars['String']['input']>;
  template_alias?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_report" */
export type Sequent_Backend_Report_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Report_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Report_Stream_Cursor_Value_Input = {
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  encryption_policy?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  report_type?: InputMaybe<Scalars['String']['input']>;
  template_alias?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.report" */
export enum Sequent_Backend_Report_Update_Column {
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CronConfig = 'cron_config',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  EncryptionPolicy = 'encryption_policy',
  /** column name */
  Id = 'id',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  ReportType = 'report_type',
  /** column name */
  TemplateAlias = 'template_alias',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Report_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Report_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Report_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Report_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Report_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Report_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Report_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Report_Bool_Exp;
};

/** columns and relationships of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest = {
  __typename?: 'sequent_backend_results_area_contest';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id: Scalars['uuid']['output'];
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id: Scalars['uuid']['output'];
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id: Scalars['uuid']['output'];
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
};


/** columns and relationships of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_ContestAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_ContestDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_ContestLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Aggregate = {
  __typename?: 'sequent_backend_results_area_contest_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Area_Contest_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Area_Contest>;
};

/** aggregate fields of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_area_contest_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Results_Area_Contest_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Area_Contest_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Area_Contest_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Results_Area_Contest_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Results_Area_Contest_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Results_Area_Contest_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Results_Area_Contest_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Results_Area_Contest_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Results_Area_Contest_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Results_Area_Contest_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Area_Contest_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Results_Area_Contest_Avg_Fields = {
  __typename?: 'sequent_backend_results_area_contest_avg_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_area_contest". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Area_Contest_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  blank_votes?: InputMaybe<Int_Comparison_Exp>;
  blank_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  elegible_census?: InputMaybe<Int_Comparison_Exp>;
  explicit_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  explicit_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  implicit_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  implicit_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  total_auditable_votes?: InputMaybe<Int_Comparison_Exp>;
  total_auditable_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  total_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_valid_votes?: InputMaybe<Int_Comparison_Exp>;
  total_valid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_votes?: InputMaybe<Int_Comparison_Exp>;
  total_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
};

/** columns and relationships of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate = {
  __typename?: 'sequent_backend_results_area_contest_candidate';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id: Scalars['uuid']['output'];
  candidate_id: Scalars['uuid']['output'];
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id: Scalars['uuid']['output'];
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
  winning_position?: Maybe<Scalars['Int']['output']>;
};


/** columns and relationships of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_CandidateAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_CandidateDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_CandidateLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Aggregate = {
  __typename?: 'sequent_backend_results_area_contest_candidate_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Area_Contest_Candidate>;
};

/** aggregate fields of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Area_Contest_Candidate_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Avg_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_avg_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_area_contest_candidate". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  candidate_id?: InputMaybe<Uuid_Comparison_Exp>;
  cast_votes?: InputMaybe<Int_Comparison_Exp>;
  cast_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  points?: InputMaybe<Int_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  winning_position?: InputMaybe<Int_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.results_area_contest_candidate" */
export enum Sequent_Backend_Results_Area_Contest_Candidate_Constraint {
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsAreaContestCandidatePkey = 'results_area_contest_candidate_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Area_Contest_Candidate_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Area_Contest_Candidate_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Area_Contest_Candidate_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Inc_Input = {
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Max_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  candidate_id?: Maybe<Scalars['uuid']['output']>;
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Min_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  candidate_id?: Maybe<Scalars['uuid']['output']>;
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Mutation_Response = {
  __typename?: 'sequent_backend_results_area_contest_candidate_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Area_Contest_Candidate>;
};

/** on_conflict condition type for table "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_On_Conflict = {
  constraint: Sequent_Backend_Results_Area_Contest_Candidate_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Area_Contest_Candidate_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_area_contest_candidate". */
export type Sequent_Backend_Results_Area_Contest_Candidate_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  candidate_id?: InputMaybe<Order_By>;
  cast_votes?: InputMaybe<Order_By>;
  cast_votes_percent?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  points?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  winning_position?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_area_contest_candidate */
export type Sequent_Backend_Results_Area_Contest_Candidate_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Area_Contest_Candidate_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_area_contest_candidate" */
export enum Sequent_Backend_Results_Area_Contest_Candidate_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CandidateId = 'candidate_id',
  /** column name */
  CastVotes = 'cast_votes',
  /** column name */
  CastVotesPercent = 'cast_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  Points = 'points',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  WinningPosition = 'winning_position'
}

/** input type for updating data in table "sequent_backend.results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_stddev_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_stddev_pop_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_stddev_samp_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_results_area_contest_candidate" */
export type Sequent_Backend_Results_Area_Contest_Candidate_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Area_Contest_Candidate_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Area_Contest_Candidate_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Sum_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_sum_fields';
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.results_area_contest_candidate" */
export enum Sequent_Backend_Results_Area_Contest_Candidate_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CandidateId = 'candidate_id',
  /** column name */
  CastVotes = 'cast_votes',
  /** column name */
  CastVotesPercent = 'cast_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  Points = 'points',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  WinningPosition = 'winning_position'
}

export type Sequent_Backend_Results_Area_Contest_Candidate_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Var_Pop_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_var_pop_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Var_Samp_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_var_samp_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Results_Area_Contest_Candidate_Variance_Fields = {
  __typename?: 'sequent_backend_results_area_contest_candidate_variance_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** unique or primary key constraints on table "sequent_backend.results_area_contest" */
export enum Sequent_Backend_Results_Area_Contest_Constraint {
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsAreaContestPkey = 'results_area_contest_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Area_Contest_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Area_Contest_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Area_Contest_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Inc_Input = {
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Area_Contest_Max_Fields = {
  __typename?: 'sequent_backend_results_area_contest_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Area_Contest_Min_Fields = {
  __typename?: 'sequent_backend_results_area_contest_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Mutation_Response = {
  __typename?: 'sequent_backend_results_area_contest_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Area_Contest>;
};

/** on_conflict condition type for table "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_On_Conflict = {
  constraint: Sequent_Backend_Results_Area_Contest_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Area_Contest_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_area_contest". */
export type Sequent_Backend_Results_Area_Contest_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  blank_votes?: InputMaybe<Order_By>;
  blank_votes_percent?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  elegible_census?: InputMaybe<Order_By>;
  explicit_invalid_votes?: InputMaybe<Order_By>;
  explicit_invalid_votes_percent?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  implicit_invalid_votes?: InputMaybe<Order_By>;
  implicit_invalid_votes_percent?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  total_auditable_votes?: InputMaybe<Order_By>;
  total_auditable_votes_percent?: InputMaybe<Order_By>;
  total_invalid_votes?: InputMaybe<Order_By>;
  total_invalid_votes_percent?: InputMaybe<Order_By>;
  total_valid_votes?: InputMaybe<Order_By>;
  total_valid_votes_percent?: InputMaybe<Order_By>;
  total_votes?: InputMaybe<Order_By>;
  total_votes_percent?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_area_contest */
export type Sequent_Backend_Results_Area_Contest_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Area_Contest_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_area_contest" */
export enum Sequent_Backend_Results_Area_Contest_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  BlankVotes = 'blank_votes',
  /** column name */
  BlankVotesPercent = 'blank_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  ExplicitInvalidVotes = 'explicit_invalid_votes',
  /** column name */
  ExplicitInvalidVotesPercent = 'explicit_invalid_votes_percent',
  /** column name */
  Id = 'id',
  /** column name */
  ImplicitInvalidVotes = 'implicit_invalid_votes',
  /** column name */
  ImplicitInvalidVotesPercent = 'implicit_invalid_votes_percent',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalAuditableVotes = 'total_auditable_votes',
  /** column name */
  TotalAuditableVotesPercent = 'total_auditable_votes_percent',
  /** column name */
  TotalInvalidVotes = 'total_invalid_votes',
  /** column name */
  TotalInvalidVotesPercent = 'total_invalid_votes_percent',
  /** column name */
  TotalValidVotes = 'total_valid_votes',
  /** column name */
  TotalValidVotesPercent = 'total_valid_votes_percent',
  /** column name */
  TotalVotes = 'total_votes',
  /** column name */
  TotalVotesPercent = 'total_votes_percent'
}

/** input type for updating data in table "sequent_backend.results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Results_Area_Contest_Stddev_Fields = {
  __typename?: 'sequent_backend_results_area_contest_stddev_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Results_Area_Contest_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_results_area_contest_stddev_pop_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Results_Area_Contest_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_results_area_contest_stddev_samp_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_results_area_contest" */
export type Sequent_Backend_Results_Area_Contest_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Area_Contest_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Area_Contest_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Results_Area_Contest_Sum_Fields = {
  __typename?: 'sequent_backend_results_area_contest_sum_fields';
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
};

/** update columns of table "sequent_backend.results_area_contest" */
export enum Sequent_Backend_Results_Area_Contest_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  BlankVotes = 'blank_votes',
  /** column name */
  BlankVotesPercent = 'blank_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  ExplicitInvalidVotes = 'explicit_invalid_votes',
  /** column name */
  ExplicitInvalidVotesPercent = 'explicit_invalid_votes_percent',
  /** column name */
  Id = 'id',
  /** column name */
  ImplicitInvalidVotes = 'implicit_invalid_votes',
  /** column name */
  ImplicitInvalidVotesPercent = 'implicit_invalid_votes_percent',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalAuditableVotes = 'total_auditable_votes',
  /** column name */
  TotalAuditableVotesPercent = 'total_auditable_votes_percent',
  /** column name */
  TotalInvalidVotes = 'total_invalid_votes',
  /** column name */
  TotalInvalidVotesPercent = 'total_invalid_votes_percent',
  /** column name */
  TotalValidVotes = 'total_valid_votes',
  /** column name */
  TotalValidVotesPercent = 'total_valid_votes_percent',
  /** column name */
  TotalVotes = 'total_votes',
  /** column name */
  TotalVotesPercent = 'total_votes_percent'
}

export type Sequent_Backend_Results_Area_Contest_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Area_Contest_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Area_Contest_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Results_Area_Contest_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Area_Contest_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Area_Contest_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Area_Contest_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Results_Area_Contest_Var_Pop_Fields = {
  __typename?: 'sequent_backend_results_area_contest_var_pop_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Results_Area_Contest_Var_Samp_Fields = {
  __typename?: 'sequent_backend_results_area_contest_var_samp_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Results_Area_Contest_Variance_Fields = {
  __typename?: 'sequent_backend_results_area_contest_variance_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** columns and relationships of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest = {
  __typename?: 'sequent_backend_results_contest';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id: Scalars['uuid']['output'];
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id: Scalars['uuid']['output'];
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
};


/** columns and relationships of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_ContestAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_ContestDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_ContestLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Aggregate = {
  __typename?: 'sequent_backend_results_contest_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Contest_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Contest>;
};

/** aggregate fields of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_contest_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Results_Contest_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Contest_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Contest_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Results_Contest_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Results_Contest_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Results_Contest_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Results_Contest_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Results_Contest_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Results_Contest_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Results_Contest_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Contest_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Contest_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Results_Contest_Avg_Fields = {
  __typename?: 'sequent_backend_results_contest_avg_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_contest". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Contest_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Contest_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Contest_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  blank_votes?: InputMaybe<Int_Comparison_Exp>;
  blank_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  counting_algorithm?: InputMaybe<String_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  elegible_census?: InputMaybe<Int_Comparison_Exp>;
  explicit_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  explicit_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  implicit_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  implicit_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  total_auditable_votes?: InputMaybe<Int_Comparison_Exp>;
  total_auditable_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_invalid_votes?: InputMaybe<Int_Comparison_Exp>;
  total_invalid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_valid_votes?: InputMaybe<Int_Comparison_Exp>;
  total_valid_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  total_votes?: InputMaybe<Int_Comparison_Exp>;
  total_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  voting_type?: InputMaybe<String_Comparison_Exp>;
};

/** columns and relationships of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate = {
  __typename?: 'sequent_backend_results_contest_candidate';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  candidate_id: Scalars['uuid']['output'];
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id: Scalars['uuid']['output'];
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
  winning_position?: Maybe<Scalars['Int']['output']>;
};


/** columns and relationships of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_CandidateAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_CandidateDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_CandidateLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Aggregate = {
  __typename?: 'sequent_backend_results_contest_candidate_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Contest_Candidate_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Contest_Candidate>;
};

/** aggregate fields of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Results_Contest_Candidate_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Contest_Candidate_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Contest_Candidate_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Results_Contest_Candidate_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Results_Contest_Candidate_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Results_Contest_Candidate_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Results_Contest_Candidate_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Results_Contest_Candidate_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Results_Contest_Candidate_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Results_Contest_Candidate_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Contest_Candidate_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Results_Contest_Candidate_Avg_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_avg_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_contest_candidate". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Contest_Candidate_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  candidate_id?: InputMaybe<Uuid_Comparison_Exp>;
  cast_votes?: InputMaybe<Int_Comparison_Exp>;
  cast_votes_percent?: InputMaybe<Numeric_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  points?: InputMaybe<Int_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  winning_position?: InputMaybe<Int_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.results_contest_candidate" */
export enum Sequent_Backend_Results_Contest_Candidate_Constraint {
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsContestCandidatePkey = 'results_contest_candidate_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Contest_Candidate_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Contest_Candidate_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Contest_Candidate_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Inc_Input = {
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Contest_Candidate_Max_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_max_fields';
  candidate_id?: Maybe<Scalars['uuid']['output']>;
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Contest_Candidate_Min_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_min_fields';
  candidate_id?: Maybe<Scalars['uuid']['output']>;
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Mutation_Response = {
  __typename?: 'sequent_backend_results_contest_candidate_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Contest_Candidate>;
};

/** on_conflict condition type for table "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_On_Conflict = {
  constraint: Sequent_Backend_Results_Contest_Candidate_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Contest_Candidate_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_contest_candidate". */
export type Sequent_Backend_Results_Contest_Candidate_Order_By = {
  annotations?: InputMaybe<Order_By>;
  candidate_id?: InputMaybe<Order_By>;
  cast_votes?: InputMaybe<Order_By>;
  cast_votes_percent?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  points?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  winning_position?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_contest_candidate */
export type Sequent_Backend_Results_Contest_Candidate_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Contest_Candidate_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_contest_candidate" */
export enum Sequent_Backend_Results_Contest_Candidate_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CandidateId = 'candidate_id',
  /** column name */
  CastVotes = 'cast_votes',
  /** column name */
  CastVotesPercent = 'cast_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  Points = 'points',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  WinningPosition = 'winning_position'
}

/** input type for updating data in table "sequent_backend.results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Results_Contest_Candidate_Stddev_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_stddev_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Results_Contest_Candidate_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_stddev_pop_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Results_Contest_Candidate_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_stddev_samp_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_results_contest_candidate" */
export type Sequent_Backend_Results_Contest_Candidate_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Contest_Candidate_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Contest_Candidate_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  candidate_id?: InputMaybe<Scalars['uuid']['input']>;
  cast_votes?: InputMaybe<Scalars['Int']['input']>;
  cast_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  points?: InputMaybe<Scalars['Int']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  winning_position?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Results_Contest_Candidate_Sum_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_sum_fields';
  cast_votes?: Maybe<Scalars['Int']['output']>;
  cast_votes_percent?: Maybe<Scalars['numeric']['output']>;
  points?: Maybe<Scalars['Int']['output']>;
  winning_position?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.results_contest_candidate" */
export enum Sequent_Backend_Results_Contest_Candidate_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CandidateId = 'candidate_id',
  /** column name */
  CastVotes = 'cast_votes',
  /** column name */
  CastVotesPercent = 'cast_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  Points = 'points',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  WinningPosition = 'winning_position'
}

export type Sequent_Backend_Results_Contest_Candidate_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Contest_Candidate_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Results_Contest_Candidate_Var_Pop_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_var_pop_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Results_Contest_Candidate_Var_Samp_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_var_samp_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Results_Contest_Candidate_Variance_Fields = {
  __typename?: 'sequent_backend_results_contest_candidate_variance_fields';
  cast_votes?: Maybe<Scalars['Float']['output']>;
  cast_votes_percent?: Maybe<Scalars['Float']['output']>;
  points?: Maybe<Scalars['Float']['output']>;
  winning_position?: Maybe<Scalars['Float']['output']>;
};

/** unique or primary key constraints on table "sequent_backend.results_contest" */
export enum Sequent_Backend_Results_Contest_Constraint {
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsContestPkey = 'results_contest_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Contest_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Contest_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Contest_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Inc_Input = {
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Contest_Max_Fields = {
  __typename?: 'sequent_backend_results_contest_max_fields';
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Contest_Min_Fields = {
  __typename?: 'sequent_backend_results_contest_min_fields';
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  counting_algorithm?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
  voting_type?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Mutation_Response = {
  __typename?: 'sequent_backend_results_contest_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Contest>;
};

/** on_conflict condition type for table "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_On_Conflict = {
  constraint: Sequent_Backend_Results_Contest_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Contest_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_contest". */
export type Sequent_Backend_Results_Contest_Order_By = {
  annotations?: InputMaybe<Order_By>;
  blank_votes?: InputMaybe<Order_By>;
  blank_votes_percent?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  counting_algorithm?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  elegible_census?: InputMaybe<Order_By>;
  explicit_invalid_votes?: InputMaybe<Order_By>;
  explicit_invalid_votes_percent?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  implicit_invalid_votes?: InputMaybe<Order_By>;
  implicit_invalid_votes_percent?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  total_auditable_votes?: InputMaybe<Order_By>;
  total_auditable_votes_percent?: InputMaybe<Order_By>;
  total_invalid_votes?: InputMaybe<Order_By>;
  total_invalid_votes_percent?: InputMaybe<Order_By>;
  total_valid_votes?: InputMaybe<Order_By>;
  total_valid_votes_percent?: InputMaybe<Order_By>;
  total_votes?: InputMaybe<Order_By>;
  total_votes_percent?: InputMaybe<Order_By>;
  voting_type?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_contest */
export type Sequent_Backend_Results_Contest_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Contest_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_contest" */
export enum Sequent_Backend_Results_Contest_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  BlankVotes = 'blank_votes',
  /** column name */
  BlankVotesPercent = 'blank_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CountingAlgorithm = 'counting_algorithm',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  ExplicitInvalidVotes = 'explicit_invalid_votes',
  /** column name */
  ExplicitInvalidVotesPercent = 'explicit_invalid_votes_percent',
  /** column name */
  Id = 'id',
  /** column name */
  ImplicitInvalidVotes = 'implicit_invalid_votes',
  /** column name */
  ImplicitInvalidVotesPercent = 'implicit_invalid_votes_percent',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalAuditableVotes = 'total_auditable_votes',
  /** column name */
  TotalAuditableVotesPercent = 'total_auditable_votes_percent',
  /** column name */
  TotalInvalidVotes = 'total_invalid_votes',
  /** column name */
  TotalInvalidVotesPercent = 'total_invalid_votes_percent',
  /** column name */
  TotalValidVotes = 'total_valid_votes',
  /** column name */
  TotalValidVotesPercent = 'total_valid_votes_percent',
  /** column name */
  TotalVotes = 'total_votes',
  /** column name */
  TotalVotesPercent = 'total_votes_percent',
  /** column name */
  VotingType = 'voting_type'
}

/** input type for updating data in table "sequent_backend.results_contest" */
export type Sequent_Backend_Results_Contest_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Results_Contest_Stddev_Fields = {
  __typename?: 'sequent_backend_results_contest_stddev_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Results_Contest_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_results_contest_stddev_pop_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Results_Contest_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_results_contest_stddev_samp_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_results_contest" */
export type Sequent_Backend_Results_Contest_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Contest_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Contest_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  blank_votes?: InputMaybe<Scalars['Int']['input']>;
  blank_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  counting_algorithm?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  explicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  implicit_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  implicit_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_auditable_votes?: InputMaybe<Scalars['Int']['input']>;
  total_auditable_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_invalid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_invalid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_valid_votes?: InputMaybe<Scalars['Int']['input']>;
  total_valid_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  total_votes?: InputMaybe<Scalars['Int']['input']>;
  total_votes_percent?: InputMaybe<Scalars['numeric']['input']>;
  voting_type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Results_Contest_Sum_Fields = {
  __typename?: 'sequent_backend_results_contest_sum_fields';
  blank_votes?: Maybe<Scalars['Int']['output']>;
  blank_votes_percent?: Maybe<Scalars['numeric']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Int']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_auditable_votes?: Maybe<Scalars['Int']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_invalid_votes?: Maybe<Scalars['Int']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_valid_votes?: Maybe<Scalars['Int']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['numeric']['output']>;
  total_votes?: Maybe<Scalars['Int']['output']>;
  total_votes_percent?: Maybe<Scalars['numeric']['output']>;
};

/** update columns of table "sequent_backend.results_contest" */
export enum Sequent_Backend_Results_Contest_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  BlankVotes = 'blank_votes',
  /** column name */
  BlankVotesPercent = 'blank_votes_percent',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CountingAlgorithm = 'counting_algorithm',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  ExplicitInvalidVotes = 'explicit_invalid_votes',
  /** column name */
  ExplicitInvalidVotesPercent = 'explicit_invalid_votes_percent',
  /** column name */
  Id = 'id',
  /** column name */
  ImplicitInvalidVotes = 'implicit_invalid_votes',
  /** column name */
  ImplicitInvalidVotesPercent = 'implicit_invalid_votes_percent',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalAuditableVotes = 'total_auditable_votes',
  /** column name */
  TotalAuditableVotesPercent = 'total_auditable_votes_percent',
  /** column name */
  TotalInvalidVotes = 'total_invalid_votes',
  /** column name */
  TotalInvalidVotesPercent = 'total_invalid_votes_percent',
  /** column name */
  TotalValidVotes = 'total_valid_votes',
  /** column name */
  TotalValidVotesPercent = 'total_valid_votes_percent',
  /** column name */
  TotalVotes = 'total_votes',
  /** column name */
  TotalVotesPercent = 'total_votes_percent',
  /** column name */
  VotingType = 'voting_type'
}

export type Sequent_Backend_Results_Contest_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Contest_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Contest_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Contest_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Results_Contest_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Contest_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Contest_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Contest_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Results_Contest_Var_Pop_Fields = {
  __typename?: 'sequent_backend_results_contest_var_pop_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Results_Contest_Var_Samp_Fields = {
  __typename?: 'sequent_backend_results_contest_var_samp_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Results_Contest_Variance_Fields = {
  __typename?: 'sequent_backend_results_contest_variance_fields';
  blank_votes?: Maybe<Scalars['Float']['output']>;
  blank_votes_percent?: Maybe<Scalars['Float']['output']>;
  elegible_census?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  explicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes?: Maybe<Scalars['Float']['output']>;
  implicit_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes?: Maybe<Scalars['Float']['output']>;
  total_auditable_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes?: Maybe<Scalars['Float']['output']>;
  total_invalid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_valid_votes?: Maybe<Scalars['Float']['output']>;
  total_valid_votes_percent?: Maybe<Scalars['Float']['output']>;
  total_votes?: Maybe<Scalars['Float']['output']>;
  total_votes_percent?: Maybe<Scalars['Float']['output']>;
};

/** columns and relationships of "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election = {
  __typename?: 'sequent_backend_results_election';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  elegible_census?: Maybe<Scalars['Int']['output']>;
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
  total_voters?: Maybe<Scalars['Int']['output']>;
  total_voters_percent?: Maybe<Scalars['numeric']['output']>;
};


/** columns and relationships of "sequent_backend.results_election" */
export type Sequent_Backend_Results_ElectionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_election" */
export type Sequent_Backend_Results_ElectionDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_election" */
export type Sequent_Backend_Results_ElectionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Aggregate = {
  __typename?: 'sequent_backend_results_election_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Election_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Election>;
};

/** aggregate fields of "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_election_aggregate_fields';
  avg?: Maybe<Sequent_Backend_Results_Election_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Election_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Election_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Results_Election_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Results_Election_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Results_Election_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Results_Election_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Results_Election_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Results_Election_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Results_Election_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Election_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Election_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** columns and relationships of "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area = {
  __typename?: 'sequent_backend_results_election_area';
  area_id: Scalars['uuid']['output'];
  created_at: Scalars['timestamptz']['output'];
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  last_updated_at: Scalars['timestamptz']['output'];
  name?: Maybe<Scalars['String']['output']>;
  results_event_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_AreaDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Aggregate = {
  __typename?: 'sequent_backend_results_election_area_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Election_Area_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Election_Area>;
};

/** aggregate fields of "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_election_area_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Election_Area_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Election_Area_Min_Fields>;
};


/** aggregate fields of "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Election_Area_Append_Input = {
  documents?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_election_area". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Election_Area_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Bool_Exp>>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.results_election_area" */
export enum Sequent_Backend_Results_Election_Area_Constraint {
  /** unique or primary key constraint on columns "id" */
  ResultsElectionAreaIdKey = 'results_election_area_id_key',
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsElectionAreaPkey = 'results_election_area_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Election_Area_Delete_At_Path_Input = {
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Election_Area_Delete_Elem_Input = {
  documents?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Election_Area_Delete_Key_Input = {
  documents?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Insert_Input = {
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Election_Area_Max_Fields = {
  __typename?: 'sequent_backend_results_election_area_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Election_Area_Min_Fields = {
  __typename?: 'sequent_backend_results_election_area_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Mutation_Response = {
  __typename?: 'sequent_backend_results_election_area_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Election_Area>;
};

/** on_conflict condition type for table "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_On_Conflict = {
  constraint: Sequent_Backend_Results_Election_Area_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Election_Area_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_election_area". */
export type Sequent_Backend_Results_Election_Area_Order_By = {
  area_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_election_area */
export type Sequent_Backend_Results_Election_Area_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Election_Area_Prepend_Input = {
  documents?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_election_area" */
export enum Sequent_Backend_Results_Election_Area_Select_Column {
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.results_election_area" */
export type Sequent_Backend_Results_Election_Area_Set_Input = {
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_results_election_area" */
export type Sequent_Backend_Results_Election_Area_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Election_Area_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Election_Area_Stream_Cursor_Value_Input = {
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.results_election_area" */
export enum Sequent_Backend_Results_Election_Area_Update_Column {
  /** column name */
  AreaId = 'area_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  Id = 'id',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Results_Election_Area_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Election_Area_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Area_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Area_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Election_Area_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Election_Area_Bool_Exp;
};

/** aggregate avg on columns */
export type Sequent_Backend_Results_Election_Avg_Fields = {
  __typename?: 'sequent_backend_results_election_avg_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_election". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Election_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Election_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Election_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  elegible_census?: InputMaybe<Int_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  total_voters?: InputMaybe<Int_Comparison_Exp>;
  total_voters_percent?: InputMaybe<Numeric_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.results_election" */
export enum Sequent_Backend_Results_Election_Constraint {
  /** unique or primary key constraint on columns "id", "results_event_id", "tenant_id", "election_event_id" */
  ResultsElectionPkey = 'results_election_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Election_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Election_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Election_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Inc_Input = {
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  total_voters?: InputMaybe<Scalars['Int']['input']>;
  total_voters_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_voters?: InputMaybe<Scalars['Int']['input']>;
  total_voters_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Election_Max_Fields = {
  __typename?: 'sequent_backend_results_election_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_voters?: Maybe<Scalars['Int']['output']>;
  total_voters_percent?: Maybe<Scalars['numeric']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Election_Min_Fields = {
  __typename?: 'sequent_backend_results_election_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  elegible_census?: Maybe<Scalars['Int']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  total_voters?: Maybe<Scalars['Int']['output']>;
  total_voters_percent?: Maybe<Scalars['numeric']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Mutation_Response = {
  __typename?: 'sequent_backend_results_election_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Election>;
};

/** on_conflict condition type for table "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_On_Conflict = {
  constraint: Sequent_Backend_Results_Election_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Election_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_election". */
export type Sequent_Backend_Results_Election_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  elegible_census?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  total_voters?: InputMaybe<Order_By>;
  total_voters_percent?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_election */
export type Sequent_Backend_Results_Election_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Election_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_election" */
export enum Sequent_Backend_Results_Election_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalVoters = 'total_voters',
  /** column name */
  TotalVotersPercent = 'total_voters_percent'
}

/** input type for updating data in table "sequent_backend.results_election" */
export type Sequent_Backend_Results_Election_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_voters?: InputMaybe<Scalars['Int']['input']>;
  total_voters_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Results_Election_Stddev_Fields = {
  __typename?: 'sequent_backend_results_election_stddev_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Results_Election_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_results_election_stddev_pop_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Results_Election_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_results_election_stddev_samp_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** Streaming cursor of the table "sequent_backend_results_election" */
export type Sequent_Backend_Results_Election_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Election_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Election_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  elegible_census?: InputMaybe<Scalars['Int']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  total_voters?: InputMaybe<Scalars['Int']['input']>;
  total_voters_percent?: InputMaybe<Scalars['numeric']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Results_Election_Sum_Fields = {
  __typename?: 'sequent_backend_results_election_sum_fields';
  elegible_census?: Maybe<Scalars['Int']['output']>;
  total_voters?: Maybe<Scalars['Int']['output']>;
  total_voters_percent?: Maybe<Scalars['numeric']['output']>;
};

/** update columns of table "sequent_backend.results_election" */
export enum Sequent_Backend_Results_Election_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionId = 'election_id',
  /** column name */
  ElegibleCensus = 'elegible_census',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  Name = 'name',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  TotalVoters = 'total_voters',
  /** column name */
  TotalVotersPercent = 'total_voters_percent'
}

export type Sequent_Backend_Results_Election_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Election_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Election_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Election_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Election_Delete_Key_Input>;
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Results_Election_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Election_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Election_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Election_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Results_Election_Var_Pop_Fields = {
  __typename?: 'sequent_backend_results_election_var_pop_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Results_Election_Var_Samp_Fields = {
  __typename?: 'sequent_backend_results_election_var_samp_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Results_Election_Variance_Fields = {
  __typename?: 'sequent_backend_results_election_variance_fields';
  elegible_census?: Maybe<Scalars['Float']['output']>;
  total_voters?: Maybe<Scalars['Float']['output']>;
  total_voters_percent?: Maybe<Scalars['Float']['output']>;
};

/** columns and relationships of "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event = {
  __typename?: 'sequent_backend_results_event';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.results_event" */
export type Sequent_Backend_Results_EventAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_event" */
export type Sequent_Backend_Results_EventDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.results_event" */
export type Sequent_Backend_Results_EventLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Aggregate = {
  __typename?: 'sequent_backend_results_event_aggregate';
  aggregate?: Maybe<Sequent_Backend_Results_Event_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Results_Event>;
};

/** aggregate fields of "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Aggregate_Fields = {
  __typename?: 'sequent_backend_results_event_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Results_Event_Max_Fields>;
  min?: Maybe<Sequent_Backend_Results_Event_Min_Fields>;
};


/** aggregate fields of "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Results_Event_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Event_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.results_event". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Results_Event_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Results_Event_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Results_Event_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.results_event" */
export enum Sequent_Backend_Results_Event_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  ResultsEventPkey = 'results_event_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Results_Event_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Results_Event_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Results_Event_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Results_Event_Max_Fields = {
  __typename?: 'sequent_backend_results_event_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Results_Event_Min_Fields = {
  __typename?: 'sequent_backend_results_event_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Mutation_Response = {
  __typename?: 'sequent_backend_results_event_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Results_Event>;
};

/** on_conflict condition type for table "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_On_Conflict = {
  constraint: Sequent_Backend_Results_Event_Constraint;
  update_columns?: Array<Sequent_Backend_Results_Event_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.results_event". */
export type Sequent_Backend_Results_Event_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.results_event */
export type Sequent_Backend_Results_Event_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Results_Event_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.results_event" */
export enum Sequent_Backend_Results_Event_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.results_event" */
export type Sequent_Backend_Results_Event_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_results_event" */
export type Sequent_Backend_Results_Event_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Results_Event_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Results_Event_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.results_event" */
export enum Sequent_Backend_Results_Event_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Documents = 'documents',
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
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Results_Event_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Results_Event_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Results_Event_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Results_Event_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Results_Event_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Results_Event_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Results_Event_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Results_Event_Bool_Exp;
};

/** columns and relationships of "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event = {
  __typename?: 'sequent_backend_scheduled_event';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  archived_at?: Maybe<Scalars['timestamptz']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  cron_config?: Maybe<Scalars['jsonb']['output']>;
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
export type Sequent_Backend_Scheduled_EventCron_ConfigArgs = {
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
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.scheduled_event". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Scheduled_Event_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Scheduled_Event_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Scheduled_Event_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  archived_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_by?: InputMaybe<String_Comparison_Exp>;
  cron_config?: InputMaybe<Jsonb_Comparison_Exp>;
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
  cron_config?: InputMaybe<Array<Scalars['String']['input']>>;
  event_payload?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Scheduled_Event_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  cron_config?: InputMaybe<Scalars['Int']['input']>;
  event_payload?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Scheduled_Event_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['String']['input']>;
  event_payload?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.scheduled_event" */
export type Sequent_Backend_Scheduled_Event_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  archived_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
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
  archived_at?: Maybe<Scalars['timestamptz']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
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
  archived_at?: Maybe<Scalars['timestamptz']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
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
  archived_at?: InputMaybe<Order_By>;
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
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
  event_payload?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.scheduled_event" */
export enum Sequent_Backend_Scheduled_Event_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  ArchivedAt = 'archived_at',
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
  archived_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
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
  archived_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  cron_config?: InputMaybe<Scalars['jsonb']['input']>;
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
  ArchivedAt = 'archived_at',
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

/** columns and relationships of "sequent_backend.secret" */
export type Sequent_Backend_Secret = {
  __typename?: 'sequent_backend_secret';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id: Scalars['uuid']['output'];
  key: Scalars['String']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  tenant_id: Scalars['uuid']['output'];
  value: Scalars['bytea']['output'];
};


/** columns and relationships of "sequent_backend.secret" */
export type Sequent_Backend_SecretAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.secret" */
export type Sequent_Backend_SecretLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.secret" */
export type Sequent_Backend_Secret_Aggregate = {
  __typename?: 'sequent_backend_secret_aggregate';
  aggregate?: Maybe<Sequent_Backend_Secret_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Secret>;
};

/** aggregate fields of "sequent_backend.secret" */
export type Sequent_Backend_Secret_Aggregate_Fields = {
  __typename?: 'sequent_backend_secret_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Secret_Max_Fields>;
  min?: Maybe<Sequent_Backend_Secret_Min_Fields>;
};


/** aggregate fields of "sequent_backend.secret" */
export type Sequent_Backend_Secret_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Secret_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Secret_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.secret". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Secret_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Secret_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Secret_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  key?: InputMaybe<String_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  value?: InputMaybe<Bytea_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.secret" */
export enum Sequent_Backend_Secret_Constraint {
  /** unique or primary key constraint on columns "key" */
  SecretKeyKey = 'secret_key_key',
  /** unique or primary key constraint on columns "key", "id", "tenant_id" */
  SecretPkey = 'secret_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Secret_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Secret_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Secret_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.secret" */
export type Sequent_Backend_Secret_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  value?: InputMaybe<Scalars['bytea']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Secret_Max_Fields = {
  __typename?: 'sequent_backend_secret_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Secret_Min_Fields = {
  __typename?: 'sequent_backend_secret_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  key?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.secret" */
export type Sequent_Backend_Secret_Mutation_Response = {
  __typename?: 'sequent_backend_secret_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Secret>;
};

/** on_conflict condition type for table "sequent_backend.secret" */
export type Sequent_Backend_Secret_On_Conflict = {
  constraint: Sequent_Backend_Secret_Constraint;
  update_columns?: Array<Sequent_Backend_Secret_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.secret". */
export type Sequent_Backend_Secret_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  key?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  value?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.secret */
export type Sequent_Backend_Secret_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
  key: Scalars['String']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Secret_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.secret" */
export enum Sequent_Backend_Secret_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Key = 'key',
  /** column name */
  Labels = 'labels',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Value = 'value'
}

/** input type for updating data in table "sequent_backend.secret" */
export type Sequent_Backend_Secret_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  value?: InputMaybe<Scalars['bytea']['input']>;
};

/** Streaming cursor of the table "sequent_backend_secret" */
export type Sequent_Backend_Secret_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Secret_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Secret_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  key?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  value?: InputMaybe<Scalars['bytea']['input']>;
};

/** update columns of table "sequent_backend.secret" */
export enum Sequent_Backend_Secret_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Key = 'key',
  /** column name */
  Labels = 'labels',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Value = 'value'
}

export type Sequent_Backend_Secret_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Secret_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Secret_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Secret_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Secret_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Secret_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Secret_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Secret_Bool_Exp;
};

/** columns and relationships of "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material = {
  __typename?: 'sequent_backend_support_material';
  annotations: Scalars['jsonb']['output'];
  created_at: Scalars['timestamptz']['output'];
  data: Scalars['jsonb']['output'];
  document_id?: Maybe<Scalars['String']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  is_hidden?: Maybe<Scalars['Boolean']['output']>;
  kind: Scalars['String']['output'];
  labels: Scalars['jsonb']['output'];
  last_updated_at: Scalars['timestamptz']['output'];
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.support_material" */
export type Sequent_Backend_Support_MaterialAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.support_material" */
export type Sequent_Backend_Support_MaterialDataArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.support_material" */
export type Sequent_Backend_Support_MaterialLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Aggregate = {
  __typename?: 'sequent_backend_support_material_aggregate';
  aggregate?: Maybe<Sequent_Backend_Support_Material_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Support_Material>;
};

/** aggregate fields of "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Aggregate_Fields = {
  __typename?: 'sequent_backend_support_material_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Support_Material_Max_Fields>;
  min?: Maybe<Sequent_Backend_Support_Material_Min_Fields>;
};


/** aggregate fields of "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Support_Material_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Support_Material_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  data?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.support_material". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Support_Material_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Support_Material_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Support_Material_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  data?: InputMaybe<Jsonb_Comparison_Exp>;
  document_id?: InputMaybe<String_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_hidden?: InputMaybe<Boolean_Comparison_Exp>;
  kind?: InputMaybe<String_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.support_material" */
export enum Sequent_Backend_Support_Material_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  SupportMaterialPkey = 'support_material_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Support_Material_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  data?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Support_Material_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  data?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Support_Material_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  data?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  data?: InputMaybe<Scalars['jsonb']['input']>;
  document_id?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_hidden?: InputMaybe<Scalars['Boolean']['input']>;
  kind?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Support_Material_Max_Fields = {
  __typename?: 'sequent_backend_support_material_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  document_id?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  kind?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Support_Material_Min_Fields = {
  __typename?: 'sequent_backend_support_material_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  document_id?: Maybe<Scalars['String']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  kind?: Maybe<Scalars['String']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Mutation_Response = {
  __typename?: 'sequent_backend_support_material_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Support_Material>;
};

/** on_conflict condition type for table "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_On_Conflict = {
  constraint: Sequent_Backend_Support_Material_Constraint;
  update_columns?: Array<Sequent_Backend_Support_Material_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.support_material". */
export type Sequent_Backend_Support_Material_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  data?: InputMaybe<Order_By>;
  document_id?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_hidden?: InputMaybe<Order_By>;
  kind?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.support_material */
export type Sequent_Backend_Support_Material_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Support_Material_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  data?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.support_material" */
export enum Sequent_Backend_Support_Material_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Data = 'data',
  /** column name */
  DocumentId = 'document_id',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  IsHidden = 'is_hidden',
  /** column name */
  Kind = 'kind',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.support_material" */
export type Sequent_Backend_Support_Material_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  data?: InputMaybe<Scalars['jsonb']['input']>;
  document_id?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_hidden?: InputMaybe<Scalars['Boolean']['input']>;
  kind?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_support_material" */
export type Sequent_Backend_Support_Material_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Support_Material_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Support_Material_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  data?: InputMaybe<Scalars['jsonb']['input']>;
  document_id?: InputMaybe<Scalars['String']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_hidden?: InputMaybe<Scalars['Boolean']['input']>;
  kind?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.support_material" */
export enum Sequent_Backend_Support_Material_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  Data = 'data',
  /** column name */
  DocumentId = 'document_id',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  IsHidden = 'is_hidden',
  /** column name */
  Kind = 'kind',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Support_Material_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Support_Material_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Support_Material_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Support_Material_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Support_Material_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Support_Material_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Support_Material_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Support_Material_Bool_Exp;
};

/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session = {
  __typename?: 'sequent_backend_tally_session';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  configuration?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id: Scalars['uuid']['output'];
  is_execution_completed: Scalars['Boolean']['output'];
  keys_ceremony_id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  tally_type?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
  threshold: Scalars['Int']['output'];
};


/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_SessionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_SessionConfigurationArgs = {
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
  avg?: Maybe<Sequent_Backend_Tally_Session_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tally_Session_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tally_Session_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Tally_Session_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Tally_Session_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Tally_Session_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Tally_Session_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Tally_Session_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Tally_Session_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Tally_Session_Variance_Fields>;
};


/** aggregate fields of "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tally_Session_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Session_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Tally_Session_Avg_Fields = {
  __typename?: 'sequent_backend_tally_session_avg_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tally_session". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tally_Session_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tally_Session_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tally_Session_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tally_Session_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
  configuration?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_ids?: InputMaybe<Uuid_Array_Comparison_Exp>;
  execution_status?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  is_execution_completed?: InputMaybe<Boolean_Comparison_Exp>;
  keys_ceremony_id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  permission_label?: InputMaybe<String_Array_Comparison_Exp>;
  tally_type?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  threshold?: InputMaybe<Int_Comparison_Exp>;
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
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
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
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
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
  election_id?: InputMaybe<Scalars['uuid']['input']>;
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
  election_id?: Maybe<Scalars['uuid']['output']>;
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
  election_id?: Maybe<Scalars['uuid']['output']>;
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
  election_id?: InputMaybe<Order_By>;
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
  ElectionId = 'election_id',
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
  election_id?: InputMaybe<Scalars['uuid']['input']>;
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
  election_id?: InputMaybe<Scalars['uuid']['input']>;
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
  ElectionId = 'election_id',
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
  configuration?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Session_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  configuration?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Session_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  configuration?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_Execution = {
  __typename?: 'sequent_backend_tally_session_execution';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id: Scalars['Int']['output'];
  documents?: Maybe<Scalars['jsonb']['output']>;
  election_event_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  session_ids?: Maybe<Array<Scalars['Int']['output']>>;
  status?: Maybe<Scalars['jsonb']['output']>;
  tally_session_id: Scalars['uuid']['output'];
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionDocumentsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_session_execution" */
export type Sequent_Backend_Tally_Session_ExecutionStatusArgs = {
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
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
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
  documents?: InputMaybe<Jsonb_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  results_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  session_ids?: InputMaybe<Int_Array_Comparison_Exp>;
  status?: InputMaybe<Jsonb_Comparison_Exp>;
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
  documents?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  status?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Session_Execution_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  documents?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Session_Execution_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  documents?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  status?: InputMaybe<Scalars['String']['input']>;
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
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  session_ids?: InputMaybe<Array<Scalars['Int']['input']>>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
  tally_session_id?: InputMaybe<Scalars['uuid']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Session_Execution_Max_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id?: Maybe<Scalars['Int']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  session_ids?: Maybe<Array<Scalars['Int']['output']>>;
  tally_session_id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Session_Execution_Min_Fields = {
  __typename?: 'sequent_backend_tally_session_execution_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  current_message_id?: Maybe<Scalars['Int']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  results_event_id?: Maybe<Scalars['uuid']['output']>;
  session_ids?: Maybe<Array<Scalars['Int']['output']>>;
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
  documents?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  results_event_id?: InputMaybe<Order_By>;
  session_ids?: InputMaybe<Order_By>;
  status?: InputMaybe<Order_By>;
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
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
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
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  SessionIds = 'session_ids',
  /** column name */
  Status = 'status',
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
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  session_ids?: InputMaybe<Array<Scalars['Int']['input']>>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
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
  documents?: InputMaybe<Scalars['jsonb']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  results_event_id?: InputMaybe<Scalars['uuid']['input']>;
  session_ids?: InputMaybe<Array<Scalars['Int']['input']>>;
  status?: InputMaybe<Scalars['jsonb']['input']>;
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
  Documents = 'documents',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  ResultsEventId = 'results_event_id',
  /** column name */
  SessionIds = 'session_ids',
  /** column name */
  Status = 'status',
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

/** input type for incrementing numeric columns in table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Inc_Input = {
  threshold?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  tally_type?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Session_Max_Fields = {
  __typename?: 'sequent_backend_tally_session_max_fields';
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  keys_ceremony_id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  tally_type?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  threshold?: Maybe<Scalars['Int']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Session_Min_Fields = {
  __typename?: 'sequent_backend_tally_session_min_fields';
  area_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_ids?: Maybe<Array<Scalars['uuid']['output']>>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  keys_ceremony_id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  permission_label?: Maybe<Array<Scalars['String']['output']>>;
  tally_type?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  threshold?: Maybe<Scalars['Int']['output']>;
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
  configuration?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_ids?: InputMaybe<Order_By>;
  execution_status?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  is_execution_completed?: InputMaybe<Order_By>;
  keys_ceremony_id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  permission_label?: InputMaybe<Order_By>;
  tally_type?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  threshold?: InputMaybe<Order_By>;
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
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tally_session" */
export enum Sequent_Backend_Tally_Session_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaIds = 'area_ids',
  /** column name */
  Configuration = 'configuration',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  IsExecutionCompleted = 'is_execution_completed',
  /** column name */
  KeysCeremonyId = 'keys_ceremony_id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  TallyType = 'tally_type',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Threshold = 'threshold'
}

/** input type for updating data in table "sequent_backend.tally_session" */
export type Sequent_Backend_Tally_Session_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  tally_type?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Tally_Session_Stddev_Fields = {
  __typename?: 'sequent_backend_tally_session_stddev_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Tally_Session_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_stddev_pop_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Tally_Session_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_stddev_samp_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
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
  configuration?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_ids?: InputMaybe<Array<Scalars['uuid']['input']>>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_execution_completed?: InputMaybe<Scalars['Boolean']['input']>;
  keys_ceremony_id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  permission_label?: InputMaybe<Array<Scalars['String']['input']>>;
  tally_type?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  threshold?: InputMaybe<Scalars['Int']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Tally_Session_Sum_Fields = {
  __typename?: 'sequent_backend_tally_session_sum_fields';
  threshold?: Maybe<Scalars['Int']['output']>;
};

/** update columns of table "sequent_backend.tally_session" */
export enum Sequent_Backend_Tally_Session_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaIds = 'area_ids',
  /** column name */
  Configuration = 'configuration',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  ElectionIds = 'election_ids',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  IsExecutionCompleted = 'is_execution_completed',
  /** column name */
  KeysCeremonyId = 'keys_ceremony_id',
  /** column name */
  Labels = 'labels',
  /** column name */
  LastUpdatedAt = 'last_updated_at',
  /** column name */
  PermissionLabel = 'permission_label',
  /** column name */
  TallyType = 'tally_type',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Threshold = 'threshold'
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
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Tally_Session_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tally_Session_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tally_Session_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tally_Session_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Tally_Session_Var_Pop_Fields = {
  __typename?: 'sequent_backend_tally_session_var_pop_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Tally_Session_Var_Samp_Fields = {
  __typename?: 'sequent_backend_tally_session_var_samp_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Tally_Session_Variance_Fields = {
  __typename?: 'sequent_backend_tally_session_variance_fields';
  threshold?: Maybe<Scalars['Float']['output']>;
};

/** columns and relationships of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet = {
  __typename?: 'sequent_backend_tally_sheet';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  area_id: Scalars['uuid']['output'];
  channel?: Maybe<Scalars['String']['output']>;
  content?: Maybe<Scalars['jsonb']['output']>;
  contest_id: Scalars['uuid']['output'];
  created_at: Scalars['timestamptz']['output'];
  created_by_user_id: Scalars['String']['output'];
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id: Scalars['uuid']['output'];
  election_id: Scalars['uuid']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  last_updated_at: Scalars['timestamptz']['output'];
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  published_by_user_id?: Maybe<Scalars['String']['output']>;
  tenant_id: Scalars['uuid']['output'];
};


/** columns and relationships of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_SheetAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_SheetContentArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_SheetLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Aggregate = {
  __typename?: 'sequent_backend_tally_sheet_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tally_Sheet_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tally_Sheet>;
};

/** aggregate fields of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Aggregate_Fields = {
  __typename?: 'sequent_backend_tally_sheet_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tally_Sheet_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tally_Sheet_Min_Fields>;
};


/** aggregate fields of "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Sheet_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  content?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tally_sheet". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tally_Sheet_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  area_id?: InputMaybe<Uuid_Comparison_Exp>;
  channel?: InputMaybe<String_Comparison_Exp>;
  content?: InputMaybe<Jsonb_Comparison_Exp>;
  contest_id?: InputMaybe<Uuid_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_by_user_id?: InputMaybe<String_Comparison_Exp>;
  deleted_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  election_id?: InputMaybe<Uuid_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  last_updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  published_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  published_by_user_id?: InputMaybe<String_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tally_sheet" */
export enum Sequent_Backend_Tally_Sheet_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id", "election_event_id" */
  TallySheetPkey = 'tally_sheet_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tally_Sheet_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  content?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tally_Sheet_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  content?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tally_Sheet_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  content?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  channel?: InputMaybe<Scalars['String']['input']>;
  content?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_by_user_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tally_Sheet_Max_Fields = {
  __typename?: 'sequent_backend_tally_sheet_max_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  channel?: Maybe<Scalars['String']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by_user_id?: Maybe<Scalars['String']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  published_by_user_id?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tally_Sheet_Min_Fields = {
  __typename?: 'sequent_backend_tally_sheet_min_fields';
  area_id?: Maybe<Scalars['uuid']['output']>;
  channel?: Maybe<Scalars['String']['output']>;
  contest_id?: Maybe<Scalars['uuid']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by_user_id?: Maybe<Scalars['String']['output']>;
  deleted_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  election_id?: Maybe<Scalars['uuid']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  last_updated_at?: Maybe<Scalars['timestamptz']['output']>;
  published_at?: Maybe<Scalars['timestamptz']['output']>;
  published_by_user_id?: Maybe<Scalars['String']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
};

/** response of any mutation on the table "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Mutation_Response = {
  __typename?: 'sequent_backend_tally_sheet_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tally_Sheet>;
};

/** on_conflict condition type for table "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_On_Conflict = {
  constraint: Sequent_Backend_Tally_Sheet_Constraint;
  update_columns?: Array<Sequent_Backend_Tally_Sheet_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tally_sheet". */
export type Sequent_Backend_Tally_Sheet_Order_By = {
  annotations?: InputMaybe<Order_By>;
  area_id?: InputMaybe<Order_By>;
  channel?: InputMaybe<Order_By>;
  content?: InputMaybe<Order_By>;
  contest_id?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  created_by_user_id?: InputMaybe<Order_By>;
  deleted_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  election_id?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  published_at?: InputMaybe<Order_By>;
  published_by_user_id?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tally_sheet */
export type Sequent_Backend_Tally_Sheet_Pk_Columns_Input = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tally_Sheet_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  content?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tally_sheet" */
export enum Sequent_Backend_Tally_Sheet_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  Channel = 'channel',
  /** column name */
  Content = 'content',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedByUserId = 'created_by_user_id',
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
  PublishedAt = 'published_at',
  /** column name */
  PublishedByUserId = 'published_by_user_id',
  /** column name */
  TenantId = 'tenant_id'
}

/** input type for updating data in table "sequent_backend.tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  channel?: InputMaybe<Scalars['String']['input']>;
  content?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_by_user_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** Streaming cursor of the table "sequent_backend_tally_sheet" */
export type Sequent_Backend_Tally_Sheet_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tally_Sheet_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tally_Sheet_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  area_id?: InputMaybe<Scalars['uuid']['input']>;
  channel?: InputMaybe<Scalars['String']['input']>;
  content?: InputMaybe<Scalars['jsonb']['input']>;
  contest_id?: InputMaybe<Scalars['uuid']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by_user_id?: InputMaybe<Scalars['String']['input']>;
  deleted_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  election_id?: InputMaybe<Scalars['uuid']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  last_updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_at?: InputMaybe<Scalars['timestamptz']['input']>;
  published_by_user_id?: InputMaybe<Scalars['String']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
};

/** update columns of table "sequent_backend.tally_sheet" */
export enum Sequent_Backend_Tally_Sheet_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  AreaId = 'area_id',
  /** column name */
  Channel = 'channel',
  /** column name */
  Content = 'content',
  /** column name */
  ContestId = 'contest_id',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedByUserId = 'created_by_user_id',
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
  PublishedAt = 'published_at',
  /** column name */
  PublishedByUserId = 'published_by_user_id',
  /** column name */
  TenantId = 'tenant_id'
}

export type Sequent_Backend_Tally_Sheet_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tally_Sheet_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tally_Sheet_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tally_Sheet_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tally_Sheet_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tally_Sheet_Bool_Exp;
};

/** columns and relationships of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution = {
  __typename?: 'sequent_backend_tasks_execution';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  end_at?: Maybe<Scalars['timestamptz']['output']>;
  executed_by_user: Scalars['String']['output'];
  execution_status: Scalars['String']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  logs?: Maybe<Scalars['json']['output']>;
  name: Scalars['String']['output'];
  start_at: Scalars['timestamptz']['output'];
  /** An object relationship */
  tenant: Sequent_Backend_Tenant;
  tenant_id: Scalars['uuid']['output'];
  type: Scalars['String']['output'];
};


/** columns and relationships of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_ExecutionAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_ExecutionLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_ExecutionLogsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Aggregate = {
  __typename?: 'sequent_backend_tasks_execution_aggregate';
  aggregate?: Maybe<Sequent_Backend_Tasks_Execution_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Tasks_Execution>;
};

/** aggregate fields of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Aggregate_Fields = {
  __typename?: 'sequent_backend_tasks_execution_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tasks_Execution_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tasks_Execution_Min_Fields>;
};


/** aggregate fields of "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tasks_Execution_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.tasks_execution". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Tasks_Execution_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Bool_Exp>>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  election_event_id?: InputMaybe<Uuid_Comparison_Exp>;
  end_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  executed_by_user?: InputMaybe<String_Comparison_Exp>;
  execution_status?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  logs?: InputMaybe<Json_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  start_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  tenant?: InputMaybe<Sequent_Backend_Tenant_Bool_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.tasks_execution" */
export enum Sequent_Backend_Tasks_Execution_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id" */
  TasksExecutionPkey = 'tasks_execution_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Tasks_Execution_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tasks_Execution_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tasks_Execution_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  end_at?: InputMaybe<Scalars['timestamptz']['input']>;
  executed_by_user?: InputMaybe<Scalars['String']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  logs?: InputMaybe<Scalars['json']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  start_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant?: InputMaybe<Sequent_Backend_Tenant_Obj_Rel_Insert_Input>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tasks_Execution_Max_Fields = {
  __typename?: 'sequent_backend_tasks_execution_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  end_at?: Maybe<Scalars['timestamptz']['output']>;
  executed_by_user?: Maybe<Scalars['String']['output']>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  start_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tasks_Execution_Min_Fields = {
  __typename?: 'sequent_backend_tasks_execution_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  election_event_id?: Maybe<Scalars['uuid']['output']>;
  end_at?: Maybe<Scalars['timestamptz']['output']>;
  executed_by_user?: Maybe<Scalars['String']['output']>;
  execution_status?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  start_at?: Maybe<Scalars['timestamptz']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
};

/** response of any mutation on the table "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Mutation_Response = {
  __typename?: 'sequent_backend_tasks_execution_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Tasks_Execution>;
};

/** on_conflict condition type for table "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_On_Conflict = {
  constraint: Sequent_Backend_Tasks_Execution_Constraint;
  update_columns?: Array<Sequent_Backend_Tasks_Execution_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.tasks_execution". */
export type Sequent_Backend_Tasks_Execution_Order_By = {
  annotations?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  election_event_id?: InputMaybe<Order_By>;
  end_at?: InputMaybe<Order_By>;
  executed_by_user?: InputMaybe<Order_By>;
  execution_status?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  logs?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  start_at?: InputMaybe<Order_By>;
  tenant?: InputMaybe<Sequent_Backend_Tenant_Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.tasks_execution */
export type Sequent_Backend_Tasks_Execution_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Tasks_Execution_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.tasks_execution" */
export enum Sequent_Backend_Tasks_Execution_Select_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EndAt = 'end_at',
  /** column name */
  ExecutedByUser = 'executed_by_user',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Logs = 'logs',
  /** column name */
  Name = 'name',
  /** column name */
  StartAt = 'start_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

/** input type for updating data in table "sequent_backend.tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Set_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  end_at?: InputMaybe<Scalars['timestamptz']['input']>;
  executed_by_user?: InputMaybe<Scalars['String']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  logs?: InputMaybe<Scalars['json']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  start_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** Streaming cursor of the table "sequent_backend_tasks_execution" */
export type Sequent_Backend_Tasks_Execution_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Tasks_Execution_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Tasks_Execution_Stream_Cursor_Value_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  election_event_id?: InputMaybe<Scalars['uuid']['input']>;
  end_at?: InputMaybe<Scalars['timestamptz']['input']>;
  executed_by_user?: InputMaybe<Scalars['String']['input']>;
  execution_status?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  logs?: InputMaybe<Scalars['json']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  start_at?: InputMaybe<Scalars['timestamptz']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
};

/** update columns of table "sequent_backend.tasks_execution" */
export enum Sequent_Backend_Tasks_Execution_Update_Column {
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  ElectionEventId = 'election_event_id',
  /** column name */
  EndAt = 'end_at',
  /** column name */
  ExecutedByUser = 'executed_by_user',
  /** column name */
  ExecutionStatus = 'execution_status',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Logs = 'logs',
  /** column name */
  Name = 'name',
  /** column name */
  StartAt = 'start_at',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type'
}

export type Sequent_Backend_Tasks_Execution_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Tasks_Execution_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Tasks_Execution_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tasks_Execution_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tasks_Execution_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tasks_Execution_Bool_Exp;
};

/** columns and relationships of "sequent_backend.template" */
export type Sequent_Backend_Template = {
  __typename?: 'sequent_backend_template';
  alias?: Maybe<Scalars['String']['output']>;
  annotations?: Maybe<Scalars['jsonb']['output']>;
  communication_method: Scalars['String']['output'];
  created_at: Scalars['timestamptz']['output'];
  created_by: Scalars['String']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  template: Scalars['jsonb']['output'];
  tenant_id: Scalars['uuid']['output'];
  type: Scalars['String']['output'];
  updated_at: Scalars['timestamptz']['output'];
};


/** columns and relationships of "sequent_backend.template" */
export type Sequent_Backend_TemplateAnnotationsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.template" */
export type Sequent_Backend_TemplateLabelsArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};


/** columns and relationships of "sequent_backend.template" */
export type Sequent_Backend_TemplateTemplateArgs = {
  path?: InputMaybe<Scalars['String']['input']>;
};

/** aggregated selection of "sequent_backend.template" */
export type Sequent_Backend_Template_Aggregate = {
  __typename?: 'sequent_backend_template_aggregate';
  aggregate?: Maybe<Sequent_Backend_Template_Aggregate_Fields>;
  nodes: Array<Sequent_Backend_Template>;
};

/** aggregate fields of "sequent_backend.template" */
export type Sequent_Backend_Template_Aggregate_Fields = {
  __typename?: 'sequent_backend_template_aggregate_fields';
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Template_Max_Fields>;
  min?: Maybe<Sequent_Backend_Template_Min_Fields>;
};


/** aggregate fields of "sequent_backend.template" */
export type Sequent_Backend_Template_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Sequent_Backend_Template_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Template_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  template?: InputMaybe<Scalars['jsonb']['input']>;
};

/** Boolean expression to filter rows from the table "sequent_backend.template". All fields are combined with a logical 'AND'. */
export type Sequent_Backend_Template_Bool_Exp = {
  _and?: InputMaybe<Array<Sequent_Backend_Template_Bool_Exp>>;
  _not?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
  _or?: InputMaybe<Array<Sequent_Backend_Template_Bool_Exp>>;
  alias?: InputMaybe<String_Comparison_Exp>;
  annotations?: InputMaybe<Jsonb_Comparison_Exp>;
  communication_method?: InputMaybe<String_Comparison_Exp>;
  created_at?: InputMaybe<Timestamptz_Comparison_Exp>;
  created_by?: InputMaybe<String_Comparison_Exp>;
  id?: InputMaybe<Uuid_Comparison_Exp>;
  labels?: InputMaybe<Jsonb_Comparison_Exp>;
  template?: InputMaybe<Jsonb_Comparison_Exp>;
  tenant_id?: InputMaybe<Uuid_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
  updated_at?: InputMaybe<Timestamptz_Comparison_Exp>;
};

/** unique or primary key constraints on table "sequent_backend.template" */
export enum Sequent_Backend_Template_Constraint {
  /** unique or primary key constraint on columns "id", "tenant_id" */
  CommunicationTemplatePkey = 'communication_template_pkey'
}

/** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
export type Sequent_Backend_Template_Delete_At_Path_Input = {
  annotations?: InputMaybe<Array<Scalars['String']['input']>>;
  labels?: InputMaybe<Array<Scalars['String']['input']>>;
  template?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Template_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  template?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Template_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  template?: InputMaybe<Scalars['String']['input']>;
};

/** input type for inserting data into table "sequent_backend.template" */
export type Sequent_Backend_Template_Insert_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  communication_method?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  template?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Template_Max_Fields = {
  __typename?: 'sequent_backend_template_max_fields';
  alias?: Maybe<Scalars['String']['output']>;
  communication_method?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Template_Min_Fields = {
  __typename?: 'sequent_backend_template_min_fields';
  alias?: Maybe<Scalars['String']['output']>;
  communication_method?: Maybe<Scalars['String']['output']>;
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  created_by?: Maybe<Scalars['String']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  tenant_id?: Maybe<Scalars['uuid']['output']>;
  type?: Maybe<Scalars['String']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** response of any mutation on the table "sequent_backend.template" */
export type Sequent_Backend_Template_Mutation_Response = {
  __typename?: 'sequent_backend_template_mutation_response';
  /** number of rows affected by the mutation */
  affected_rows: Scalars['Int']['output'];
  /** data from the rows affected by the mutation */
  returning: Array<Sequent_Backend_Template>;
};

/** on_conflict condition type for table "sequent_backend.template" */
export type Sequent_Backend_Template_On_Conflict = {
  constraint: Sequent_Backend_Template_Constraint;
  update_columns?: Array<Sequent_Backend_Template_Update_Column>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
};

/** Ordering options when selecting data from "sequent_backend.template". */
export type Sequent_Backend_Template_Order_By = {
  alias?: InputMaybe<Order_By>;
  annotations?: InputMaybe<Order_By>;
  communication_method?: InputMaybe<Order_By>;
  created_at?: InputMaybe<Order_By>;
  created_by?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  labels?: InputMaybe<Order_By>;
  template?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
  updated_at?: InputMaybe<Order_By>;
};

/** primary key columns input for table: sequent_backend.template */
export type Sequent_Backend_Template_Pk_Columns_Input = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};

/** prepend existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Template_Prepend_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  template?: InputMaybe<Scalars['jsonb']['input']>;
};

/** select columns of table "sequent_backend.template" */
export enum Sequent_Backend_Template_Select_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CommunicationMethod = 'communication_method',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedBy = 'created_by',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Template = 'template',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type',
  /** column name */
  UpdatedAt = 'updated_at'
}

/** input type for updating data in table "sequent_backend.template" */
export type Sequent_Backend_Template_Set_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  communication_method?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  template?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** Streaming cursor of the table "sequent_backend_template" */
export type Sequent_Backend_Template_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Sequent_Backend_Template_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Sequent_Backend_Template_Stream_Cursor_Value_Input = {
  alias?: InputMaybe<Scalars['String']['input']>;
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  communication_method?: InputMaybe<Scalars['String']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  created_by?: InputMaybe<Scalars['String']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  template?: InputMaybe<Scalars['jsonb']['input']>;
  tenant_id?: InputMaybe<Scalars['uuid']['input']>;
  type?: InputMaybe<Scalars['String']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
};

/** update columns of table "sequent_backend.template" */
export enum Sequent_Backend_Template_Update_Column {
  /** column name */
  Alias = 'alias',
  /** column name */
  Annotations = 'annotations',
  /** column name */
  CommunicationMethod = 'communication_method',
  /** column name */
  CreatedAt = 'created_at',
  /** column name */
  CreatedBy = 'created_by',
  /** column name */
  Id = 'id',
  /** column name */
  Labels = 'labels',
  /** column name */
  Template = 'template',
  /** column name */
  TenantId = 'tenant_id',
  /** column name */
  Type = 'type',
  /** column name */
  UpdatedAt = 'updated_at'
}

export type Sequent_Backend_Template_Updates = {
  /** append existing jsonb value of filtered columns with new jsonb value */
  _append?: InputMaybe<Sequent_Backend_Template_Append_Input>;
  /** delete the field or element with specified path (for JSON arrays, negative integers count from the end) */
  _delete_at_path?: InputMaybe<Sequent_Backend_Template_Delete_At_Path_Input>;
  /** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
  _delete_elem?: InputMaybe<Sequent_Backend_Template_Delete_Elem_Input>;
  /** delete key/value pair or string element. key/value pairs are matched based on their key value */
  _delete_key?: InputMaybe<Sequent_Backend_Template_Delete_Key_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Template_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Template_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Template_Bool_Exp;
};

/** columns and relationships of "sequent_backend.tenant" */
export type Sequent_Backend_Tenant = {
  __typename?: 'sequent_backend_tenant';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  id: Scalars['uuid']['output'];
  is_active: Scalars['Boolean']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  settings?: Maybe<Scalars['jsonb']['output']>;
  slug: Scalars['String']['output'];
  test?: Maybe<Scalars['Int']['output']>;
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
export type Sequent_Backend_TenantSettingsArgs = {
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
  avg?: Maybe<Sequent_Backend_Tenant_Avg_Fields>;
  count: Scalars['Int']['output'];
  max?: Maybe<Sequent_Backend_Tenant_Max_Fields>;
  min?: Maybe<Sequent_Backend_Tenant_Min_Fields>;
  stddev?: Maybe<Sequent_Backend_Tenant_Stddev_Fields>;
  stddev_pop?: Maybe<Sequent_Backend_Tenant_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Sequent_Backend_Tenant_Stddev_Samp_Fields>;
  sum?: Maybe<Sequent_Backend_Tenant_Sum_Fields>;
  var_pop?: Maybe<Sequent_Backend_Tenant_Var_Pop_Fields>;
  var_samp?: Maybe<Sequent_Backend_Tenant_Var_Samp_Fields>;
  variance?: Maybe<Sequent_Backend_Tenant_Variance_Fields>;
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate avg on columns */
export type Sequent_Backend_Tenant_Avg_Fields = {
  __typename?: 'sequent_backend_tenant_avg_fields';
  test?: Maybe<Scalars['Float']['output']>;
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
  settings?: InputMaybe<Jsonb_Comparison_Exp>;
  slug?: InputMaybe<String_Comparison_Exp>;
  test?: InputMaybe<Int_Comparison_Exp>;
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
  settings?: InputMaybe<Array<Scalars['String']['input']>>;
  voting_channels?: InputMaybe<Array<Scalars['String']['input']>>;
};

/** delete the array element with specified index (negative integers count from the end). throws an error if top level container is not an array */
export type Sequent_Backend_Tenant_Delete_Elem_Input = {
  annotations?: InputMaybe<Scalars['Int']['input']>;
  labels?: InputMaybe<Scalars['Int']['input']>;
  settings?: InputMaybe<Scalars['Int']['input']>;
  voting_channels?: InputMaybe<Scalars['Int']['input']>;
};

/** delete key/value pair or string element. key/value pairs are matched based on their key value */
export type Sequent_Backend_Tenant_Delete_Key_Input = {
  annotations?: InputMaybe<Scalars['String']['input']>;
  labels?: InputMaybe<Scalars['String']['input']>;
  settings?: InputMaybe<Scalars['String']['input']>;
  voting_channels?: InputMaybe<Scalars['String']['input']>;
};

/** input type for incrementing numeric columns in table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Inc_Input = {
  test?: InputMaybe<Scalars['Int']['input']>;
};

/** input type for inserting data into table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Insert_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  created_at?: InputMaybe<Scalars['timestamptz']['input']>;
  id?: InputMaybe<Scalars['uuid']['input']>;
  is_active?: InputMaybe<Scalars['Boolean']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  test?: InputMaybe<Scalars['Int']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate max on columns */
export type Sequent_Backend_Tenant_Max_Fields = {
  __typename?: 'sequent_backend_tenant_max_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  slug?: Maybe<Scalars['String']['output']>;
  test?: Maybe<Scalars['Int']['output']>;
  updated_at?: Maybe<Scalars['timestamptz']['output']>;
};

/** aggregate min on columns */
export type Sequent_Backend_Tenant_Min_Fields = {
  __typename?: 'sequent_backend_tenant_min_fields';
  created_at?: Maybe<Scalars['timestamptz']['output']>;
  id?: Maybe<Scalars['uuid']['output']>;
  slug?: Maybe<Scalars['String']['output']>;
  test?: Maybe<Scalars['Int']['output']>;
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

/** input type for inserting object relation for remote table "sequent_backend.tenant" */
export type Sequent_Backend_Tenant_Obj_Rel_Insert_Input = {
  data: Sequent_Backend_Tenant_Insert_Input;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Tenant_On_Conflict>;
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
  settings?: InputMaybe<Order_By>;
  slug?: InputMaybe<Order_By>;
  test?: InputMaybe<Order_By>;
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
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
  Settings = 'settings',
  /** column name */
  Slug = 'slug',
  /** column name */
  Test = 'test',
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  test?: InputMaybe<Scalars['Int']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate stddev on columns */
export type Sequent_Backend_Tenant_Stddev_Fields = {
  __typename?: 'sequent_backend_tenant_stddev_fields';
  test?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_pop on columns */
export type Sequent_Backend_Tenant_Stddev_Pop_Fields = {
  __typename?: 'sequent_backend_tenant_stddev_pop_fields';
  test?: Maybe<Scalars['Float']['output']>;
};

/** aggregate stddev_samp on columns */
export type Sequent_Backend_Tenant_Stddev_Samp_Fields = {
  __typename?: 'sequent_backend_tenant_stddev_samp_fields';
  test?: Maybe<Scalars['Float']['output']>;
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
  settings?: InputMaybe<Scalars['jsonb']['input']>;
  slug?: InputMaybe<Scalars['String']['input']>;
  test?: InputMaybe<Scalars['Int']['input']>;
  updated_at?: InputMaybe<Scalars['timestamptz']['input']>;
  voting_channels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** aggregate sum on columns */
export type Sequent_Backend_Tenant_Sum_Fields = {
  __typename?: 'sequent_backend_tenant_sum_fields';
  test?: Maybe<Scalars['Int']['output']>;
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
  Settings = 'settings',
  /** column name */
  Slug = 'slug',
  /** column name */
  Test = 'test',
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
  /** increments the numeric columns with given value of the filtered values */
  _inc?: InputMaybe<Sequent_Backend_Tenant_Inc_Input>;
  /** prepend existing jsonb value of filtered columns with new jsonb value */
  _prepend?: InputMaybe<Sequent_Backend_Tenant_Prepend_Input>;
  /** sets the columns of the filtered rows to the given values */
  _set?: InputMaybe<Sequent_Backend_Tenant_Set_Input>;
  /** filter the rows which have to be updated */
  where: Sequent_Backend_Tenant_Bool_Exp;
};

/** aggregate var_pop on columns */
export type Sequent_Backend_Tenant_Var_Pop_Fields = {
  __typename?: 'sequent_backend_tenant_var_pop_fields';
  test?: Maybe<Scalars['Float']['output']>;
};

/** aggregate var_samp on columns */
export type Sequent_Backend_Tenant_Var_Samp_Fields = {
  __typename?: 'sequent_backend_tenant_var_samp_fields';
  test?: Maybe<Scalars['Float']['output']>;
};

/** aggregate variance on columns */
export type Sequent_Backend_Tenant_Variance_Fields = {
  __typename?: 'sequent_backend_tenant_variance_fields';
  test?: Maybe<Scalars['Float']['output']>;
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

export type Sequent_Backend_Trustee_Aggregate_Bool_Exp = {
  count?: InputMaybe<Sequent_Backend_Trustee_Aggregate_Bool_Exp_Count>;
};

export type Sequent_Backend_Trustee_Aggregate_Bool_Exp_Count = {
  arguments?: InputMaybe<Array<Sequent_Backend_Trustee_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']['input']>;
  filter?: InputMaybe<Sequent_Backend_Trustee_Bool_Exp>;
  predicate: Int_Comparison_Exp;
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

/** order by aggregate values of table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Aggregate_Order_By = {
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Sequent_Backend_Trustee_Max_Order_By>;
  min?: InputMaybe<Sequent_Backend_Trustee_Min_Order_By>;
};

/** append existing jsonb value of filtered columns with new jsonb value */
export type Sequent_Backend_Trustee_Append_Input = {
  annotations?: InputMaybe<Scalars['jsonb']['input']>;
  labels?: InputMaybe<Scalars['jsonb']['input']>;
};

/** input type for inserting array relation for remote table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Arr_Rel_Insert_Input = {
  data: Array<Sequent_Backend_Trustee_Insert_Input>;
  /** upsert condition */
  on_conflict?: InputMaybe<Sequent_Backend_Trustee_On_Conflict>;
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

/** order by max() on columns of table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Max_Order_By = {
  created_at?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  public_key?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
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

/** order by min() on columns of table "sequent_backend.trustee" */
export type Sequent_Backend_Trustee_Min_Order_By = {
  created_at?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  last_updated_at?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  public_key?: InputMaybe<Order_By>;
  tenant_id?: InputMaybe<Order_By>;
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
  /** fetch data from the table: "sequent_backend.applications" */
  sequent_backend_applications: Array<Sequent_Backend_Applications>;
  /** fetch aggregated fields from the table: "sequent_backend.applications" */
  sequent_backend_applications_aggregate: Sequent_Backend_Applications_Aggregate;
  /** fetch data from the table: "sequent_backend.applications" using primary key columns */
  sequent_backend_applications_by_pk?: Maybe<Sequent_Backend_Applications>;
  /** fetch data from the table in a streaming manner: "sequent_backend.applications" */
  sequent_backend_applications_stream: Array<Sequent_Backend_Applications>;
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
  /** fetch data from the table: "sequent_backend.ballot_publication" */
  sequent_backend_ballot_publication: Array<Sequent_Backend_Ballot_Publication>;
  /** fetch aggregated fields from the table: "sequent_backend.ballot_publication" */
  sequent_backend_ballot_publication_aggregate: Sequent_Backend_Ballot_Publication_Aggregate;
  /** fetch data from the table: "sequent_backend.ballot_publication" using primary key columns */
  sequent_backend_ballot_publication_by_pk?: Maybe<Sequent_Backend_Ballot_Publication>;
  /** fetch data from the table in a streaming manner: "sequent_backend.ballot_publication" */
  sequent_backend_ballot_publication_stream: Array<Sequent_Backend_Ballot_Publication>;
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
  /** fetch data from the table: "sequent_backend.notification" */
  sequent_backend_notification: Array<Sequent_Backend_Notification>;
  /** fetch aggregated fields from the table: "sequent_backend.notification" */
  sequent_backend_notification_aggregate: Sequent_Backend_Notification_Aggregate;
  /** fetch data from the table: "sequent_backend.notification" using primary key columns */
  sequent_backend_notification_by_pk?: Maybe<Sequent_Backend_Notification>;
  /** fetch data from the table in a streaming manner: "sequent_backend.notification" */
  sequent_backend_notification_stream: Array<Sequent_Backend_Notification>;
  /** fetch data from the table: "sequent_backend.report" */
  sequent_backend_report: Array<Sequent_Backend_Report>;
  /** fetch aggregated fields from the table: "sequent_backend.report" */
  sequent_backend_report_aggregate: Sequent_Backend_Report_Aggregate;
  /** fetch data from the table: "sequent_backend.report" using primary key columns */
  sequent_backend_report_by_pk?: Maybe<Sequent_Backend_Report>;
  /** fetch data from the table in a streaming manner: "sequent_backend.report" */
  sequent_backend_report_stream: Array<Sequent_Backend_Report>;
  /** fetch data from the table: "sequent_backend.results_area_contest" */
  sequent_backend_results_area_contest: Array<Sequent_Backend_Results_Area_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.results_area_contest" */
  sequent_backend_results_area_contest_aggregate: Sequent_Backend_Results_Area_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.results_area_contest" using primary key columns */
  sequent_backend_results_area_contest_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest>;
  /** fetch data from the table: "sequent_backend.results_area_contest_candidate" */
  sequent_backend_results_area_contest_candidate: Array<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.results_area_contest_candidate" */
  sequent_backend_results_area_contest_candidate_aggregate: Sequent_Backend_Results_Area_Contest_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.results_area_contest_candidate" using primary key columns */
  sequent_backend_results_area_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_area_contest_candidate" */
  sequent_backend_results_area_contest_candidate_stream: Array<Sequent_Backend_Results_Area_Contest_Candidate>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_area_contest" */
  sequent_backend_results_area_contest_stream: Array<Sequent_Backend_Results_Area_Contest>;
  /** fetch data from the table: "sequent_backend.results_contest" */
  sequent_backend_results_contest: Array<Sequent_Backend_Results_Contest>;
  /** fetch aggregated fields from the table: "sequent_backend.results_contest" */
  sequent_backend_results_contest_aggregate: Sequent_Backend_Results_Contest_Aggregate;
  /** fetch data from the table: "sequent_backend.results_contest" using primary key columns */
  sequent_backend_results_contest_by_pk?: Maybe<Sequent_Backend_Results_Contest>;
  /** fetch data from the table: "sequent_backend.results_contest_candidate" */
  sequent_backend_results_contest_candidate: Array<Sequent_Backend_Results_Contest_Candidate>;
  /** fetch aggregated fields from the table: "sequent_backend.results_contest_candidate" */
  sequent_backend_results_contest_candidate_aggregate: Sequent_Backend_Results_Contest_Candidate_Aggregate;
  /** fetch data from the table: "sequent_backend.results_contest_candidate" using primary key columns */
  sequent_backend_results_contest_candidate_by_pk?: Maybe<Sequent_Backend_Results_Contest_Candidate>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_contest_candidate" */
  sequent_backend_results_contest_candidate_stream: Array<Sequent_Backend_Results_Contest_Candidate>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_contest" */
  sequent_backend_results_contest_stream: Array<Sequent_Backend_Results_Contest>;
  /** fetch data from the table: "sequent_backend.results_election" */
  sequent_backend_results_election: Array<Sequent_Backend_Results_Election>;
  /** fetch aggregated fields from the table: "sequent_backend.results_election" */
  sequent_backend_results_election_aggregate: Sequent_Backend_Results_Election_Aggregate;
  /** fetch data from the table: "sequent_backend.results_election_area" */
  sequent_backend_results_election_area: Array<Sequent_Backend_Results_Election_Area>;
  /** fetch aggregated fields from the table: "sequent_backend.results_election_area" */
  sequent_backend_results_election_area_aggregate: Sequent_Backend_Results_Election_Area_Aggregate;
  /** fetch data from the table: "sequent_backend.results_election_area" using primary key columns */
  sequent_backend_results_election_area_by_pk?: Maybe<Sequent_Backend_Results_Election_Area>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_election_area" */
  sequent_backend_results_election_area_stream: Array<Sequent_Backend_Results_Election_Area>;
  /** fetch data from the table: "sequent_backend.results_election" using primary key columns */
  sequent_backend_results_election_by_pk?: Maybe<Sequent_Backend_Results_Election>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_election" */
  sequent_backend_results_election_stream: Array<Sequent_Backend_Results_Election>;
  /** fetch data from the table: "sequent_backend.results_event" */
  sequent_backend_results_event: Array<Sequent_Backend_Results_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.results_event" */
  sequent_backend_results_event_aggregate: Sequent_Backend_Results_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.results_event" using primary key columns */
  sequent_backend_results_event_by_pk?: Maybe<Sequent_Backend_Results_Event>;
  /** fetch data from the table in a streaming manner: "sequent_backend.results_event" */
  sequent_backend_results_event_stream: Array<Sequent_Backend_Results_Event>;
  /** fetch data from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch aggregated fields from the table: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_aggregate: Sequent_Backend_Scheduled_Event_Aggregate;
  /** fetch data from the table: "sequent_backend.scheduled_event" using primary key columns */
  sequent_backend_scheduled_event_by_pk?: Maybe<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table in a streaming manner: "sequent_backend.scheduled_event" */
  sequent_backend_scheduled_event_stream: Array<Sequent_Backend_Scheduled_Event>;
  /** fetch data from the table: "sequent_backend.secret" */
  sequent_backend_secret: Array<Sequent_Backend_Secret>;
  /** fetch aggregated fields from the table: "sequent_backend.secret" */
  sequent_backend_secret_aggregate: Sequent_Backend_Secret_Aggregate;
  /** fetch data from the table: "sequent_backend.secret" using primary key columns */
  sequent_backend_secret_by_pk?: Maybe<Sequent_Backend_Secret>;
  /** fetch data from the table in a streaming manner: "sequent_backend.secret" */
  sequent_backend_secret_stream: Array<Sequent_Backend_Secret>;
  /** fetch data from the table: "sequent_backend.support_material" */
  sequent_backend_support_material: Array<Sequent_Backend_Support_Material>;
  /** fetch aggregated fields from the table: "sequent_backend.support_material" */
  sequent_backend_support_material_aggregate: Sequent_Backend_Support_Material_Aggregate;
  /** fetch data from the table: "sequent_backend.support_material" using primary key columns */
  sequent_backend_support_material_by_pk?: Maybe<Sequent_Backend_Support_Material>;
  /** fetch data from the table in a streaming manner: "sequent_backend.support_material" */
  sequent_backend_support_material_stream: Array<Sequent_Backend_Support_Material>;
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
  /** fetch data from the table: "sequent_backend.tally_sheet" */
  sequent_backend_tally_sheet: Array<Sequent_Backend_Tally_Sheet>;
  /** fetch aggregated fields from the table: "sequent_backend.tally_sheet" */
  sequent_backend_tally_sheet_aggregate: Sequent_Backend_Tally_Sheet_Aggregate;
  /** fetch data from the table: "sequent_backend.tally_sheet" using primary key columns */
  sequent_backend_tally_sheet_by_pk?: Maybe<Sequent_Backend_Tally_Sheet>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tally_sheet" */
  sequent_backend_tally_sheet_stream: Array<Sequent_Backend_Tally_Sheet>;
  /** fetch data from the table: "sequent_backend.tasks_execution" */
  sequent_backend_tasks_execution: Array<Sequent_Backend_Tasks_Execution>;
  /** fetch aggregated fields from the table: "sequent_backend.tasks_execution" */
  sequent_backend_tasks_execution_aggregate: Sequent_Backend_Tasks_Execution_Aggregate;
  /** fetch data from the table: "sequent_backend.tasks_execution" using primary key columns */
  sequent_backend_tasks_execution_by_pk?: Maybe<Sequent_Backend_Tasks_Execution>;
  /** fetch data from the table in a streaming manner: "sequent_backend.tasks_execution" */
  sequent_backend_tasks_execution_stream: Array<Sequent_Backend_Tasks_Execution>;
  /** fetch data from the table: "sequent_backend.template" */
  sequent_backend_template: Array<Sequent_Backend_Template>;
  /** fetch aggregated fields from the table: "sequent_backend.template" */
  sequent_backend_template_aggregate: Sequent_Backend_Template_Aggregate;
  /** fetch data from the table: "sequent_backend.template" using primary key columns */
  sequent_backend_template_by_pk?: Maybe<Sequent_Backend_Template>;
  /** fetch data from the table in a streaming manner: "sequent_backend.template" */
  sequent_backend_template_stream: Array<Sequent_Backend_Template>;
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


export type Subscription_RootSequent_Backend_ApplicationsArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Applications_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Applications_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Applications_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Applications_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Applications_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Applications_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Applications_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Applications_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Applications_Bool_Exp>;
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


export type Subscription_RootSequent_Backend_Ballot_PublicationArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Ballot_Publication_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Ballot_Publication_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Ballot_Publication_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Ballot_Publication_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Ballot_Publication_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Ballot_Publication_Bool_Exp>;
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


export type Subscription_RootSequent_Backend_NotificationArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Notification_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Notification_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Notification_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Notification_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Notification_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Notification_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Notification_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Notification_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Notification_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_ReportArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Report_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Report_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Report_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Report_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Report_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Report_By_PkArgs = {
  id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Report_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Report_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Report_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Area_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_Candidate_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Area_Contest_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Area_Contest_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Area_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_ContestArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Contest_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Contest_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Contest_CandidateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Contest_Candidate_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Contest_Candidate_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Contest_Candidate_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Contest_Candidate_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Contest_Candidate_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Candidate_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Contest_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Contest_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Contest_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_ElectionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Election_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Election_AreaArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Election_Area_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Election_Area_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Election_Area_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Election_Area_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Election_Area_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Area_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Election_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  results_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Election_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Election_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Election_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_EventArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Event_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Results_Event_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Results_Event_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Results_Event_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Results_Event_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Results_Event_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Results_Event_Bool_Exp>;
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


export type Subscription_RootSequent_Backend_SecretArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Secret_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Secret_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Secret_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Secret_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Secret_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Secret_By_PkArgs = {
  id: Scalars['uuid']['input'];
  key: Scalars['String']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Secret_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Secret_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Secret_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Support_MaterialArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Support_Material_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Support_Material_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Support_Material_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Support_Material_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Support_Material_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Support_Material_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Support_Material_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Support_Material_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Support_Material_Bool_Exp>;
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


export type Subscription_RootSequent_Backend_Tally_SheetArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Sheet_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tally_Sheet_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tally_Sheet_By_PkArgs = {
  election_event_id: Scalars['uuid']['input'];
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tally_Sheet_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tally_Sheet_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tally_Sheet_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tasks_ExecutionArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tasks_Execution_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Tasks_Execution_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Tasks_Execution_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Tasks_Execution_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Tasks_Execution_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Tasks_Execution_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_TemplateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Template_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Template_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Template_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Sequent_Backend_Template_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  order_by?: InputMaybe<Array<Sequent_Backend_Template_Order_By>>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
};


export type Subscription_RootSequent_Backend_Template_By_PkArgs = {
  id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
};


export type Subscription_RootSequent_Backend_Template_StreamArgs = {
  batch_size: Scalars['Int']['input'];
  cursor: Array<InputMaybe<Sequent_Backend_Template_Stream_Cursor_Input>>;
  where?: InputMaybe<Sequent_Backend_Template_Bool_Exp>;
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

export type TaskOutput = {
  __typename?: 'taskOutput';
  task_execution: Tasks_Execution_Type;
};

export type Tasks_Execution_Type = {
  __typename?: 'tasks_execution_type';
  annotations?: Maybe<Scalars['jsonb']['output']>;
  created_at: Scalars['timestamptz']['output'];
  election_event_id: Scalars['uuid']['output'];
  end_at?: Maybe<Scalars['timestamptz']['output']>;
  executed_by_user: Scalars['String']['output'];
  execution_status: Scalars['String']['output'];
  id: Scalars['uuid']['output'];
  labels?: Maybe<Scalars['jsonb']['output']>;
  logs?: Maybe<Scalars['json']['output']>;
  name: Scalars['String']['output'];
  start_at: Scalars['timestamptz']['output'];
  tenant_id: Scalars['uuid']['output'];
  type: Scalars['String']['output'];
};

export type TemplateOutput = {
  __typename?: 'templateOutput';
  document_id: Scalars['String']['output'];
  error_msg?: Maybe<Scalars['String']['output']>;
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

export type CreateBallotReceiptMutationVariables = Exact<{
  ballot_id: Scalars['String']['input'];
  ballot_tracker_url: Scalars['String']['input'];
  election_event_id: Scalars['uuid']['input'];
  tenant_id: Scalars['uuid']['input'];
  election_id: Scalars['uuid']['input'];
}>;


export type CreateBallotReceiptMutation = { __typename?: 'mutation_root', create_ballot_receipt?: { __typename?: 'createBallotReceiptOutput', id: any, ballot_id?: string | null, status?: string | null } | null };

export type FetchDocumentQueryVariables = Exact<{
  electionEventId: Scalars['String']['input'];
  documentId: Scalars['String']['input'];
}>;


export type FetchDocumentQuery = { __typename?: 'query_root', fetchDocument?: { __typename?: 'FetchDocumentOutput', url: string } | null };

export type GetBallotStylesQueryVariables = Exact<{ [key: string]: never; }>;


export type GetBallotStylesQuery = { __typename?: 'query_root', sequent_backend_ballot_style: Array<{ __typename?: 'sequent_backend_ballot_style', id: any, election_id: any, election_event_id: any, status?: string | null, tenant_id: any, ballot_eml?: string | null, ballot_signature?: any | null, created_at?: any | null, area_id?: any | null, annotations?: any | null, labels?: any | null, last_updated_at?: any | null, deleted_at?: any | null }> };

export type GetCastVoteQueryVariables = Exact<{
  tenantId?: InputMaybe<Scalars['uuid']['input']>;
  electionEventId?: InputMaybe<Scalars['uuid']['input']>;
  electionId?: InputMaybe<Scalars['uuid']['input']>;
  ballotId: Scalars['String']['input'];
}>;


export type GetCastVoteQuery = { __typename?: 'query_root', sequent_backend_cast_vote: Array<{ __typename?: 'sequent_backend_cast_vote', ballot_id?: string | null, content?: string | null }> };

export type GetCastVotesQueryVariables = Exact<{ [key: string]: never; }>;


export type GetCastVotesQuery = { __typename?: 'query_root', sequent_backend_cast_vote: Array<{ __typename?: 'sequent_backend_cast_vote', id: any, tenant_id: any, election_id?: any | null, area_id?: any | null, created_at?: any | null, last_updated_at?: any | null, labels?: any | null, annotations?: any | null, content?: string | null, cast_ballot_signature?: any | null, voter_id_string?: string | null, election_event_id: any }> };

export type GetDocumentQueryVariables = Exact<{
  ids?: InputMaybe<Array<Scalars['uuid']['input']> | Scalars['uuid']['input']>;
  tenantId?: InputMaybe<Scalars['uuid']['input']>;
  electionEventId?: InputMaybe<Scalars['uuid']['input']>;
}>;


export type GetDocumentQuery = { __typename?: 'query_root', sequent_backend_document: Array<{ __typename?: 'sequent_backend_document', id: any, tenant_id?: any | null, election_event_id?: any | null, name?: string | null, media_type?: string | null, size?: any | null, labels?: any | null, annotations?: any | null, created_at?: any | null, last_updated_at?: any | null, is_public?: boolean | null }> };

export type GetElectionEventQueryVariables = Exact<{
  electionEventId: Scalars['uuid']['input'];
  tenantId: Scalars['uuid']['input'];
}>;


export type GetElectionEventQuery = { __typename?: 'query_root', sequent_backend_election_event: Array<{ __typename?: 'sequent_backend_election_event', id: any, presentation?: any | null, status?: any | null, description?: string | null }> };

export type GetElectionsQueryVariables = Exact<{
  electionIds: Array<Scalars['uuid']['input']> | Scalars['uuid']['input'];
}>;


export type GetElectionsQuery = { __typename?: 'query_root', sequent_backend_election: Array<{ __typename?: 'sequent_backend_election', annotations?: any | null, created_at?: any | null, description?: string | null, election_event_id: any, eml?: string | null, id: any, is_consolidated_ballot_encoding?: boolean | null, labels?: any | null, last_updated_at?: any | null, name: string, num_allowed_revotes?: number | null, presentation?: any | null, spoil_ballot_option?: boolean | null, status?: any | null, tenant_id: any, alias?: string | null }> };

export type GetSupportMaterialsQueryVariables = Exact<{
  electionEventId: Scalars['uuid']['input'];
  tenantId: Scalars['uuid']['input'];
}>;


export type GetSupportMaterialsQuery = { __typename?: 'query_root', sequent_backend_support_material: Array<{ __typename?: 'sequent_backend_support_material', data: any, document_id?: string | null, id: any, annotations: any, created_at: any, election_event_id: any, kind: string, labels: any, last_updated_at: any, tenant_id: any }> };

export type InsertCastVoteMutationVariables = Exact<{
  electionId: Scalars['uuid']['input'];
  ballotId: Scalars['String']['input'];
  content: Scalars['String']['input'];
}>;


export type InsertCastVoteMutation = { __typename?: 'mutation_root', insert_cast_vote?: { __typename?: 'InsertCastVoteOutput', id: any, ballot_id?: string | null, election_id: any, election_event_id: any, tenant_id: any, area_id: any, created_at?: any | null, last_updated_at?: any | null, labels?: any | null, annotations?: any | null, content?: string | null, cast_ballot_signature: any, voter_id_string?: string | null } | null };

export type ListCastVoteMessagesQueryVariables = Exact<{
  tenantId: Scalars['String']['input'];
  electionEventId: Scalars['String']['input'];
  electionId?: InputMaybe<Scalars['String']['input']>;
  ballotId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
  orderBy?: InputMaybe<ElectoralLogOrderBy>;
}>;


export type ListCastVoteMessagesQuery = { __typename?: 'query_root', list_cast_vote_messages?: { __typename?: 'ListCastVoteMessagesOutput', total: number, list: Array<{ __typename?: 'CastVoteEntry', statement_timestamp: number, statement_kind: string, ballot_id: string, username: string } | null> } | null };


export const CreateBallotReceiptDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"createBallotReceipt"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ballot_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ballot_tracker_url"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"election_event_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"election_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"create_ballot_receipt"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ballot_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ballot_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"ballot_tracker_url"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ballot_tracker_url"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"election_event_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenant_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"election_id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_id"}},{"kind":"Field","name":{"kind":"Name","value":"status"}}]}}]}}]} as unknown as DocumentNode<CreateBallotReceiptMutation, CreateBallotReceiptMutationVariables>;
export const FetchDocumentDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"FetchDocument"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"documentId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fetchDocument"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"Argument","name":{"kind":"Name","value":"document_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"documentId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}}]}}]}}]} as unknown as DocumentNode<FetchDocumentQuery, FetchDocumentQueryVariables>;
export const GetBallotStylesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetBallotStyles"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_ballot_style"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"deleted_at"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_is_null"},"value":{"kind":"BooleanValue","value":true}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_eml"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_signature"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"area_id"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"deleted_at"}}]}}]}}]} as unknown as DocumentNode<GetBallotStylesQuery, GetBallotStylesQueryVariables>;
export const GetCastVoteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetCastVote"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_cast_vote"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"ballot_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ballot_id"}},{"kind":"Field","name":{"kind":"Name","value":"content"}}]}}]}}]} as unknown as DocumentNode<GetCastVoteQuery, GetCastVoteQueryVariables>;
export const GetCastVotesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetCastVotes"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_cast_vote"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"area_id"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"content"}},{"kind":"Field","name":{"kind":"Name","value":"cast_ballot_signature"}},{"kind":"Field","name":{"kind":"Name","value":"voter_id_string"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}}]}}]}}]} as unknown as DocumentNode<GetCastVotesQuery, GetCastVotesQueryVariables>;
export const GetDocumentDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetDocument"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ids"}},"type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_document"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_in"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ids"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"media_type"}},{"kind":"Field","name":{"kind":"Name","value":"size"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"is_public"}}]}}]}}]} as unknown as DocumentNode<GetDocumentQuery, GetDocumentQueryVariables>;
export const GetElectionEventDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetElectionEvent"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_election_event"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"presentation"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"description"}}]}}]}}]} as unknown as DocumentNode<GetElectionEventQuery, GetElectionEventQueryVariables>;
export const GetElectionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetElections"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionIds"}},"type":{"kind":"NonNullType","type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_election"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_in"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionIds"}}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"eml"}},{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"is_consolidated_ballot_encoding"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"num_allowed_revotes"}},{"kind":"Field","name":{"kind":"Name","value":"presentation"}},{"kind":"Field","name":{"kind":"Name","value":"spoil_ballot_option"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"alias"}}]}}]}}]} as unknown as DocumentNode<GetElectionsQuery, GetElectionsQueryVariables>;
export const GetSupportMaterialsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetSupportMaterials"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sequent_backend_support_material"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"where"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_and"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"is_hidden"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"BooleanValue","value":false}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}}]}},{"kind":"ObjectField","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"_eq"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}}]}}]}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"data"}},{"kind":"Field","name":{"kind":"Name","value":"document_id"}},{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}}]}}]}}]} as unknown as DocumentNode<GetSupportMaterialsQuery, GetSupportMaterialsQueryVariables>;
export const InsertCastVoteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"InsertCastVote"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"uuid"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"content"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"insert_cast_vote"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"election_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}}},{"kind":"Argument","name":{"kind":"Name","value":"ballot_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}}},{"kind":"Argument","name":{"kind":"Name","value":"content"},"value":{"kind":"Variable","name":{"kind":"Name","value":"content"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}},{"kind":"Field","name":{"kind":"Name","value":"tenant_id"}},{"kind":"Field","name":{"kind":"Name","value":"election_id"}},{"kind":"Field","name":{"kind":"Name","value":"area_id"}},{"kind":"Field","name":{"kind":"Name","value":"created_at"}},{"kind":"Field","name":{"kind":"Name","value":"last_updated_at"}},{"kind":"Field","name":{"kind":"Name","value":"labels"}},{"kind":"Field","name":{"kind":"Name","value":"annotations"}},{"kind":"Field","name":{"kind":"Name","value":"content"}},{"kind":"Field","name":{"kind":"Name","value":"cast_ballot_signature"}},{"kind":"Field","name":{"kind":"Name","value":"voter_id_string"}},{"kind":"Field","name":{"kind":"Name","value":"election_event_id"}}]}}]}}]} as unknown as DocumentNode<InsertCastVoteMutation, InsertCastVoteMutationVariables>;
export const ListCastVoteMessagesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"listCastVoteMessages"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"limit"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"offset"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"orderBy"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"ElectoralLogOrderBy"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"list_cast_vote_messages"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"tenant_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"tenantId"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_event_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionEventId"}}},{"kind":"Argument","name":{"kind":"Name","value":"election_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"electionId"}}},{"kind":"Argument","name":{"kind":"Name","value":"ballot_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ballotId"}}},{"kind":"Argument","name":{"kind":"Name","value":"limit"},"value":{"kind":"Variable","name":{"kind":"Name","value":"limit"}}},{"kind":"Argument","name":{"kind":"Name","value":"offset"},"value":{"kind":"Variable","name":{"kind":"Name","value":"offset"}}},{"kind":"Argument","name":{"kind":"Name","value":"order_by"},"value":{"kind":"Variable","name":{"kind":"Name","value":"orderBy"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"list"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"statement_timestamp"}},{"kind":"Field","name":{"kind":"Name","value":"statement_kind"}},{"kind":"Field","name":{"kind":"Name","value":"ballot_id"}},{"kind":"Field","name":{"kind":"Name","value":"username"}}]}},{"kind":"Field","name":{"kind":"Name","value":"total"}}]}}]}}]} as unknown as DocumentNode<ListCastVoteMessagesQuery, ListCastVoteMessagesQueryVariables>;