"""Check executable-only release packaging and current-run artifact transfer."""

from pathlib import Path
import os
import stat
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock
import zipfile

import package_release


ROOT = Path(__file__).resolve().parents[2]


class PackagingWorkflowTests(unittest.TestCase):
    def test_native_builds_are_packaged_before_artifact_upload(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        native = workflow.split('\n  native:\n', 1)[1].split('\n  release:\n', 1)[0]
        self.assertIn('.github/scripts/package_release.py', native)
        self.assertLess(native.index('cargo build --locked --release --bin smolsbx'), native.index('.github/scripts/package_release.py'))
        self.assertLess(native.index('check_baseline_independence'), native.index('.github/scripts/package_release.py'))
        self.assertLess(native.index('.github/scripts/package_release.py'), native.index('uses: actions/upload-artifact@'))
        self.assertIn("PACKAGE_PYTHON: ${{ runner.os == 'Windows' && 'python' || 'python3' }}", native)
        self.assertIn('"$PACKAGE_PYTHON" .github/scripts/package_release.py', native)
        self.assertIn('path: release-assets/*.zip', native)
        self.assertIn('overwrite: true', native)
        self.assertNotIn('target/${{ matrix.target }}/release/smolsbx.exe', native)

    def test_release_downloads_current_run_zips_before_publication(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        release = workflow.split('\n  release:\n', 1)[1]
        self.assertIn('uses: actions/download-artifact@', release)
        self.assertIn('pattern: smolsbx-*', release)
        self.assertIn('merge-multiple: true', release)
        self.assertIn('path: release-assets', release)
        self.assertIn('RELEASE_ASSET_DIR: release-assets', release)
        self.assertLess(release.index('uses: actions/download-artifact@'), release.index('python3 .github/scripts/release.py'))
        for field in ('run-id:', 'repository:', 'github-token:'):
            self.assertNotIn(field, release)


class PackageReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.output = self.root / 'assets'
        (self.root / 'Cargo.toml').write_text('[package]\nname = "smolsandbox"\nversion = "1.2.3"\n')
        (self.root / 'Cargo.lock').write_text('version = 4\n[[package]]\nname = "smolsandbox"\nversion = "1.2.3"\n')
        self.messages = []

    def executable(self, target):
        if target.endswith('-linux-gnu'):
            content = bytearray(64)
            content[:6] = b'\x7fELF\x02\x01'
            content[18:20] = (62 if target.startswith('x86_64-') else 183).to_bytes(2, 'little')
        elif target.endswith('-apple-darwin'):
            content = bytearray(b'\xcf\xfa\xed\xfe' + (0x0100000C).to_bytes(4, 'little'))
        else:
            content = bytearray(128)
            content[:2] = b'MZ'
            content[60:64] = (64).to_bytes(4, 'little')
            content[64:70] = b'PE\x00\x00\x64\x86'
        source = self.root / 'target' / target / 'release' / package_release.TARGETS[target][0]
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(content)
        source.chmod(0o755)
        return source

    def package(self, target, runner=None):
        if runner is None:
            runner = mock.Mock(return_value=SimpleNamespace(returncode=0, stdout='smolsbx 1.2.3\n'))
        return package_release.package(self.root, target, self.output, '1' * 40,
                                       report=self.messages.append, runner=runner)

    def test_all_targets_contain_only_the_native_executable_with_fixed_permissions(self):
        for target, (name, _) in package_release.TARGETS.items():
            with self.subTest(target=target):
                source = self.executable(target)
                def run(arguments, **options):
                    extracted = Path(arguments[0])
                    self.assertEqual(arguments[1:], ['--version'])
                    self.assertEqual(extracted.read_bytes(), source.read_bytes())
                    if name == 'smolsbx':
                        self.assertEqual(stat.S_IMODE(extracted.stat().st_mode), 0o755)
                        self.assertTrue(os.access(extracted, os.X_OK))
                    self.assertTrue(options['capture_output'])
                    self.assertTrue(options['text'])
                    return SimpleNamespace(returncode=0, stdout='smolsbx 1.2.3\n')
                archive = self.package(target, run)
                self.assertEqual(archive.name, f'smolsbx-{target}.zip')
                with zipfile.ZipFile(archive) as zipped:
                    self.assertEqual(zipped.namelist(), [name])
                    self.assertEqual(zipped.read(name), source.read_bytes())
                    entry = zipped.infolist()[0]
                    self.assertEqual(entry.create_system, 3)
                    self.assertTrue(stat.S_ISREG(entry.external_attr >> 16))
                    self.assertEqual(stat.S_IMODE(entry.external_attr >> 16), 0o755 if name == 'smolsbx' else 0o644)
                    self.assertEqual(entry.date_time, (1980, 1, 1, 0, 0, 0))
                before = archive.read_bytes()
                os.utime(source, (1900000000, 1900000000))
                self.assertEqual(self.package(target, run).read_bytes(), before)
        self.assertTrue(any('release commit ' + '1' * 40 in message for message in self.messages))
        self.assertTrue(any('SHA-256' in message for message in self.messages))

    def test_missing_and_invalid_architecture_fail_for_each_named_target(self):
        for target in package_release.TARGETS:
            with self.subTest(target=target):
                with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*missing'):
                    self.package(target)
                source = self.executable(target)
                source.write_bytes(b'source archive or wrong architecture')
                with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*header'):
                    self.package(target)

    def test_two_versions_keep_stable_names_and_check_the_selected_executable_version(self):
        for version in ('1.2.3', '2.0.0'):
            (self.root / 'Cargo.toml').write_text(f'[package]\nname = "smolsandbox"\nversion = "{version}"\n')
            (self.root / 'Cargo.lock').write_text(f'version = 4\n[[package]]\nname = "smolsandbox"\nversion = "{version}"\n')
            for target in package_release.TARGETS:
                with self.subTest(version=version, target=target):
                    self.executable(target)
                    runner = mock.Mock(return_value=SimpleNamespace(returncode=0, stdout=f'smolsbx {version}\n'))
                    archive = self.package(target, runner)
                    self.assertEqual(archive.name, f'smolsbx-{target}.zip')
                    runner.assert_called_once()
                    wrong_version = mock.Mock(return_value=SimpleNamespace(returncode=0, stdout='smolsbx 0.0.0\n'))
                    with self.assertRaisesRegex(package_release.ReleaseError, f'expected.*{version}'):
                        self.package(target, wrong_version)
            self.assertEqual({path.name for path in self.output.iterdir()},
                             {f'smolsbx-{target}.zip' for target in package_release.TARGETS})

    def test_permission_denied_read_fails_with_target(self):
        target = 'x86_64-unknown-linux-gnu'
        self.executable(target)
        with mock.patch.object(Path, 'read_bytes', side_effect=PermissionError('permission denied')):
            with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*permission denied'):
                self.package(target)

    def test_wrong_elf_machine_is_rejected(self):
        target = 'aarch64-unknown-linux-gnu'
        content = self.executable('x86_64-unknown-linux-gnu').read_bytes()
        self.executable(target).write_bytes(content)
        with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*ARM64'):
            self.package(target)

    def test_version_mismatch_exit_failure_and_timeout_fail_with_target(self):
        target = 'x86_64-unknown-linux-gnu'
        self.executable(target)
        outcomes = [SimpleNamespace(returncode=0, stdout='smolsbx 0.0.0'),
                    SimpleNamespace(returncode=1, stdout='smolsbx 1.2.3'),
                    subprocess.TimeoutExpired('smolsbx', 30)]
        for outcome in outcomes:
            with self.subTest(outcome=outcome):
                runner = mock.Mock(side_effect=outcome) if isinstance(outcome, Exception) else mock.Mock(return_value=outcome)
                with self.assertRaisesRegex(package_release.ReleaseError, target + ':'):
                    self.package(target, runner)

    def test_invalid_metadata_and_output_fail_with_target(self):
        target = 'x86_64-unknown-linux-gnu'
        self.executable(target)
        (self.root / 'Cargo.lock').write_text('invalid TOML')
        with self.assertRaisesRegex(package_release.ReleaseError, target + ':'):
            self.package(target)
        (self.root / 'Cargo.lock').write_text('version = 4\n[[package]]\nname = "smolsandbox"\nversion = "1.2.3"\n')
        self.output.write_text('not a directory')
        with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*packaging failed'):
            self.package(target)

    @unittest.skipIf(os.name == 'nt', 'Unix permissions and symlinks require a Unix host')
    def test_nonexecutable_and_symlink_sources_are_rejected(self):
        target = 'x86_64-unknown-linux-gnu'
        source = self.executable(target)
        source.chmod(0o644)
        with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*permission'):
            self.package(target)
        original = source.with_name('original')
        source.rename(original)
        source.symlink_to(original)
        with self.assertRaisesRegex(package_release.ReleaseError, target + ':.*missing'):
            self.package(target)


if __name__ == '__main__':
    unittest.main()
