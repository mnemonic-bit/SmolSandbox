"""Package and smoke-test one native production executable for a release."""

import argparse
import hashlib
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import zipfile

from release import ReleaseError, parse_version


TARGETS = {
    'x86_64-unknown-linux-gnu': ('smolsbx', 'ELF x86-64'),
    'aarch64-unknown-linux-gnu': ('smolsbx', 'ELF ARM64'),
    'aarch64-apple-darwin': ('smolsbx', 'Mach-O ARM64'),
    'x86_64-pc-windows-msvc': ('smolsbx.exe', 'PE x86-64'),
}


def check_architecture(content, target):
    valid = False
    if target.endswith('-linux-gnu'):
        machine = 62 if target.startswith('x86_64-') else 183
        valid = (len(content) >= 20 and content[:6] == b'\x7fELF\x02\x01'
                 and int.from_bytes(content[18:20], 'little') == machine)
    elif target == 'aarch64-apple-darwin':
        valid = (len(content) >= 8 and content[:4] == b'\xcf\xfa\xed\xfe'
                 and int.from_bytes(content[4:8], 'little') == 0x0100000C)
    elif target == 'x86_64-pc-windows-msvc' and len(content) >= 64:
        offset = int.from_bytes(content[60:64], 'little')
        valid = (content[:2] == b'MZ' and content[offset:offset + 4] == b'PE\x00\x00'
                 and int.from_bytes(content[offset + 4:offset + 6], 'little') == 0x8664)
    if not valid:
        raise ReleaseError(f'{target}: executable does not have the expected {TARGETS[target][1]} header')


def package(root, target, destination, sha, report=print, runner=subprocess.run):
    if target not in TARGETS:
        raise ReleaseError(f'{target}: unsupported release target')
    if not re.fullmatch(r'[0-9a-fA-F]{40}', sha):
        raise ReleaseError(f'{target}: release commit must be an immutable 40-character SHA')
    root = Path(root)
    destination = Path(destination)
    try:
        version = parse_version((root / 'Cargo.toml').read_text(), (root / 'Cargo.lock').read_text())
        executable, architecture = TARGETS[target]
        source = root / 'target' / target / 'release' / executable
        if source.is_symlink() or not source.is_file():
            raise ReleaseError(f'{target}: native production executable is missing: {source}')
        content = source.read_bytes()
        check_architecture(content, target)
        if executable == 'smolsbx' and not source.stat().st_mode & 0o111:
            raise ReleaseError(f'{target}: native production executable has no executable permission')
        destination.mkdir(parents=True, exist_ok=True)
        archive = destination / f'smolsbx-v{version}-{target}.zip'
        entry = zipfile.ZipInfo(executable, date_time=(1980, 1, 1, 0, 0, 0))
        entry.create_system = 3
        entry.external_attr = (stat.S_IFREG | (0o755 if executable == 'smolsbx' else 0o644)) << 16
        entry.compress_type = zipfile.ZIP_STORED
        with zipfile.ZipFile(archive, 'w') as output:
            output.writestr(entry, content)
        with tempfile.TemporaryDirectory(prefix='smolsbx-release-') as temporary:
            with zipfile.ZipFile(archive) as output:
                entries = output.infolist()
                if len(entries) != 1 or entries[0].filename != executable:
                    raise ReleaseError(f'{target}: binary ZIP must contain only {executable}')
                extracted = Path(output.extract(entries[0], temporary))
                extracted.chmod(entries[0].external_attr >> 16 & 0o777)
                if extracted.read_bytes() != content:
                    raise ReleaseError(f'{target}: extracted executable differs from the native production build')
                if executable == 'smolsbx' and not os.access(extracted, os.X_OK):
                    raise ReleaseError(f'{target}: extracted executable lost executable permission')
            result = runner([str(extracted), '--version'], capture_output=True, text=True, check=False, timeout=30)
            expected = f'smolsbx {version}'
            if result.returncode != 0 or result.stdout.strip() != expected:
                raise ReleaseError(f'{target}: extracted --version failed: expected {expected!r}, '
                                   f'got exit {result.returncode}, stdout {result.stdout.strip()!r}')
        report(f'{target}: {architecture}; extracted --version: {expected}')
        report(f'{target}: release commit {sha}; workflow run {os.environ.get("GITHUB_RUN_ID", "local")}')
        report(f'{target}: {archive.name}; sole entry {executable}; SHA-256 {hashlib.sha256(archive.read_bytes()).hexdigest()}')
        return archive
    except ReleaseError as error:
        if str(error).startswith(f'{target}:'):
            raise
        raise ReleaseError(f'{target}: binary packaging failed: {error}') from error
    except (OSError, ValueError, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        raise ReleaseError(f'{target}: binary packaging failed: {error}') from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True)
    parser.add_argument('--sha', required=True)
    parser.add_argument('--root', default='.')
    parser.add_argument('--output', default='release-assets')
    arguments = parser.parse_args()
    try:
        package(arguments.root, arguments.target, arguments.output, arguments.sha)
        return 0
    except ReleaseError as error:
        print(f'Release packaging failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
