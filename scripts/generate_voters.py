#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
import csv
import sys
from datetime import datetime, timezone

"""
Generate a users CSV with
password_salt, hashed_password (PBKDF2-HMAC-SHA256), and num_of_iterations.

- email,email_verified,enabled,first_name,last_name,username,area_name,
  authorized-election-ids,annotations,election__Election,password_salt,
  hashed_password,num_of_iterations

Defaults:
- Underlying plaintext password for each user is: Qwerty1234!
- PBKDF2 algorithm: sha256
- Iterations: 27500
- Salt: 16 random bytes per user, base64-encoded
- election__Election: current UTC timestamp string

Usage:
  python generate_users.py <num_users> [output.csv]

Example:
  python generate_users.py 50 /Users/you/users-import2.csv
"""

DEFAULT_ITERATIONS = 27500
DEFAULT_AREA = "Area"
DEFAULT_AUTHORIZED_ELECTION_IDS = "Election"
DEFAULT_SALT = "+M7PT9XHdJZsX4woGdJ2Og=="
DEFAULT_HASH = "lfRVZO6/B5gc/4b44BuFAQu38hl616pxot3WSuGXCak="

def now_utc_str() -> str:
    # Format similar to the example: "YYYY-MM-DD HH:MM:SS.ssssss UTC"
    return datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S.%f UTC")


def generate_users_csv(num_users: int, output_filename: str = "users.csv") -> None:
    header = [
        "email",
        "email_verified",
        "enabled",
        "first_name",
        "last_name",
        "username",
        "area_name",
        "authorized-election-ids",
        "annotations",
        "election__Election",
        "password_salt",
        "hashed_password",
        "num_of_iterations",
    ]

    with open(output_filename, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(header)

        for i in range(1, num_users + 1):
            username = f"user{i}"
            email = f"{username}@sequentech.io"

            row = [
                email,
                "true",
                "true",
                username,            # first_name
                username,            # last_name (matches example CSV)
                username,            # username
                DEFAULT_AREA,
                DEFAULT_AUTHORIZED_ELECTION_IDS,
                "",                 # annotations
                now_utc_str(),       # election__Election
                DEFAULT_SALT,
                DEFAULT_HASH,
                str(DEFAULT_ITERATIONS),
            ]
            w.writerow(row)

    print(f"Created '{output_filename}' with {num_users} users.")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python generate_users.py <num_users> [output.csv]")
        sys.exit(1)

    try:
        num = int(sys.argv[1])
        if num < 1:
            print("Error: <num_users> must be an integer")
            sys.exit(1)
    except ValueError:
        print("Error: <num_users> must be an integer")
        sys.exit(1)

    out = sys.argv[2] if len(sys.argv) > 2 else f"users{num}.csv"
    generate_users_csv(num, out)

