"""Exercise binary release recovery against local ZIPs and a stateful API fake."""

import copy
import io
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock
import urllib.error
import urllib.request
from urllib.parse import parse_qs, urlsplit
import zipfile

from test_release import FakeAPI, SHA, TAG, release


TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


def archive(target, entries=None, mode=None):
    output = io.BytesIO()
    executable = "smolsbx.exe" if "windows" in target else "smolsbx"
    if entries is None:
        entries = [(executable, f"native executable for {target}".encode())]
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as package:
        for name, data in entries:
            item = zipfile.ZipInfo(name)
            item.create_system = 3
            item.external_attr = (mode if mode is not None else stat.S_IFREG | 0o755) << 16
            package.writestr(item, data)
    return output.getvalue()


class AssetAPI(FakeAPI):
    def __init__(self):
        super().__init__()
        self.assets = {}
        self.asset_bytes = {}
        self.next_asset_id = 100
        self.uploads = []
        self.upload_failure = None
        self.corrupt_upload = False

    def add_release(self, *args, **kwargs):
        item = super().add_release(*args, **kwargs)
        item["upload_url"] = f"https://uploads.github.com/repos/owner/repository/releases/{item['id']}/assets{{?name,label}}"
        return item

    def request(self, method, path, data=None, allow_missing=False):
        url = urlsplit(path)
        if method == "GET" and url.path.endswith("/assets"):
            self.calls.append((method, path, copy.deepcopy(data)))
            release_id = int(url.path.split("/")[2])
            query = parse_qs(url.query)
            page = int(query.get("page", ["1"])[0])
            size = int(query.get("per_page", ["100"])[0])
            return copy.deepcopy(self.assets.get(release_id, [])[(page - 1) * size:page * size])
        return super().request(method, path, data, allow_missing)

    def add_asset(self, release_id, name, data, **fields):
        item = {"id": self.next_asset_id, "name": name, "size": len(data), "state": "uploaded"}
        item.update(fields)
        self.next_asset_id += 1
        self.assets.setdefault(release_id, []).append(item)
        self.asset_bytes[item["id"]] = data
        return copy.deepcopy(item)

    def upload_asset(self, item, name, data):
        if self.upload_failure == name:
            raise release.APIError(503, "Simulated upload interruption")
        self.uploads.append((item["id"], name, data))
        stored = b"corrupt" if self.corrupt_upload else data
        return self.add_asset(item["id"], name, stored)

    def download_asset(self, asset_id):
        return self.asset_bytes[asset_id]


class BinaryReleaseTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.environment = mock.patch.dict(os.environ, RELEASE_ASSET_DIR=self.directory.name)
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.api = AssetAPI()
        self.messages = []
        self.packages = {}
        for target in TARGETS:
            name = f"smolsbx-{TAG}-{target}.zip"
            self.packages[name] = archive(target)
            (Path(self.directory.name) / name).write_bytes(self.packages[name])

    def run_release(self):
        return release.run("publish", self.api, SHA, report=self.messages.append)

    def matching_release(self):
        self.api.tags[TAG] = {"type": "commit", "sha": SHA}
        return self.api.add_release()

    def test_existing_published_release_recovers_all_missing_binary_zips(self):
        item = self.matching_release()
        before = copy.deepcopy(item)
        self.assertEqual(self.run_release(), "already-published")
        self.assertEqual({asset["name"] for asset in self.api.assets.get(item["id"], [])}, set(self.packages))
        self.assertEqual(len(self.api.uploads), 4)
        self.assertEqual(item, before)

    def test_new_release_uploads_exact_four_packages_after_verification(self):
        self.assertEqual(self.run_release(), "published")
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual({name: data for _, name, data in self.api.uploads}, self.packages)
        self.assertEqual(self.api.tags[TAG], {"type": "commit", "sha": SHA})
        self.assertTrue(any("Verified complete binary release" in message for message in self.messages))

    def test_partial_upload_failure_is_recovered_without_replacing_assets(self):
        item = self.matching_release()
        names = list(self.packages)
        self.api.upload_failure = names[2]
        with self.assertRaisesRegex(release.ReleaseError, names[2]):
            self.run_release()
        first = copy.deepcopy(self.api.assets[item["id"]])
        self.assertEqual(len(first), 2)
        self.assertFalse(any("Verified complete binary release" in message for message in self.messages))
        self.api.upload_failure = None
        self.assertEqual(self.run_release(), "already-published")
        self.assertEqual(self.api.assets[item["id"]][:2], first)
        self.assertEqual(len(self.api.uploads), 4)

    def test_complete_rerun_preserves_ids_bytes_notes_tag_and_performs_no_upload(self):
        item = self.matching_release()
        self.run_release()
        before = copy.deepcopy((item, self.api.assets, self.api.asset_bytes, self.api.tags))
        self.api.calls.clear()
        self.api.uploads.clear()
        self.assertEqual(self.run_release(), "already-published")
        self.assertEqual((item, self.api.assets, self.api.asset_bytes, self.api.tags), before)
        self.assertEqual(self.api.uploads, [])
        self.assertEqual(self.api.writes, [])

    def test_each_missing_package_fails_before_any_publication_write(self):
        for name, data in self.packages.items():
            with self.subTest(name=name):
                path = Path(self.directory.name) / name
                path.unlink()
                try:
                    with self.assertRaisesRegex(release.ReleaseError, name):
                        self.run_release()
                    self.assertEqual(self.api.writes, [])
                    self.assertEqual(self.api.uploads, [])
                finally:
                    path.write_bytes(data)

    def test_invalid_archives_and_nonexecutable_unix_modes_fail_preflight(self):
        target = TARGETS[0]
        name = f"smolsbx-{TAG}-{target}.zip"
        invalid = (
            b"not a ZIP",
            archive(target, []),
            archive(target, [("smolsbx", b"binary"), ("README.md", b"source")]),
            archive(target, [("nested/smolsbx", b"binary")]),
            archive(target, [("smolsbx/", b"")]),
            archive(target, [("smolsbx", b"")]),
            archive(target, mode=stat.S_IFREG | 0o644),
            archive(target, mode=stat.S_IFLNK | 0o777),
        )
        for data in invalid:
            with self.subTest(data=data[:30]):
                (Path(self.directory.name) / name).write_bytes(data)
                with self.assertRaisesRegex(release.ReleaseError, target):
                    self.run_release()
                self.assertEqual(self.api.writes, [])

    def test_unexpected_local_file_is_rejected_before_writes(self):
        (Path(self.directory.name) / "source.zip").write_bytes(b"source")
        with self.assertRaisesRegex(release.ReleaseError, "source.zip"):
            self.run_release()
        self.assertEqual(self.api.writes, [])

    def test_corrupt_deflate_payload_has_target_specific_error(self):
        target = TARGETS[0]
        name = f"smolsbx-{TAG}-{target}.zip"
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as package:
            item = zipfile.ZipInfo("smolsbx")
            item.create_system = 3
            item.external_attr = (stat.S_IFREG | 0o755) << 16
            package.writestr(item, b"executable" * 1000, compress_type=zipfile.ZIP_DEFLATED)
        data = bytearray(output.getvalue())
        offset = 30 + len("smolsbx")
        data[offset:offset + 8] = b"\xff" * 8
        (Path(self.directory.name) / name).write_bytes(data)
        with self.assertRaisesRegex(release.ReleaseError, target):
            self.run_release()
        self.assertEqual(self.api.writes, [])

    def test_missing_asset_directory_fails_before_writes(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(release.ReleaseError, "RELEASE_ASSET_DIR"):
                self.run_release()
        self.assertEqual(self.api.writes, [])

    def test_conflicting_bytes_fail_before_upload_and_preserve_existing_asset(self):
        item = self.matching_release()
        name = next(iter(self.packages))
        data = self.packages[name]
        asset = self.api.add_asset(item["id"], name, b"x" * len(data))
        before = copy.deepcopy((item, self.api.assets, self.api.asset_bytes))
        with self.assertRaisesRegex(release.ReleaseError, "conflicting contents"):
            self.run_release()
        self.assertEqual((item, self.api.assets, self.api.asset_bytes), before)
        self.assertEqual(self.api.uploads, [])
        self.assertEqual(self.api.asset_bytes[asset["id"]], b"x" * len(data))

    def test_conflicting_size_state_and_id_fail_without_replacement(self):
        for fields in ({"size": 1}, {"state": "starter"}, {"id": 0}, {"id": True}):
            with self.subTest(fields=fields):
                self.api = AssetAPI()
                item = self.matching_release()
                name = next(iter(self.packages))
                self.api.add_asset(item["id"], name, self.packages[name], **fields)
                with self.assertRaisesRegex(release.ReleaseError, name):
                    self.run_release()
                self.assertEqual(self.api.uploads, [])

    def test_unexpected_or_duplicate_remote_asset_fails_without_writes(self):
        item = self.matching_release()
        self.api.add_asset(item["id"], "source.zip", b"source")
        with self.assertRaisesRegex(release.ReleaseError, "Unexpected release assets.*source.zip"):
            self.run_release()
        self.assertEqual(self.api.uploads, [])
        name = next(iter(self.packages))
        self.api.assets.clear()
        self.api.add_asset(item["id"], name, self.packages[name])
        self.api.add_asset(item["id"], name, self.packages[name])
        with self.assertRaisesRegex(release.ReleaseError, "Duplicate release asset"):
            self.run_release()
        self.assertEqual(self.api.uploads, [])

    def test_asset_listing_reads_pages_and_detects_unexpected_later_asset(self):
        item = self.matching_release()
        for index in range(100):
            self.api.add_asset(item["id"], f"extra-{index}.zip", b"binary")
        self.api.add_asset(item["id"], "later-source.zip", b"source")
        with self.assertRaisesRegex(release.ReleaseError, "later-source.zip"):
            self.run_release()
        self.assertIn(("GET", "/releases/1/assets?per_page=100&page=2", None), self.api.calls)
        self.assertEqual(self.api.uploads, [])

    def test_each_upload_failure_names_target_and_never_reports_completion(self):
        for name in self.packages:
            with self.subTest(name=name):
                self.api = AssetAPI()
                self.matching_release()
                self.api.upload_failure = name
                self.messages.clear()
                with self.assertRaisesRegex(release.ReleaseError, name):
                    self.run_release()
                self.assertFalse(any("Verified complete binary release" in message for message in self.messages))

    def test_uploaded_bytes_are_read_back_and_conflicts_fail(self):
        self.matching_release()
        self.api.corrupt_upload = True
        with self.assertRaisesRegex(release.ReleaseError, "Binary upload failed"):
            self.run_release()
        self.assertEqual(len(self.api.uploads), 1)

    def test_duplicate_upload_race_accepts_only_identical_bytes(self):
        original_upload = self.api.upload_asset
        name = next(iter(self.packages))
        for corrupt in (False, True):
            with self.subTest(corrupt=corrupt):
                self.api = AssetAPI()
                self.matching_release()
                original_upload = self.api.upload_asset

                def racing_upload(item, asset_name, data):
                    if asset_name == name:
                        self.api.add_asset(item["id"], asset_name, b"x" * len(data) if corrupt else data)
                        raise release.APIError(422, "Duplicate name after racing upload")
                    return original_upload(item, asset_name, data)

                self.api.upload_asset = racing_upload
                if corrupt:
                    with self.assertRaisesRegex(release.ReleaseError, "conflicting contents"):
                        self.run_release()
                else:
                    self.assertEqual(self.run_release(), "already-published")
                    self.assertEqual(len(self.api.assets[1]), 4)

    def test_changed_tag_before_upload_fails_without_upload(self):
        self.matching_release()
        original_request = self.api.request

        def changing_request(method, path, data=None, allow_missing=False):
            result = original_request(method, path, data, allow_missing)
            if path.startswith("/releases/1/assets"):
                self.api.tags[TAG]["sha"] = "2" * 40
            return result

        self.api.request = changing_request
        with self.assertRaisesRegex(release.ReleaseError, "changed before binary upload"):
            self.run_release()
        self.assertEqual(self.api.uploads, [])

    def test_asset_missing_from_final_listing_names_target(self):
        item = self.matching_release()
        name = next(iter(self.packages))
        original_request = self.api.request
        listing_reads = 0

        def missing_request(method, path, data=None, allow_missing=False):
            nonlocal listing_reads
            result = original_request(method, path, data, allow_missing)
            if path.startswith("/releases/1/assets"):
                listing_reads += 1
                if listing_reads > 1:
                    return [asset for asset in result if asset["name"] != name]
            return result

        self.api.request = missing_request
        with self.assertRaisesRegex(release.ReleaseError, name):
            self.run_release()
        self.assertEqual(len(self.api.assets[item["id"]]), 4)


class BinaryTransportTests(unittest.TestCase):
    def setUp(self):
        self.api = release.GitHubAPI("test-token", "owner/repository")
        self.item = {"id": 1, "upload_url": "https://uploads.github.com/repos/owner/repository/releases/1/assets{?name,label}"}

    def test_upload_sends_raw_zip_to_validated_upload_url(self):
        with mock.patch("urllib.request.build_opener") as build:
            build.return_value.open.return_value.__enter__.return_value = io.BytesIO(b'{"id":123}')
            self.assertEqual(self.api.upload_asset(self.item, "binary target.zip", b"raw zip"), {"id": 123})
            request = build.return_value.open.call_args.args[0]
            self.assertEqual(request.full_url, "https://uploads.github.com/repos/owner/repository/releases/1/assets?name=binary+target.zip")
            self.assertEqual(request.get_header("Authorization"), "Bearer test-token")
            self.assertEqual(request.get_header("Content-type"), "application/zip")
            self.assertEqual(request.data, b"raw zip")

    def test_download_uses_authenticated_api_octet_stream(self):
        with mock.patch("urllib.request.build_opener") as build:
            build.return_value.open.return_value.__enter__.return_value = io.BytesIO(b"downloaded zip")
            self.assertEqual(self.api.download_asset(123), b"downloaded zip")
            request = build.return_value.open.call_args.args[0]
            self.assertEqual(request.full_url, "https://api.github.com/repos/owner/repository/releases/assets/123")
            self.assertEqual(request.get_header("Accept"), "application/octet-stream")
            self.assertEqual(request.get_header("Authorization"), "Bearer test-token")

    def test_server_upload_urls_cannot_send_credentials_to_another_host_or_release(self):
        for url in (
            "http://uploads.github.com/repos/owner/repository/releases/1/assets",
            "https://attacker.example/repos/owner/repository/releases/1/assets",
            "https://uploads.github.com.attacker.example/repos/owner/repository/releases/1/assets",
            "https://uploads.github.com/repos/owner/repository/releases/2/assets",
            "https://test-token@uploads.github.com/repos/owner/repository/releases/1/assets",
            "https://uploads.github.com/repos/owner/repository/releases/1/assets?redirect=evil",
        ):
            with self.subTest(url=url), mock.patch("urllib.request.build_opener") as build:
                with self.assertRaisesRegex(release.ReleaseError, "invalid upload URL"):
                    self.api.upload_asset(dict(self.item, upload_url=url), "binary.zip", b"binary")
                build.assert_not_called()

    def test_signed_storage_redirect_removes_bearer_token(self):
        handler = release.AssetRedirectHandler()
        request = urllib.request.Request("https://api.github.com/repos/owner/repository/releases/assets/123", headers={"Authorization": "Bearer secret", "Accept": "application/octet-stream"})
        redirected = handler.redirect_request(request, None, 302, "Found", {}, "https://release-assets.githubusercontent.com/signed-file")
        self.assertIsNone(redirected.get_header("Authorization"))
        self.assertEqual(redirected.get_header("Accept"), "application/octet-stream")
        self.assertEqual(request.get_header("Authorization"), "Bearer secret")
        with self.assertRaises(release.ReleaseError):
            handler.redirect_request(request, None, 302, "Found", {}, "http://release-assets.githubusercontent.com/file")

    def test_upload_redirect_is_rejected(self):
        request = urllib.request.Request("https://uploads.github.com/asset", data=b"zip", method="POST")
        with self.assertRaises(release.ReleaseError):
            release.AssetRedirectHandler().redirect_request(request, None, 302, "Found", {}, "https://other.example/file")

    def test_transfer_http_network_and_json_errors_fail(self):
        failures = (
            urllib.error.HTTPError("https://uploads.github.com", 503, "failure", {}, io.BytesIO(b"failed")),
            urllib.error.URLError("offline"),
            TimeoutError("timed out"),
        )
        for error in failures:
            with self.subTest(error=error), mock.patch("urllib.request.build_opener") as build:
                build.return_value.open.side_effect = error
                with self.assertRaises(release.ReleaseError):
                    self.api.upload_asset(self.item, "binary.zip", b"zip")
        with mock.patch("urllib.request.build_opener") as build:
            build.return_value.open.return_value.__enter__.return_value = io.BytesIO(b"invalid JSON")
            with self.assertRaises(release.ReleaseError):
                self.api.upload_asset(self.item, "binary.zip", b"zip")


if __name__ == "__main__":
    unittest.main()
