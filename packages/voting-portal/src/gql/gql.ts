/* eslint-disable */
import * as types from './graphql';
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 */
const documents = {
    "\n    query GetBallotStyles {\n        sequent_backend_ballot_style(where: {deleted_at: {_is_null: true}}) {\n            id\n            election_id\n            election_event_id\n            status\n            tenant_id\n            ballot_eml\n            ballot_signature\n            created_at\n            area_id\n            annotations\n            labels\n            last_updated_at\n            deleted_at\n        }\n    }\n": types.GetBallotStylesDocument,
    "\n    query GetCastVotes {\n        sequent_backend_cast_vote {\n            id\n            tenant_id\n            election_id\n            area_id\n            created_at\n            last_updated_at\n            labels\n            annotations\n            content\n            cast_ballot_signature\n            voter_id_string\n            election_event_id\n        }\n    }\n": types.GetCastVotesDocument,
    "\n    query GetElections($electionIds: [uuid!]!) {\n        sequent_backend_election(where: {id: {_in: $electionIds}}) {\n            annotations\n            created_at\n            dates\n            description\n            election_event_id\n            eml\n            id\n            is_consolidated_ballot_encoding\n            labels\n            last_updated_at\n            name\n            num_allowed_revotes\n            presentation\n            spoil_ballot_option\n            status\n            tenant_id\n        }\n    }\n": types.GetElectionsDocument,
    "\n    mutation InsertCastVote(\n        $id: uuid\n        $electionId: uuid\n        $electionEventId: uuid\n        $tenantId: uuid\n        $areaId: uuid\n        $content: String!\n    ) {\n        insert_sequent_backend_cast_vote(\n            objects: {\n                id: $id\n                election_id: $electionId\n                election_event_id: $electionEventId\n                tenant_id: $tenantId\n                area_id: $areaId\n                content: $content\n            }\n        ) {\n            returning {\n                id\n                tenant_id\n                election_id\n                area_id\n                created_at\n                last_updated_at\n                labels\n                annotations\n                content\n                cast_ballot_signature\n                voter_id_string\n                election_event_id\n            }\n        }\n    }\n": types.InsertCastVoteDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = graphql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n    query GetBallotStyles {\n        sequent_backend_ballot_style(where: {deleted_at: {_is_null: true}}) {\n            id\n            election_id\n            election_event_id\n            status\n            tenant_id\n            ballot_eml\n            ballot_signature\n            created_at\n            area_id\n            annotations\n            labels\n            last_updated_at\n            deleted_at\n        }\n    }\n"): (typeof documents)["\n    query GetBallotStyles {\n        sequent_backend_ballot_style(where: {deleted_at: {_is_null: true}}) {\n            id\n            election_id\n            election_event_id\n            status\n            tenant_id\n            ballot_eml\n            ballot_signature\n            created_at\n            area_id\n            annotations\n            labels\n            last_updated_at\n            deleted_at\n        }\n    }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n    query GetCastVotes {\n        sequent_backend_cast_vote {\n            id\n            tenant_id\n            election_id\n            area_id\n            created_at\n            last_updated_at\n            labels\n            annotations\n            content\n            cast_ballot_signature\n            voter_id_string\n            election_event_id\n        }\n    }\n"): (typeof documents)["\n    query GetCastVotes {\n        sequent_backend_cast_vote {\n            id\n            tenant_id\n            election_id\n            area_id\n            created_at\n            last_updated_at\n            labels\n            annotations\n            content\n            cast_ballot_signature\n            voter_id_string\n            election_event_id\n        }\n    }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n    query GetElections($electionIds: [uuid!]!) {\n        sequent_backend_election(where: {id: {_in: $electionIds}}) {\n            annotations\n            created_at\n            dates\n            description\n            election_event_id\n            eml\n            id\n            is_consolidated_ballot_encoding\n            labels\n            last_updated_at\n            name\n            num_allowed_revotes\n            presentation\n            spoil_ballot_option\n            status\n            tenant_id\n        }\n    }\n"): (typeof documents)["\n    query GetElections($electionIds: [uuid!]!) {\n        sequent_backend_election(where: {id: {_in: $electionIds}}) {\n            annotations\n            created_at\n            dates\n            description\n            election_event_id\n            eml\n            id\n            is_consolidated_ballot_encoding\n            labels\n            last_updated_at\n            name\n            num_allowed_revotes\n            presentation\n            spoil_ballot_option\n            status\n            tenant_id\n        }\n    }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n    mutation InsertCastVote(\n        $id: uuid\n        $electionId: uuid\n        $electionEventId: uuid\n        $tenantId: uuid\n        $areaId: uuid\n        $content: String!\n    ) {\n        insert_sequent_backend_cast_vote(\n            objects: {\n                id: $id\n                election_id: $electionId\n                election_event_id: $electionEventId\n                tenant_id: $tenantId\n                area_id: $areaId\n                content: $content\n            }\n        ) {\n            returning {\n                id\n                tenant_id\n                election_id\n                area_id\n                created_at\n                last_updated_at\n                labels\n                annotations\n                content\n                cast_ballot_signature\n                voter_id_string\n                election_event_id\n            }\n        }\n    }\n"): (typeof documents)["\n    mutation InsertCastVote(\n        $id: uuid\n        $electionId: uuid\n        $electionEventId: uuid\n        $tenantId: uuid\n        $areaId: uuid\n        $content: String!\n    ) {\n        insert_sequent_backend_cast_vote(\n            objects: {\n                id: $id\n                election_id: $electionId\n                election_event_id: $electionEventId\n                tenant_id: $tenantId\n                area_id: $areaId\n                content: $content\n            }\n        ) {\n            returning {\n                id\n                tenant_id\n                election_id\n                area_id\n                created_at\n                last_updated_at\n                labels\n                annotations\n                content\n                cast_ballot_signature\n                voter_id_string\n                election_event_id\n            }\n        }\n    }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;