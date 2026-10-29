# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Extract concrete image references only in syntactically appropriate files."""
from pathlib import PurePosixPath
from contextlib import contextmanager
from tempfile import TemporaryDirectory, TemporaryFile
from pathlib import Path
import tarfile
import csv
import re
import subprocess

def image_reference(filename, line):
    path = PurePosixPath(filename)
    if path.name.startswith('Dockerfile') or path.name.endswith('.Dockerfile'):
        match = re.match(r'^\s*FROM\s+(?:--platform=\S+\s+)?(\S+)', line, re.I)
    elif path.suffix in ('.yaml', '.yml'):
        match = re.match(r'^\s*image:\s*[\"\x27]?([^\s\"\x27]+)', line)
    else:
        return None
    if not match:
        return None
    ref = match.group(1)
    if ref == 'scratch' or not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9._/:@+-]*', ref):
        return None
    return ref


def image_version(reference):
    """A registry port or digest is not an image tag."""
    name = reference.split('@', 1)[0].rsplit('/', 1)[-1]
    return name.rsplit(':', 1)[1] if ':' in name else 'unversioned'


def gitlink(repository, path):
    """Read a committed submodule pin, never an assumed checkout pairing."""
    entry = subprocess.check_output(
        ['git', '-C', str(repository), 'ls-tree', 'HEAD', '--', path], text=True
    ).strip().split()
    if len(entry) == 4 and entry[:2] == ['160000', 'commit'] and entry[3] == path:
        return entry[2]
    return None


def evidence_review_status(status, profile):
    """License evidence cannot approve ownership, scope or other review gates."""
    if profile == 'UNRESOLVED':
        return 'Needs confirmation'
    if status == 'Source evidence collected':
        return 'Source license evidence recorded'
    return status


def write_platform_dependencies(path, records, repository):
    """Include a header-only inventory when no provided dependencies remain."""
    with path.open('w', newline='', encoding='utf-8') as stream:
        writer = csv.DictWriter(stream, fieldnames=['repository', 'component', 'purl', 'reason', 'evidence'])
        writer.writeheader()
        writer.writerows(row for row in records if row['repository'] == repository)


@contextmanager
def pinned_source(repository, revision):
    """Read committed content without switching or modifying a source checkout."""
    with TemporaryDirectory(prefix='sbom-pin-') as directory, TemporaryFile() as archive:
        subprocess.run(['git', '-C', str(repository), 'archive', '--format=tar', revision],
                       stdout=archive, check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as source:
            source.extractall(directory, filter='data')
        yield Path(directory)


def step_source_matches(row, pin, local_identity):
    """Never substitute same-name Git/registry packages with local source rights."""
    from urllib.parse import urlsplit
    source = row.get('_source', '')
    if source:
        if not source.startswith('git+'):
            return False
        url = urlsplit(source[4:])
        return (url.scheme == 'https' and url.netloc == 'github.com'
                and url.path.removesuffix('.git') == '/sequentech/step'
                and url.fragment == pin)
    return local_identity


def module_rights_profile(license, classification):
    if not license or 'NOASSERTION' in license or 'LicenseRef-Trellis' in license:
        return 'UNRESOLVED'
    return 'SEQUENT' if classification == 'Sequent-authored module' else 'OPEN'


def local_step_identities(step_root, beyond_root):
    """Follow declared local build paths in Step's nested Beyond build layout.

    Return reachable Step manifests and their Maven coordinates; package names
    alone are never evidence that a dependency belongs to this source tree.
    """
    import posixpath
    import tomllib
    import xml.etree.ElementTree as ET
    manifests = {}
    for root, prefix in [(step_root, '/source'), (beyond_root, '/source/beyond')]:
        for name in ('Cargo.toml', 'pom.xml'):
            for path in (root/'packages').rglob(name):
                manifests[prefix+'/'+path.relative_to(root).as_posix()] = path
    cargo = {key: tomllib.loads(path.read_text()) for key, path in manifests.items() if key.endswith('Cargo.toml')}
    xml = {key: ET.parse(path).getroot() for key, path in manifests.items() if key.endswith('pom.xml')}
    ns = {'m': 'http://maven.apache.org/POM/4.0.0'}
    def join(key, path):
        return posixpath.normpath(posixpath.join(posixpath.dirname(key), path))
    def dependency_tables(data):
        for key, value in data.items():
            if key in ('dependencies', 'dev-dependencies', 'build-dependencies'):
                yield value
            elif key == 'target':
                for target in value.values():
                    yield from dependency_tables(target)
    def cargo_edges(key):
        data = cargo[key]
        for table in dependency_tables(data):
            for name, value in table.items():
                origin = key
                if not isinstance(value, dict):
                    continue
                if value.get('workspace'):
                    parent = posixpath.dirname(key)
                    while parent != '/':
                        candidate = parent+'/Cargo.toml'
                        workspace = cargo.get(candidate, {}).get('workspace')
                        if workspace is not None:
                            value = workspace.get('dependencies', {}).get(name, {})
                            origin = candidate
                            break
                        parent = posixpath.dirname(parent)
                if isinstance(value, dict) and 'path' in value:
                    yield join(origin, value['path']+'/Cargo.toml')
    pending = [key for key in manifests if key.startswith('/source/beyond/')]
    seen = set()
    while pending:
        key = pending.pop()
        if key in seen or key not in manifests:
            continue
        seen.add(key)
        if key in cargo:
            pending.extend(cargo_edges(key))
        else:
            doc = xml[key]
            parent = doc.find('m:parent', ns)
            if parent is not None:
                relative = parent.find('m:relativePath', ns)
                path = '../pom.xml' if relative is None else relative.text
                if path:
                    pending.append(join(key, path))
            pending.extend(join(key, node.text+'/pom.xml') for node in doc.findall('m:modules/m:module', ns) if node.text)
    result = {}
    for key in seen:
        if key.startswith('/source/beyond/'):
            continue
        coordinate = None
        if key in xml:
            doc = xml[key]
            group = doc.findtext('m:groupId', namespaces=ns) or doc.findtext('m:parent/m:groupId', namespaces=ns)
            name = doc.findtext('m:artifactId', namespaces=ns)
            version = doc.findtext('m:version', namespaces=ns) or doc.findtext('m:parent/m:version', namespaces=ns)
            if group and name and version:
                coordinate = f'pkg:maven/{group}/{name}@{version}'
        result[key.removeprefix('/source/')] = coordinate
    return result
