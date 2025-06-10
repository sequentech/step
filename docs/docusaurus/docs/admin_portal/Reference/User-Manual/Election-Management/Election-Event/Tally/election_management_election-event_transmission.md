<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

---
id: election_management_election_event_transmission
title: Transmission
---

The Transmission module under the Tally tab enables sending election results to external servers that have been predefined in the system environment. This feature ensures that encrypted and signed tally data can be securely delivered to authorized endpoints for archival, auditing, or further processing.

## Prerequisites
- A completed Tally Ceremony: The tally must have been finalized and results are available.
- Preconfigured destination servers: External servers (with URLs, credentials, or endpoints) must be set up in the system environment beforehand.
- Trustee PnPKI files: Each trustee must possess their PnPKI signing files to sign the transmission package.

## Usage Steps

1. **Open the Transmission Module**
   - In the Administration Portal, navigate to the relevant **Election Event**.
   - Go to the **Tally** tab.
   - Locate and open the **Transmission** module or section.

2. **Select the Election for Transmission**
   - Click the antenna (transmission) icon associated with the Election or Election Event you wish to transmit.  
     - Typically, this icon appears next to the “Actions” for a completed tally.
   - Alternatively, under the Election “Actions” menu, choose **Send Transmission Package…**.

3. **Prepare the Transmission Package**
   - The system will package the finalized tally results into a transmission bundle.
   - Review any prompts or metadata fields (e.g., timestamp, package identifier) as required by your organization’s workflow.

4. **Trustee Signing**
   - Trustees must sign the transmission package using their PnPKI signing files:
     1. The interface will prompt each trustee to upload or apply their PnPKI file.
     2. Trustees follow on-screen instructions to digitally sign the package.
     3. Repeat for each required trustee until the threshold of signatures is collected.
   - Signing ensures authenticity and non-repudiation of the transmitted data.

5. **Send the Transmission Package**
   - Once all required signatures are applied, click the **Send** button.
   - The system will transmit the signed package to the designated external servers configured in the environment.

6. **Monitor Transmission Status**
   - **Destination Servers** section:  
     - Review the status of each server (e.g., Pending, In Progress, Success, Failure).  
     - Details may include timestamps, response codes, or error messages from the target endpoint.
   - **Logs**:  
     - Access the system logs for detailed entries about the transmission process.  
     - Logs record actions such as package creation, trustee signatures, transmission attempts, and server responses.

## Error Handling & Troubleshooting
- **Signature Issues**:  
  - If a trustee’s signature fails (invalid PnPKI file or mismatch), the system will prompt an error.  
  - Verify the correct PnPKI file and re-attempt signing.
- **Transmission Failures**:  
  - If sending to a server fails (network error, authentication issue, server down), the status will indicate an error.  
  - Check destination server configuration (URL, credentials, connectivity).  
  - Consult logs for detailed error messages and retry when resolved.
- **Partial Success**:  
  - If multiple destination servers are configured, some may succeed while others fail.  
  - Address failures individually and retransmit as needed.

## Best Practices
- **Predefine and Test Servers**:  
  - Configure destination endpoints in a test environment first to validate connectivity and authentication.
- **Secure Key Management**:  
  - Ensure trustees’ PnPKI files are stored and accessed securely during signing.
- **Audit Trails**:  
  - Keep logs and transmission receipts for audit purposes.  
  - Retain transmitted packages or checksums as evidence of delivery.
- **Retry Policies**:  
  - Establish procedures for automatic or manual retries upon transient failures.
- **Notification**:  
  - Configure notifications or alerts for successful or failed transmissions to relevant stakeholders.

---

By following these steps, administrators can securely package, sign, and transmit election results to external systems, while maintaining an auditable trail and ensuring data integrity throughout the process.


