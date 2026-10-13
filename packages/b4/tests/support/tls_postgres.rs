// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A disposable TLS PostgreSQL server bound only to loopback.

use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::TempDir;

pub struct TlsPostgres {
    directory: TempDir,
    bin: PathBuf,
    pub port: u16,
}

fn checked(output: std::io::Result<Output>) {
    let output = output.expect("PostgreSQL/OpenSSL fixture command is available");
    assert!(
        output.status.success(),
        "TLS fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

impl TlsPostgres {
    pub fn start(tls: bool) -> Self {
        let bin = std::env::var_os("PG_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let output = Command::new("pg_config").arg("--bindir").output().unwrap();
                assert!(output.status.success());
                PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
            });
        let directory = tempfile::tempdir().unwrap();
        checked(
            Command::new(bin.join("initdb"))
                .args([
                    "--no-locale",
                    "--encoding=UTF8",
                    "--auth=trust",
                    "--username=b4_test",
                ])
                .arg("-D")
                .arg(directory.path().join("data"))
                .output(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = Self {
            directory,
            bin,
            port,
        };
        if tls {
            let openssl = std::env::var_os("OPENSSL_BIN").unwrap_or_else(|| "openssl".into());
            let command = |args: &[&str]| {
                checked(
                    Command::new(&openssl)
                        .current_dir(server.directory.path())
                        .args(args)
                        .output(),
                );
            };
            for (name, key) in [("ca.pem", "ca.key"), ("wrong-ca.pem", "wrong-ca.key")] {
                command(&[
                    "req",
                    "-x509",
                    "-newkey",
                    "rsa:2048",
                    "-nodes",
                    "-days",
                    "2",
                    "-subj",
                    "/CN=b4-test-ca",
                    "-addext",
                    "basicConstraints=critical,CA:TRUE",
                    "-keyout",
                    key,
                    "-out",
                    name,
                ]);
            }
            command(&[
                "req",
                "-new",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-subj",
                "/CN=localhost",
                "-keyout",
                "server.key",
                "-out",
                "server.csr",
            ]);
            fs::write(
                server.directory.path().join("server.ext"),
                "subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n",
            )
            .unwrap();
            command(&[
                "x509",
                "-req",
                "-in",
                "server.csr",
                "-CA",
                "ca.pem",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-days",
                "2",
                "-extfile",
                "server.ext",
                "-out",
                "server.pem",
            ]);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    server.directory.path().join("server.key"),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
            }
        }
        let settings = if tls {
            format!(
                "\nssl=on\nssl_cert_file='{}'\nssl_key_file='{}'\n",
                server.directory.path().join("server.pem").display(),
                server.directory.path().join("server.key").display()
            )
        } else {
            "\nssl=off\n".into()
        };
        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(server.directory.path().join("data/postgresql.conf"))
            .unwrap()
            .write_all(settings.as_bytes())
            .unwrap();
        // Startup owns the selected port; no test connects to a shared service.
        drop(listener);
        checked(
            Command::new(server.bin.join("pg_ctl"))
                .arg("-D")
                .arg(server.directory.path().join("data"))
                .arg("-l")
                .arg(server.directory.path().join("postgres.log"))
                .args(["-t", "10", "-w", "-o"])
                .arg(format!(
                    "-F -h 127.0.0.1 -p {} -k '{}'",
                    server.port,
                    server.directory.path().display()
                ))
                .arg("start")
                .output(),
        );
        server
    }

    pub fn ca(&self, filename: &str) -> PathBuf {
        self.directory.path().join(filename)
    }
}

impl Drop for TlsPostgres {
    fn drop(&mut self) {
        let output = Command::new(self.bin.join("pg_ctl"))
            .arg("-D")
            .arg(self.directory.path().join("data"))
            .args(["-m", "immediate", "-t", "10", "-w", "stop"])
            .output();
        if !std::thread::panicking() {
            checked(output);
        }
    }
}
