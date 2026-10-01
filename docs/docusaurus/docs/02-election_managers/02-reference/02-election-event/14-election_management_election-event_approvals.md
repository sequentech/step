---
id: election_management_election_event_approvals
title: Approvals
---

The **Approvals** tab of an election event lists the voters' enrollment
applications. It opens filtered to the pending ones, the applications that
wait for an election manager.

## Where approvals come from

When a voter enrolls, the enrollment is matched against the voters of the
census by the attributes the realm searches with (for example names, date of
birth and Post). Every enrollment is recorded as an application, with one of
these outcomes:

| Outcome | When |
| --- | --- |
| Accepted | It matches a voter of the census. |
| Pending | It matches a voter only partly, for example on all but one attribute, and needs a manual verification. |
| Rejected | It matches no voter, or the voter is already enrolled. |

An enrollment that matches **no voter of the census** is rejected by default.
An election event's realm can send it to Approvals instead, with the
`no-matching-voter-policy` setting of its census lookup
(`lookup-and-update-user`) set to `PENDING_APPROVAL`; it then appears as
pending, with the reason *no matching voter*. The COMELEC template does this.
See
[Scanovate Identity Verification](../../../integrations/scanovate_identity_verification_guide.md#after-the-identity-verification).

## Reviewing a pending application

Open the application to see the data the voter enrolled with, and the voters
of the census that match it. Approving it links the enrollment to one of those
voters. A voter who isn't in the census yet has to be added to it before the
application can be approved; otherwise, reject it.
