# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import io
import tarfile
import unittest
import archive_check

class ArchiveTests(unittest.TestCase):
    def archive(self, extra=None):
        out=io.BytesIO()
        with tarfile.open(fileobj=out,mode='w') as tar:
            for name in ['root/deploy/config','var/lib/sequent-data/state']:
                info=tarfile.TarInfo(name);info.size=4;tar.addfile(info,io.BytesIO(b'data'))
            if extra is not None:tar.addfile(extra,io.BytesIO(b'x'*extra.size))
        return out.getvalue()

    def test_complete_backup_is_accepted(self):
        archive_check.validate(io.BytesIO(self.archive()),'system')

    def test_traversal_absolute_and_unexpected_paths_are_rejected(self):
        for name in ['/etc/passwd','root/deploy/../../etc/passwd','var/lib/sequent-data/../oops','root/deploy2/file']:
            with self.subTest(name=name),self.assertRaises(ValueError):
                archive_check.validate(io.BytesIO(self.archive(tarfile.TarInfo(name))),'system')

    def test_links_devices_and_duplicate_files_are_rejected(self):
        for kind in [tarfile.SYMTYPE,tarfile.LNKTYPE,tarfile.CHRTYPE,tarfile.FIFOTYPE]:
            info=tarfile.TarInfo('root/deploy/link');info.type=kind;info.linkname='/etc'
            with self.assertRaises(ValueError):archive_check.validate(io.BytesIO(self.archive(info)),'system')
        with self.assertRaises(ValueError):archive_check.validate(io.BytesIO(self.archive(tarfile.TarInfo('root/deploy/config'))),'system')

    def test_truncated_body_and_wrong_backup_kind_are_rejected(self):
        with self.assertRaises((ValueError,tarfile.TarError)):
            archive_check.validate(io.BytesIO(self.archive()[:600]),'system')
        with self.assertRaises(ValueError):archive_check.validate(io.BytesIO(self.archive()),'trustee')

if __name__=='__main__':unittest.main()
