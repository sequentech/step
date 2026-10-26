#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Validate delivered OCI graphs and make private transport archives without a registry."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

INDEX_TYPES={'application/vnd.oci.image.index.v1+json','application/vnd.docker.distribution.manifest.list.v2+json'}
MANIFEST_TYPES={'application/vnd.oci.image.manifest.v1+json','application/vnd.docker.distribution.manifest.v2+json'}

def need(ok,message):
    if not ok:raise ValueError(message)

def hash_file(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()

def validate(root,expected):
    root=Path(root)
    need(root.is_dir() and not root.is_symlink(),'Missing OCI image directory')
    need(re.fullmatch(r'sha256:[0-9a-f]{64}',expected) is not None,'Invalid accepted OCI digest')
    for p in root.rglob('*'):
        need(not p.is_symlink(),'OCI symbolic links are refused')
        if p.is_file():
            relative=p.relative_to(root).as_posix()
            need(relative in ('index.json','oci-layout') or re.fullmatch(r'blobs/sha256/[0-9a-f]{64}',relative),'Unexpected OCI image path')
        else:need(p.is_dir(),'Special files are refused in OCI images')
    layout=(root/'oci-layout').read_bytes();index=(root/'index.json').read_bytes()
    need(json.loads(layout)=={'imageLayoutVersion':'1.0.0'},'Unsupported OCI layout version')
    top=json.loads(index)
    need(top.get('schemaVersion')==2 and isinstance(top.get('manifests'),list) and len(top['manifests'])==1,'Expected one image descriptor in the OCI index')
    need(top['manifests'][0].get('digest')==expected,'OCI index differs from the accepted image digest')
    files={'oci-layout':hashlib.sha256(layout).hexdigest(),'index.json':hashlib.sha256(index).hexdigest()}
    visited=set()
    def visit(descriptor,depth=0):
        need(depth<=32 and isinstance(descriptor,dict),'Invalid or overly deep OCI descriptor graph')
        digest=descriptor.get('digest');size=descriptor.get('size');media=descriptor.get('mediaType')
        need(isinstance(digest,str) and re.fullmatch(r'sha256:[0-9a-f]{64}',digest),'Invalid OCI blob digest')
        need(isinstance(size,int) and not isinstance(size,bool) and size>=0,'Invalid OCI blob size')
        relative='blobs/sha256/'+digest.split(':')[1];p=root/relative
        need(p.is_file() and not p.is_symlink() and p.stat().st_size==size,'OCI blob is missing or its size differs')
        need(hash_file(p)==digest.split(':')[1],'OCI blob checksum differs')
        files[relative]=digest.split(':')[1]
        if digest in visited:return
        visited.add(digest)
        if media in INDEX_TYPES:
            body=json.loads(p.read_bytes());need(body.get('schemaVersion')==2 and isinstance(body.get('manifests'),list) and body['manifests'],'Invalid OCI image index')
            for child in body['manifests']:visit(child,depth+1)
        elif media in MANIFEST_TYPES:
            body=json.loads(p.read_bytes());need(body.get('schemaVersion')==2 and isinstance(body.get('layers'),list),'Invalid OCI image manifest')
            visit(body.get('config'),depth+1)
            for child in body['layers']:visit(child,depth+1)
        else:
            need(media in {'application/vnd.oci.image.config.v1+json','application/vnd.docker.container.image.v1+json','application/vnd.oci.image.layer.v1.tar','application/vnd.oci.image.layer.v1.tar+gzip','application/vnd.oci.image.layer.v1.tar+zstd','application/vnd.docker.image.rootfs.diff.tar.gzip','application/vnd.docker.image.rootfs.foreign.diff.tar.gzip'},'Unsupported OCI blob media type')
    visit(top['manifests'][0])
    return files

class _CheckedReader:
    def __init__(self,stream):self.stream=stream;self.hash=hashlib.sha256()
    def read(self,size):
        data=self.stream.read(size);self.hash.update(data);return data

def archive(root,expected,out):
    root=Path(root);out=Path(out);files=validate(root,expected)
    need(not out.exists(),'Refusing to overwrite an image transport archive')
    try:
        with out.open('xb') as target:
            out.chmod(0o600)
            with tarfile.open(fileobj=target,mode='w') as tar:
                for relative,expected_hash in sorted(files.items()):
                    p=root/relative
                    need(not p.is_symlink() and p.resolve().is_relative_to(root.resolve()),'OCI image changed while preparing transport')
                    with p.open('rb') as stream:
                        reader=_CheckedReader(stream);info=tarfile.TarInfo(relative);info.size=p.stat().st_size;info.mode=0o600
                        tar.addfile(info,reader)
                        need(reader.hash.hexdigest()==expected_hash,'OCI image changed while preparing transport')
        return hash_file(out)
    except BaseException:
        if out.exists():out.unlink()
        raise
