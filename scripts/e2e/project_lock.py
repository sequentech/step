# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Hold a Compose-project lock across launcher execs and their child processes."""

import fcntl
import os
import re
import sys
from pathlib import Path


def lock_path(project):
    if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", project):
        raise ValueError("Invalid Compose project")
    # Deliberately independent of checkout, output directory and TMPDIR.
    directory = Path("/tmp") / f"step-e2e-project-locks-{os.getuid()}"
    directory.mkdir(mode=0o700, exist_ok=True)
    metadata = directory.stat()
    if metadata.st_uid != os.getuid() or metadata.st_mode & 0o077:
        raise ValueError(f"Project lock directory must be private: {directory}")
    return directory / project


def inherited_lock(path):
    try:
        descriptor = int(os.environ.get("STEP_E2E_PROJECT_LOCK_FD", ""))
        actual, expected = os.fstat(descriptor), path.stat()
        if (actual.st_dev, actual.st_ino) != (expected.st_dev, expected.st_ino):
            return None
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return descriptor
    except (OSError, ValueError):
        return None


def main():
    checking = sys.argv[1:2] == ["--check"]
    arguments = sys.argv[2:] if checking else sys.argv[1:]
    if not arguments or (not checking and len(arguments) < 2):
        return 2
    path = lock_path(arguments[0])
    descriptor = inherited_lock(path)
    if checking:
        return 0 if descriptor is not None else 1
    if descriptor is None:
        descriptor = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            os.close(descriptor)
            print(
                f"Project {arguments[0]} is already claimed by another run",
                file=sys.stderr,
            )
            return 1
    # Never unlink the file: competing processes must keep locking the same inode.
    os.set_inheritable(descriptor, True)
    os.environ["STEP_E2E_PROJECT_LOCK_FD"] = str(descriptor)
    os.execvpe(arguments[1], arguments[1:], os.environ)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(f"Could not claim E2E project: {error}", file=sys.stderr)
        sys.exit(1)
