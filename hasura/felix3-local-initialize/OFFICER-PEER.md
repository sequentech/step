# Officer peer connection on the ordinary Felix3 project

This is a separately installed, default-off Compose override for the existing
`postgres` and `harvest` services. It does not create a database, publish any
port, replace the app data volume, change the existing browser origin, or grant
a permission to an administrator or nominee. Use only after the exact backend
and Portal have been verified. It is not part of ordinary startup.

1.  On the pinned ordinary PostgreSQL image, read `SHOW unix_socket_directories`,
    `SHOW hba_file` and `SHOW ident_file`. This override is for the path
    `/var/run/postgresql`, not an alternative database instance. Prepare a new
    private host socket directory for that existing instance and appropriate
    operator-managed client group access; mount exactly this directory in any
    specifically authorized signer/CA service that expects that database socket.
    Do not share the data directory. Confirm no PostgreSQL service is already
    using the new directory.
2.  Choose a non-root UID/GID with no collision in either pinned image. Preserve
    all entries in each image's existing passwd file, appending one locked,
    non-login name `felix3_local_attachment_officer` for the chosen UID. Prepare
    private, read-only copies of both files. The PostgreSQL image must resolve
    the actual Unix peer to that exact name. A mapping from an unrelated
    username to the officer database login is not accepted by the backend.
    Set `FELIX3_LOCAL_HARVEST_PRIVATE_GID` to the dedicated supplementary
    group used only for this Harvest installation. All protected **private**
    Harvest inputs, including `harvest/broker-client-key.pem` and the private
    files under `harvest/setup/`, must stay UID 0, group that exact GID, mode 0440. Restrict parent traversal to the same designated group (for example
    root:that GID, 0550). Do not change incumbent keys, grant this group to other
    containers, or give the new UID write access. A rootless launch cannot
    read a root:root 0400 file. Preflight every private file with `test -r`
    from a throwaway container using the exact candidate Harvest UID, groups
    and read-only mounts. Do not read, copy or print the key bytes during this
    check. Do not switch a running app if any check fails.
3.  Stage protected copies of the **effective existing** HBA and ident files
    (not generated guesses), preserving existing integration rules. Prepend
    entries scoped to just the verified database and the two fixed service
    logins. In the following template replace `APP_DB` with the installed
    database identifier after confirming it, not a literal placeholder:

        local   APP_DB  felix3_local_attachment_officer   peer
        local   APP_DB  felix3_local_staff_registry       peer map=felix3_officer_registry
        local   all     felix3_local_attachment_officer,felix3_local_staff_registry reject
        host    all     felix3_local_attachment_officer,felix3_local_staff_registry all reject

    The dedicated ident entry, before any generic patterns, is:

        felix3_officer_registry felix3_local_attachment_officer felix3_local_staff_registry

    Maintain separate setup and normal app login rules. The existing setup
    adapter uses the protected setup password over this same Unix socket, not
    the officer's peer mapping. Admit only its own database name and login
    (for example `local APP_DB felix3_local_setup_portal scram-sha-256`
    followed by rejects for that login in every other database or over TCP).
    Independently preserve the private setup device, Gold, and CA role rules.
    Both officer and registry logins have no password, privileged flags,
    ownership, or role memberships. Keep the assessment/worker identity rules separate. Do not
    give either login general ledger or approval privileges. Follow the narrow
    2250/2260 runtime grants and the existing transactional staff registry and
    audit requirements; a broad administrative pool is not a substitute.

4.  In an isolated release rehearsal, validate both mounted file formats and the
    exact role/ACL checks before activating the browser flag. From the scoped
    Harvest process each login must show its own `session_user`, the exact same
    `system_user = 'peer:felix3_local_attachment_officer'`, and
    `inet_client_addr() IS NULL`. A different OS user or any network client using
    those logins must fail. Verify the added officer routes are actually mounted
    and existing ordinary app health/roles are unchanged. The private certificate
    verifier and registry writer use different DB logins. These tests do not
    replace a real two-person Portal enrollment against trusted certificates.
5.  Applying or removing this override changes the socket mounts and user, so it
    normally restarts the existing PostgreSQL and Harvest containers. Preserve
    the exact running project, options, image identities and database volume to
    roll back; never remove the database volume or run a reset. Keep the browser
    setting disabled during checks. A failure disables the optional path rather
    than allowing a different signing mode.
