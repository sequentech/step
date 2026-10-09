---
id: developers_windmill
title: Developers Windmill
---

# Tally

## Discarded/Auditable Ballots

Discarded Ballots that for some reason are not included in the tally. Possible reasons:

- The voter was disabled.
- The voter was deleted.
- The ballot was cast outside the Voting Period.
- The voter is not authorized for the election of the ballot.
- The voter is not assigned to the area of the ballot.
- The ballot is from a previous revote (only the last vote counts). (Note, this is not counted as a discarded ballot yet).

## Eligible Voters

Eligible voters are voters that can vote. Voters not included here are disabled and deleted voters.

## Report document storage

Generated reports are stored in the private bucket by default, including previews,
manual-verification documents, activity logs, statistical reports, ballot images
and electoral-results reports. Real ballot receipts remain public for voter downloads.
Administrators continue downloading private reports through the authenticated `fetchDocument`
action and its presigned URL. Email delivery attaches the generated file directly.
Explicit public results publications and support materials keep their own upload
behavior.

The public bucket permits anonymous `s3:GetObject` reads only. It does not grant
anonymous listing or uploads. Re-running MinIO configuration replaces the existing
public-bucket policy. S3 buckets provisioned separately need the equivalent policy.

### Existing report objects

Changing the upload policy does not move existing objects. Inventory public report
documents using authenticated administrative access, then copy each non-receipt
report into the private bucket and update its document row to `is_public = false`.
For an event document, the old object key is
`tenant-<tenant-id>/document-<document-id>/<name>` and the private key is
`tenant-<tenant-id>/event-<event-id>/document-<document-id>/<name>`. Verify the private
object and authenticated download before removing the public copy. Include
encrypted `.epdf` files and report previews. Preserve intended public assets and
ballot receipts.

Treat pending manual-verification links contained in previously public files as
exposed. Invalidate those action tokens through the deployment's Keycloak procedure
and re-issue verification documents after migration. Deleting an object alone does
not revoke its token.

Run the configuration regression with
`python3 scripts/test_public_bucket_policy.py`. Report visibility is covered by
Windmill's `document_visibility` unit tests.
