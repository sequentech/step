// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

#[test]
fn only_a_whole_normal_device_descriptor_can_admit_the_software_profile() {
    let ids = ["tenant", "event", "post", "publication", "installation"];
    let mut good = json!({"profile": "softhsm-test"});
    for (index, field) in ids.iter().enumerate() {
        good[*field] = json!(Uuid::from_u128(index as u128 + 1));
    }
    let parse = |v: &serde_json::Value| Installation::parse(v.to_string().as_bytes());
    assert!(parse(&good).is_ok());
    for field in ids {
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(parse(&bad).is_err());
        bad[field] = json!(Uuid::nil());
        assert!(parse(&bad).is_err());
    }
    let mut bad = good.clone();
    bad["profile"] = json!("yubihsm");
    assert!(parse(&bad).is_err());
    bad.as_object_mut().unwrap().remove("profile");
    assert!(parse(&bad).is_err());
    bad = good.clone();
    bad["issuer"] = json!("unexpected");
    assert!(parse(&bad).is_err());
    assert!(parse(&json!([good["tenant"], good["event"], good["post"], good["publication"],
                          good["installation"], good["profile"]])).is_err());
    let padded = format!("{}{good}", " ".repeat(4096));
    assert!(Installation::parse(padded.as_bytes()).is_err());
}

#[test]
fn cross_event_and_incomplete_device_references_are_refused_before_the_db_lookup() {
    let id = Uuid::from_u128;
    let install = Installation {
        tenant: id(1), event: id(2), post: id(3), publication: id(4),
        installation: id(5), profile: "softhsm-test".into(),
    };
    let probe = DeviceCaProbe {
        tenant: id(1), event: id(2), request: id(6), official: id(7),
        challenge: id(8), fence: id(9),
    };
    assert!(install.accepts_reference(&probe));
    for wrong in [
        DeviceCaProbe { tenant: id(10), ..probe.clone() },
        DeviceCaProbe { event: id(10), ..probe.clone() },
        DeviceCaProbe { request: Uuid::nil(), ..probe.clone() },
        DeviceCaProbe { official: Uuid::nil(), ..probe.clone() },
        DeviceCaProbe { challenge: Uuid::nil(), ..probe.clone() },
        DeviceCaProbe { fence: Uuid::nil(), ..probe },
    ] {
        assert!(!install.accepts_reference(&wrong));
    }
}

#[test]
fn a_different_role_bridge_or_port_cannot_start_the_listener() {
    let assigned = Ipv4Addr::new(10, 1, 2, 3);
    let valid = json!({
        "role": "felix3-local-initialize-ca", "listen": "10.1.2.3:21443",
        "server_spki_sha256": "01".repeat(32), "server_ca_sha256": "02".repeat(32),
        "client_ca_sha256": "03".repeat(32),
    });
    let validate = |v: &serde_json::Value, ip| -> Result<()> {
        serde_json::from_value::<Registration>(v.clone())?.validate(ip)
    };
    assert!(validate(&valid, assigned).is_ok());
    assert!(validate(&valid, Ipv4Addr::new(10, 1, 2, 4)).is_err());
    for ip in [Ipv4Addr::UNSPECIFIED, Ipv4Addr::LOCALHOST, Ipv4Addr::new(192, 0, 2, 1)] {
        let mut bad = valid.clone();
        bad["listen"] = json!(format!("{ip}:21443"));
        assert!(validate(&bad, ip).is_err());
    }
    for endpoint in ["10.1.2.3:3002", "10.1.2.3:20443", "[::]:21443", "ca:21443"] {
        let mut bad = valid.clone();
        bad["listen"] = json!(endpoint);
        assert!(validate(&bad, assigned).is_err());
    }
    let mut bad = valid;
    bad["role"] = json!("device");
    assert!(validate(&bad, assigned).is_err());
}

#[test]
fn the_role_needs_its_own_read_only_mount_without_writable_overrides() {
    let ca = Path::new(ROOT).join("ca");
    let row = format!("1 2 0:3 / {} ro,nosuid - tmpfs tmpfs rw", ca.display());
    assert!(read_only_mount(&row, &ca));
    assert!(!read_only_mount("", &ca));
    assert!(!read_only_mount(&row.replace("/ca ", "/broker "), &ca));
    assert!(!read_only_mount(&row.replace("ro,nosuid", "rw"), &ca));
    let overlay = format!("{row}\n2 1 0:4 / {}/client-ca.pem rw - tmpfs tmpfs rw", ca.display());
    assert!(!read_only_mount(&overlay, &ca));
}
