#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Read-only, bounded deployment inspections; never expose protected inputs."""
import base64
import hashlib
import ipaddress
import json
from pathlib import Path
import re
import shlex
import socket
import subprocess
import install


def require(condition, message):
    if not condition:
        raise ValueError(message)


def kit(d):
    install.verify_kit(d)
    print('PASS: kit signing fingerprint matches the independently supplied value', flush=True)
    print('PASS: signature, checksums and required supporting-image records verified', flush=True)
    print('KIT CHECK PASSED — compare nixos/site.json with the approved site plan before booting either server', flush=True)


def port_open(host, port):
    try:
        with socket.create_connection((host, port), timeout=3):
            return True
    except OSError:
        return False


def validate_client(d):
    require(isinstance(d, dict) and set(d) == {'schema', 'nodes', 'ingress_ports', 'voting_host'}, 'Client network report has invalid fields')
    require(type(d['schema']) is int and d['schema'] == 1, 'Unsupported client network report schema')
    require(isinstance(d['nodes'], dict) and set(d['nodes']) == {'c1', 'c2'}, 'Client report must name both nodes')
    for value in d['nodes'].values():
        require(isinstance(value, str), 'Node address must be an IP address')
        try:
            ipaddress.ip_address(value)
        except ValueError:
            raise ValueError('Node address must be an IP address') from None
    require(d['nodes']['c1'] != d['nodes']['c2'], 'Node addresses must differ')
    ports = d['ingress_ports']
    require(isinstance(ports, list) and 0 < len(ports) <= 128, 'Ingress ports are missing or excessive')
    require(all(type(p) is int and 1 <= p <= 65535 for p in ports), 'Invalid ingress port')
    require(len(set(ports)) == len(ports), 'Duplicate ingress ports')
    require(isinstance(d['voting_host'], str) and re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?', d['voting_host']), 'Invalid voting host')
    return d


def save_client(d, path=Path('client-network.json')):
    validate_client(d)
    path = Path(path)
    body = json.dumps(d, indent=2) + '\n'
    require(not path.is_symlink(), 'Client network report path must not be a symlink')
    if path.exists():
        require(path.is_file() and path.read_text() == body, 'client-network.json already exists with different contents; preserve it and choose an empty working directory')
        return
    with path.open('x') as output:
        output.write(body)
    path.chmod(0o644)


def network(d):
    raw = install.ssh(d, 'c1', 'k3s kubectl -n kube-system get service traefik -o json\n', 'Ingress port inspection', timeout=15)
    service = json.loads(raw)
    service_ports = {port[key] for port in service['spec']['ports'] for key in ('port', 'nodePort') if port.get(key)}
    deployment = json.loads(install.ssh(d, 'c1', 'k3s kubectl -n kube-system get deployment traefik -o json\n', 'Ingress listener inspection', timeout=15))
    arguments = [arg.lower() for container in deployment['spec']['template']['spec']['containers'] for arg in container.get('args', []) if arg.lower().startswith('--entrypoints.') and '.address=' in arg.lower()]
    matches = [re.fullmatch(r'--entrypoints\.[a-z0-9_-]+\.address=127\.0\.0\.1:([0-9]+)(?:/tcp)?', arg) for arg in arguments]
    require(arguments and all(matches), 'An ingress listener is not restricted to C1 loopback')
    listener_ports = {int(match[1]) for match in matches}
    ports = sorted(service_ports | listener_ports)
    report = validate_client({'schema': 1, 'nodes': d['nodes'].copy(), 'ingress_ports': ports, 'voting_host': d['hosts']['voting']})
    print('C1 ingress service ports and node ports: ' + ' '.join(map(str, sorted(service_ports))), flush=True)
    print('C1 ingress loopback listener ports: ' + ' '.join(map(str, sorted(listener_ports))), flush=True)
    policy = r'''policy=$(sshd -T)
printf '%s\n' "$policy" | grep -iE '^(passwordauthentication|pubkeyauthentication|permitrootlogin) '
printf '%s\n' "$policy" | grep -qi '^passwordauthentication no$'
printf '%s\n' "$policy" | grep -qi '^pubkeyauthentication yes$'
printf '%s\n' "$policy" | grep -qiE '^permitrootlogin (prohibit-password|without-password)$'
auth=$(chronyc -N authdata)
printf '%s\n' "$auth"
printf '%s\n' "$auth" | awk 'NR>2 {n++; if ($2 != "NTS") bad=1} END {exit !(n>0 && !bad)}'
chronyc tracking | grep -E 'Leap status'
chronyc tracking | grep -qE 'Leap status *: Normal'
chronyc sources | grep -E '^.\*'
'''
    for node in ('c1', 'c2'):
        require(port_open(d['nodes'][node], 22), node.upper() + ' administration SSH port is not reachable')
        print(node.upper() + ' ' + d['nodes'][node] + ':22 open from this administration workstation', flush=True)
        print(install.ssh(d, node, policy, node.upper() + ' SSH and authenticated-time inspection', timeout=20).decode().strip(), flush=True)
        print('PASS: ' + node.upper() + ' key-only SSH and synchronized NTS time', flush=True)
    raw = install.ssh(d, 'c1', 'k3s secrets-encrypt status\nk3s kubectl get nodes -o wide\n', 'Cluster encryption and node inspection', timeout=15).decode()
    print(raw.strip(), flush=True)
    require('Encryption Status: Enabled' in raw, 'Cluster Secrets encryption is not enabled')
    nodes = json.loads(install.ssh(d, 'c1', 'k3s kubectl get nodes -o json\n', 'Cluster readiness readback', timeout=15))['items']
    require(len(nodes) == 2, 'Expected exactly two cluster nodes')
    require(all(any(x['type'] == 'Ready' and x['status'] == 'True' for x in node['status']['conditions']) for node in nodes), 'A cluster node is not Ready')
    addresses = {a['address'] for node in nodes for a in node['status']['addresses'] if a['type'] == 'InternalIP'}
    require(addresses == set(d['nodes'].values()), 'Cluster addresses differ from the site configuration')
    save_client(report)
    print('PASS: both configured nodes Ready; cluster Secrets encryption Enabled', flush=True)
    print('Saved public client-network.json. Copy only this report and the verified deployment tools to a separate client-network computer.', flush=True)
    print('ADMINISTRATION NETWORK CHECK PASSED — client-network filtering still requires the separate client test', flush=True)


def client_network(d):
    validate_client(d)
    require(port_open(d['nodes']['c1'], 443), 'Required positive control failed: C1:443 must be reachable from the client network')
    print('PASS: positive control ' + d['nodes']['c1'] + ':443 open', flush=True)
    ports = sorted(set([22, 80, 443, 6443, 10250] + d['ingress_ports']))
    for node in ('c1', 'c2'):
        for port in ports:
            if node == 'c1' and port == 443:
                continue
            opened = port_open(d['nodes'][node], port)
            print(node.upper() + ' ' + d['nodes'][node] + ':' + str(port) + (' open — FORBIDDEN' if opened else ' closed'), flush=True)
            require(not opened, 'A forbidden client-network port is reachable; correct the site firewall before continuing')
    print('CLIENT NETWORK CHECK PASSED — positive HTTPS reachability and all tested forbidden ports closed', flush=True)
    print('This TCP test does not validate a certificate or prove the physical network position. Record the test machine and network.', flush=True)


def check_allowed(code, output):
    require(code == 0, 'Allowed TLS connection failed; check name, CA, route and endpoint')
    require(re.search(r'New, TLSv1\.3, Cipher is TLS_AES_256_GCM_SHA384', output), 'Allowed TLS version or cipher differs')
    require(re.search(r'(?:Peer|Server) Temp Key:.*secp384r1.*384 bits', output), 'TLS key exchange must use secp384r1')
    require('Server public key is 384 bit' in output, 'TLS server public key must be 384 bits')
    require('Verify return code: 0 (ok)' in output, 'TLS certificate verification did not succeed')


def check_rejected(code, output):
    require(code not in (124, 137), 'TLS probe timed out; timeout is not proof of protocol refusal')
    negotiated = re.search(r'(?:New|Reused), TLSv[^,]+, Cipher is (?!\(NONE\))\S+', output)
    require(code != 0 and not negotiated, 'A prohibited TLS connection was accepted')


def probe(arguments, d=None, node=None):
    if node:
        command = ' '.join('"$agd_ca"' if part == '__AGD_CA_FILE__' else shlex.quote(part) for part in ['timeout', '8', 'openssl', 's_client'] + arguments)
        encoded = base64.b64encode(d['_inspection_auth_ca']).decode()
        script = "agd_ca=$(mktemp /tmp/agd-inspection-ca.XXXXXXXX)\ntrap 'rm -f \"$agd_ca\"' EXIT\nprintf %s " + shlex.quote(encoded) + " | base64 -d > \"$agd_ca\"\nset +e\nprintf '\\n' | " + command + " 2>&1\nstatus=$?\nprintf '\\nAGD_TLS_EXIT=%s\\n' \"$status\"\n"
        raw = install.ssh(d, node, script, node.upper() + ' authentication-server TLS probe', timeout=15).decode(errors='replace')
        match = re.search(r'\nAGD_TLS_EXIT=([0-9]+)\n$', raw)
        require(match is not None, 'TLS probe result is incomplete')
        return int(match[1]), raw[:match.start()]
    try:
        result = subprocess.run(['openssl', 's_client'] + arguments, input=b'\n', stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=8, check=False)
    except subprocess.TimeoutExpired:
        raise ValueError('TLS probe timed out; no successful check is recorded') from None
    return result.returncode, result.stdout.decode(errors='replace')


def endpoint(d, address, host, ca, label, node=None, names=False):
    target = '[' + address + ']:443' if ':' in address else address + ':443'
    common = ['-connect', target, '-servername', host, '-CAfile', ca, '-verify_hostname', host, '-verify_return_error']
    allowed = ['-tls1_3', '-ciphersuites', 'TLS_AES_256_GCM_SHA384', '-groups', 'secp384r1']
    code, output = probe(common + allowed, d, node)
    check_allowed(code, output)
    print('PASS: ' + label + ' — TLS 1.3 / TLS_AES_256_GCM_SHA384 / secp384r1 / 384-bit key / certificate verified', flush=True)
    prohibited = [('TLS 1.2', ['-tls1_2']), ('AES-128', ['-tls1_3', '-ciphersuites', 'TLS_AES_128_GCM_SHA256']), ('ChaCha20', ['-tls1_3', '-ciphersuites', 'TLS_CHACHA20_POLY1305_SHA256']), ('X25519', ['-tls1_3', '-groups', 'X25519'])]
    for name, options in prohibited:
        code, output = probe(common + options, d, node)
        check_rejected(code, output)
        print('PASS: ' + label + ' — ' + name + ' refused', flush=True)
    if names:
        for title, options in [('wrong SNI', ['-servername', 'other.invalid']), ('no SNI', ['-noservername'])]:
            code, output = probe(['-connect', target] + options + allowed, d, node)
            check_rejected(code, output)
            print('PASS: ' + label + ' — ' + title + ' refused', flush=True)
    code, output = probe(common + allowed, d, node)
    check_allowed(code, output)
    print('PASS: ' + label + ' — allowed connection still succeeds after rejection tests', flush=True)


def verified_ca(d, group):
    path = install.checked_path(d[group]['ca'], label=group + ' CA certificate')
    require(path.stat().st_size <= 262144, 'CA certificate input is too large')
    data = path.read_bytes()
    require(b'PRIVATE KEY' not in data, 'CA input must not contain a private key')
    der = install.run(['openssl', 'x509', '-in', str(path), '-outform', 'DER'], 'Inspection CA fingerprint')
    require(hashlib.sha256(der).hexdigest() == d[group]['ca_sha256'].lower(), 'Inspection CA fingerprint differs from the independently supplied value')
    return str(path), data


def tls(d):
    ca, _ = verified_ca(d, 'tls')
    _, auth_ca = verified_ca(d, 'identity_provider')
    inspection = dict(d, _inspection_auth_ca=auth_ca)
    for name in ('voting', 'administration', 'verifier'):
        host = d['hosts'][name]
        endpoint(d, d['nodes']['c1'], host, ca, 'TOE ' + host, names=name == 'voting')
    require(not port_open(d['nodes']['c1'], 80), 'C1 plain HTTP port 80 is open')
    print('PASS: C1 plain HTTP port 80 closed', flush=True)
    for node in ('c1', 'c2'):
        # A public CA certificate is staged in a temporary file and removed on exit.
        # Neither the client secret nor a private key is sent by these probes.
        host = d['hosts']['authentication']
        endpoint(inspection, host, host, '__AGD_CA_FILE__', node.upper() + ' to authentication server ' + host, node=node)
    print('TLS PROFILE CHECK PASSED — allowed connections verified and prohibited variants refused', flush=True)
