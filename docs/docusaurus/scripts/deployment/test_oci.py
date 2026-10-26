# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import hashlib,json,tarfile,tempfile,unittest
from pathlib import Path
import oci

class OCITests(unittest.TestCase):
    def image(self,root):
        (root/'blobs/sha256').mkdir(parents=True)
        def blob(value,media):
            raw=value if isinstance(value,bytes) else json.dumps(value).encode();digest=hashlib.sha256(raw).hexdigest();(root/'blobs/sha256'/digest).write_bytes(raw)
            return {'mediaType':media,'digest':'sha256:'+digest,'size':len(raw)}
        config=blob({'architecture':'amd64','os':'linux'},'application/vnd.oci.image.config.v1+json')
        layer=blob(b'layer-test-data','application/vnd.oci.image.layer.v1.tar')
        manifest=blob({'schemaVersion':2,'config':config,'layers':[layer]},'application/vnd.oci.image.manifest.v1+json')
        (root/'index.json').write_text(json.dumps({'schemaVersion':2,'manifests':[manifest]}));(root/'oci-layout').write_text('{"imageLayoutVersion":"1.0.0"}')
        return manifest,layer
    def test_private_transport_preserves_verified_graph(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'image';manifest,_=self.image(root);out=Path(folder)/'transport.tar'
            result=oci.archive(root,manifest['digest'],out)
            self.assertEqual(result,hashlib.sha256(out.read_bytes()).hexdigest());self.assertEqual(out.stat().st_mode&0o777,0o600)
            with tarfile.open(out) as t:
                self.assertEqual(set(t.getnames()),set(oci.validate(root,manifest['digest'])))
                for member in t.getmembers():self.assertEqual(t.extractfile(member).read(),(root/member.name).read_bytes())
    def test_changed_layer_is_rejected_before_transport(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'image';manifest,layer=self.image(root);(root/'blobs/sha256'/layer['digest'].split(':')[1]).write_bytes(b'changed')
            with self.assertRaises(ValueError):oci.archive(root,manifest['digest'],Path(folder)/'transport.tar')
            self.assertFalse((Path(folder)/'transport.tar').exists())
    def test_index_cannot_select_a_different_image(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'image';self.image(root)
            with self.assertRaisesRegex(ValueError,'index differs'):oci.validate(root,'sha256:'+'a'*64)
    def test_symlinks_and_unexpected_paths_are_refused(self):
        for kind in ('symlink','unexpected'):
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as folder:
                root=Path(folder)/'image';manifest,_=self.image(root)
                if kind=='symlink':(root/'linked').symlink_to('/etc/passwd')
                else:(root/'unexpected').write_text('not an OCI member')
                with self.assertRaises(ValueError):oci.validate(root,manifest['digest'])
    def test_descriptor_size_is_authenticated(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'image';manifest,_=self.image(root);manifest['size']+=1
            (root/'index.json').write_text(json.dumps({'schemaVersion':2,'manifests':[manifest]}))
            with self.assertRaisesRegex(ValueError,'size differs'):oci.validate(root,manifest['digest'])
if __name__=='__main__':unittest.main()
