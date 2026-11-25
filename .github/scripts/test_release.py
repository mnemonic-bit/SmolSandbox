"""Exercise release decisions against a recorded, stateful GitHub API fake."""

import base64
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest import mock
import urllib.error
from urllib.parse import parse_qs, urlsplit


SCRIPT = Path(__file__).with_name("release.py")
ROOT = SCRIPT.parents[2]
spec = importlib.util.spec_from_file_location("release", SCRIPT)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)

SHA = "1" * 40
OTHER_SHA = "2" * 40
TAG = "v0.11.0"


def manifest(version="0.11.0"):
    return f'[package]\nname = "smolsandbox"\nversion = {json.dumps(version)}\n'


def lock(version="0.11.0"):
    return f'version = 4\n[[package]]\nname = "smolsandbox"\nversion = {json.dumps(version)}\n'


class FakeAPI:
    def __init__(self):
        self.calls = []
        self.files = {SHA: {"Cargo.toml": manifest(), "Cargo.lock": lock()}}
        self.tags = {}
        self.annotations = {}
        self.releases = []
        self.hooks = []
        self.next_id = 1
        self.publication_happened = False
        self.list_missing_reads = 0
        self.direct_missing_reads = 0
        self.direct_draft_reads = 0
        self.tag_missing_reads = 0

    def add_release(self, tag=TAG, draft=False, body="Retained release notes", **fields):
        item = {
            "id": self.next_id,
            "tag_name": tag,
            "name": tag,
            "body": body,
            "draft": draft,
            "prerelease": False,
            "html_url": f"https://github.example/owner/repo/releases/{self.next_id}",
        }
        item.update(fields)
        self.next_id += 1
        self.releases.append(item)
        return item

    def at_next(self, method, path, callback):
        self.hooks.append((method, path, callback))

    def error_next(self, method, path, status):
        def fail(api, data):
            raise release.APIError(status, "Simulated GitHub API failure")
        self.at_next(method, path, fail)

    @property
    def writes(self):
        return [call for call in self.calls if call[0] != "GET"]

    def request(self, method, path, data=None, allow_missing=False):
        self.calls.append((method, path, copy.deepcopy(data)))
        for index, (expected_method, expected_path, callback) in enumerate(self.hooks):
            if (method, path.split("?")[0]) == (expected_method, expected_path):
                self.hooks.pop(index)
                result = callback(self, data)
                if result is not None:
                    return copy.deepcopy(result)
                break
        url = urlsplit(path)
        if method == "GET" and url.path.startswith("/contents/"):
            sha = parse_qs(url.query)["ref"][0]
            source = self.files[sha][url.path.removeprefix("/contents/")]
            return {"encoding": "base64", "content": base64.b64encode(source.encode()).decode()}
        if method == "GET" and url.path.startswith("/git/ref/tags/"):
            tag = url.path.removeprefix("/git/ref/tags/")
            if self.tag_missing_reads:
                self.tag_missing_reads -= 1
                if allow_missing:
                    return None
                raise release.APIError(404, "Tag temporarily absent")
            if tag in self.tags:
                return {"ref": "refs/tags/" + tag, "object": copy.deepcopy(self.tags[tag])}
            if allow_missing:
                return None
            raise release.APIError(404, "Tag absent")
        if method == "GET" and url.path.startswith("/git/tags/"):
            return {"object": copy.deepcopy(self.annotations[url.path.removeprefix("/git/tags/")])}
        if method == "GET" and url.path == "/releases":
            if self.publication_happened and self.list_missing_reads:
                self.list_missing_reads -= 1
                return []
            query = parse_qs(url.query)
            page = int(query.get("page", ["1"])[0])
            size = int(query.get("per_page", ["30"])[0])
            return copy.deepcopy(self.releases[(page - 1) * size:page * size])
        if method == "GET" and re.fullmatch(r"/releases/[0-9]+", url.path):
            if self.direct_missing_reads:
                self.direct_missing_reads -= 1
                if allow_missing:
                    return None
                raise release.APIError(404, "Release temporarily absent")
            for item in self.releases:
                if item["id"] == int(url.path.rsplit("/", 1)[1]):
                    visible = copy.deepcopy(item)
                    if self.direct_draft_reads:
                        self.direct_draft_reads -= 1
                        visible["draft"] = True
                    return visible
            if allow_missing:
                return None
            raise release.APIError(404, "Release absent")
        if method == "POST" and url.path == "/git/refs":
            tag = data["ref"].removeprefix("refs/tags/")
            if tag in self.tags:
                raise release.APIError(422, "Reference already exists")
            self.tags[tag] = {"type": "commit", "sha": data["sha"]}
            return {"ref": data["ref"], "object": copy.deepcopy(self.tags[tag])}
        if method == "POST" and url.path == "/releases/generate-notes":
            return {"name": "Generated title", "body": "Generated release notes"}
        if method == "POST" and url.path == "/releases":
            if any(item["tag_name"] == data["tag_name"] for item in self.releases):
                raise release.APIError(422, "Release already exists")
            item = self.add_release(**{key: value for key, value in data.items() if key != "tag_name"}, tag=data["tag_name"])
            self.publication_happened = True
            return copy.deepcopy(item)
        if method == "PATCH" and url.path.startswith("/releases/"):
            for item in self.releases:
                if item["id"] == int(url.path.rsplit("/", 1)[1]):
                    item.update(copy.deepcopy(data))
                    self.publication_happened = True
                    return copy.deepcopy(item)
        raise AssertionError(f"Unexpected API call: {method} {path} {data}")


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.api = FakeAPI()
        self.messages = []

    def run_release(self, mode="publish", sha=SHA):
        # These cases isolate metadata decisions; binary transfer has its own suite.
        with mock.patch.object(release, "load_binary_assets", return_value={}), mock.patch.object(release, "ensure_binary_assets"):
            return release.run(mode, self.api, sha, report=self.messages.append)

    def matching_tag(self, tag=TAG, sha=SHA):
        self.api.tags[tag] = {"type": "commit", "sha": sha}

    def assert_no_writes(self):
        self.assertEqual(self.api.writes, [])

    def delay_created_tag(self, missing_reads, status=None, sha=SHA, concurrent_release=False):
        def create_tag(api, data):
            api.tag_missing_reads = missing_reads
            if concurrent_release:
                api.add_release(body="Concurrent publication notes")
            if status is not None:
                api.tags[TAG] = {"type": "commit", "sha": sha}
                raise release.APIError(status, "Reference already exists")
            if sha != SHA:
                def conflict(api, data):
                    api.tags[TAG] = {"type": "commit", "sha": sha}
                api.at_next("GET", "/git/ref/tags/" + TAG, conflict)
        self.api.at_next("POST", "/git/refs", create_tag)

    def test_created_tag_retries_missing_reads_and_accepts_last_attempt(self):
        for missing_reads in (1, 2, 4):
            with self.subTest(missing_reads=missing_reads):
                self.api = FakeAPI()
                self.delay_created_tag(missing_reads)
                with mock.patch.object(release.time, "sleep") as sleep:
                    self.assertEqual(self.run_release(), "published")
                self.assertEqual(sleep.call_args_list, [mock.call(1)] * missing_reads)
                self.assertEqual(self.api.tags[TAG], {"type": "commit", "sha": SHA})
                self.assertEqual(len(self.api.releases), 1)
                self.assertEqual(sum(path == "/git/refs" for _, path, _ in self.api.writes), 1)
                self.assertTrue(all(method == "POST" for method, _, _ in self.api.writes))

    def test_tag_creation_races_retry_missing_reads_without_repeating_creation(self):
        for status in (409, 422):
            for missing_reads in (1, 4):
                with self.subTest(status=status, missing_reads=missing_reads):
                    self.api = FakeAPI()
                    self.delay_created_tag(missing_reads, status)
                    with mock.patch.object(release.time, "sleep") as sleep:
                        self.assertEqual(self.run_release(), "published")
                    self.assertEqual(sleep.call_args_list, [mock.call(1)] * missing_reads)
                    self.assertEqual(sum(path == "/git/refs" for _, path, _ in self.api.writes), 1)
                    self.assertEqual(len(self.api.releases), 1)
                    self.assertEqual(self.api.tags[TAG]["sha"], SHA)

    def test_visible_concurrent_release_does_not_bypass_or_prevent_tag_verification(self):
        for status in (None, 409, 422):
            with self.subTest(status=status):
                self.api = FakeAPI()
                self.delay_created_tag(2, status, concurrent_release=True)
                with mock.patch.object(release.time, "sleep") as sleep:
                    self.assertEqual(self.run_release(), "already-published")
                self.assertEqual(sleep.call_args_list, [mock.call(1)] * 2)
                self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])
                self.assertEqual(self.api.releases[0]["body"], "Concurrent publication notes")

    def test_created_tag_verification_exhaustion_preserves_tag_and_supports_rerun(self):
        for status in (None, 409, 422):
            for concurrent_release in (False, True):
                with self.subTest(status=status, concurrent_release=concurrent_release):
                    self.api = FakeAPI()
                    self.delay_created_tag(100, status, concurrent_release=concurrent_release)
                    with mock.patch.object(release, "load_binary_assets", return_value={}), mock.patch.object(release, "ensure_binary_assets") as ensure_assets, mock.patch.object(release.time, "sleep") as sleep:
                        with self.assertRaises(release.ReleaseError) as raised:
                            release.run("publish", self.api, SHA, report=self.messages.append)
                    for expected in (TAG, SHA, "5 attempts", "rerun the original workflow"):
                        self.assertIn(expected, str(raised.exception))
                    self.assertEqual(sleep.call_args_list, [mock.call(1)] * 4)
                    self.assertEqual(self.api.calls.count(("GET", "/git/ref/tags/" + TAG, None)), 6)
                    self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])
                    self.assertEqual(self.api.tags[TAG]["sha"], SHA)
                    self.assertEqual(len(self.api.releases), int(concurrent_release))
                    ensure_assets.assert_not_called()
                    self.api.tag_missing_reads = 0
                    self.api.calls.clear()
                    self.assertEqual(self.run_release(), "already-published" if concurrent_release else "published")
                    self.assertFalse(any(path == "/git/refs" for _, path, _ in self.api.writes))
                    before = copy.deepcopy((self.api.tags, self.api.releases))
                    self.api.calls.clear()
                    self.assertEqual(self.run_release(), "already-published")
                    self.assertEqual((self.api.tags, self.api.releases), before)
                    self.assert_no_writes()

    def test_created_tag_verification_rejects_conflicts_without_retrying_conflict(self):
        for status in (None, 409, 422):
            with self.subTest(status=status):
                self.api = FakeAPI()
                self.delay_created_tag(1, status, sha=OTHER_SHA)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaisesRegex(release.ReleaseError, "expected " + SHA):
                        self.run_release()
                self.assertEqual(sleep.call_args_list, [mock.call(1)])
                self.assertEqual(self.api.tags[TAG]["sha"], OTHER_SHA)
                self.assertEqual(self.api.releases, [])
                self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])

    def test_created_tag_verification_http_errors_fail_without_visibility_retry(self):
        for creation_status in (None, 409, 422):
            for read_status in (401, 403, 429, 500):
                with self.subTest(creation_status=creation_status, read_status=read_status):
                    self.api = FakeAPI()
                    def create_tag(api, data):
                        api.error_next("GET", "/git/ref/tags/" + TAG, read_status)
                        if creation_status is not None:
                            api.tags[TAG] = {"type": "commit", "sha": SHA}
                            raise release.APIError(creation_status, "Reference already exists")
                    self.api.at_next("POST", "/git/refs", create_tag)
                    with mock.patch.object(release.time, "sleep") as sleep:
                        with self.assertRaises(release.APIError) as raised:
                            self.run_release()
                    self.assertEqual(raised.exception.status, read_status)
                    sleep.assert_not_called()
                    self.assertEqual(self.api.releases, [])
                    self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])

    def test_created_tag_verification_malformed_metadata_fails_without_visibility_retry(self):
        invalid = [[], {}, {"object": None}, {"object": []}, {"object": {"type": "tree", "sha": SHA}}, {"object": {"type": "commit", "sha": 1}}]
        for response in invalid:
            with self.subTest(response=response):
                self.api = FakeAPI()
                def create_tag(api, data):
                    api.at_next("GET", "/git/ref/tags/" + TAG, lambda api, data: response)
                self.api.at_next("POST", "/git/refs", create_tag)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaises(release.ReleaseError):
                        self.run_release()
                sleep.assert_not_called()
                self.assertEqual(self.api.releases, [])
                self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])

    def test_delayed_created_annotated_tag_resolves_validated_commit(self):
        self.delay_created_tag(1, 422)
        def annotate(delay):
            self.api.tags[TAG] = {"type": "tag", "sha": "a" * 40}
            self.api.annotations["a" * 40] = {"type": "tag", "sha": "b" * 40}
            self.api.annotations["b" * 40] = {"type": "commit", "sha": SHA}
        with mock.patch.object(release.time, "sleep", side_effect=annotate) as sleep:
            self.assertEqual(self.run_release(), "published")
        sleep.assert_called_once_with(1)
        self.assertEqual(len(self.api.releases), 1)

    def test_created_annotated_tag_malformed_metadata_fails_without_visibility_retry(self):
        for annotation in ([], {}, {"object": None}, {"object": []}):
            with self.subTest(annotation=annotation):
                self.api = FakeAPI()
                def create_tag(api, data):
                    api.tags[TAG] = {"type": "tag", "sha": "a" * 40}
                    api.at_next("GET", "/git/tags/" + "a" * 40, lambda api, data: annotation)
                    raise release.APIError(422, "Reference already exists")
                self.api.at_next("POST", "/git/refs", create_tag)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaisesRegex(release.ReleaseError, "invalid tag metadata"):
                        self.run_release()
                sleep.assert_not_called()
                self.assertEqual(self.api.releases, [])
                self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])

    def test_conflict_after_created_tag_visibility_is_caught_before_publication(self):
        self.delay_created_tag(1)
        def schedule_conflict(delay):
            def match_then_conflict(api, data):
                def conflict(api, data):
                    api.tags[TAG] = {"type": "commit", "sha": OTHER_SHA}
                api.at_next("GET", "/git/ref/tags/" + TAG, conflict)
            self.api.at_next("GET", "/git/ref/tags/" + TAG, match_then_conflict)
        with mock.patch.object(release.time, "sleep", side_effect=schedule_conflict):
            with self.assertRaisesRegex(release.ReleaseError, "expected " + SHA):
                self.run_release()
        self.assertEqual(self.api.tags[TAG]["sha"], OTHER_SHA)
        self.assertEqual(self.api.releases, [])
        self.assertEqual([call[:2] for call in self.api.writes], [("POST", "/git/refs")])

    def test_new_release_uses_committed_version_exact_sha_and_generated_notes(self):
        self.run_release()
        self.assertEqual(self.api.tags[TAG], {"type": "commit", "sha": SHA})
        self.assertEqual(len(self.api.releases), 1)
        item = self.api.releases[0]
        self.assertEqual((item["tag_name"], item["name"], item["target_commitish"]), (TAG, TAG, SHA))
        self.assertEqual(item["body"], "Generated release notes")
        self.assertFalse(item["draft"])
        self.assertFalse(item["prerelease"])
        self.assertEqual(item["make_latest"], "legacy")
        self.assertIn(("POST", "/releases/generate-notes", {"tag_name": TAG, "target_commitish": SHA}), self.api.calls)
        for method, path, _ in self.api.calls:
            if path.startswith("/contents/"):
                self.assertEqual(parse_qs(urlsplit(path).query)["ref"], [SHA])
            self.assertNotIn(method, ("DELETE", "PUT"))

    def test_created_release_survives_post_write_list_omission(self):
        self.api.list_missing_reads = 1
        self.assertEqual(self.run_release(), "published")
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(self.api.tags[TAG], {"type": "commit", "sha": SHA})
        creations = [call for call in self.api.writes if call[:2] == ("POST", "/releases")]
        self.assertEqual(len(creations), 1)
        self.assertIn(("GET", "/releases/1", None), self.api.calls)

    def test_created_release_retries_direct_visibility_without_another_write(self):
        self.api.list_missing_reads = 100
        self.api.direct_missing_reads = 2
        with mock.patch.object(release.time, "sleep") as sleep:
            self.assertEqual(self.run_release(), "published")
        self.assertEqual(sleep.call_args_list, [mock.call(1), mock.call(1)])
        publication = next(index for index, call in enumerate(self.api.calls) if call[:2] == ("POST", "/releases"))
        verification = self.api.calls[publication + 1:]
        self.assertTrue(all(method == "GET" for method, _, _ in verification))
        self.assertFalse(any(path.startswith("/releases?") for _, path, _ in verification))
        self.assertEqual(verification.count(("GET", "/releases/1", None)), 3)
        self.assertEqual(verification.count(("GET", "/git/ref/tags/" + TAG, None)), 3)
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(self.api.releases[0]["body"], "Generated release notes")

    def test_draft_publication_retries_stale_visibility_and_preserves_notes(self):
        self.matching_tag()
        item = self.api.add_release(draft=True, body="Hand-edited draft notes")
        self.api.list_missing_reads = 100
        self.api.direct_draft_reads = 2
        with mock.patch.object(release.time, "sleep") as sleep:
            self.assertEqual(self.run_release(), "published")
        self.assertEqual(sleep.call_args_list, [mock.call(1), mock.call(1)])
        self.assertEqual(item["id"], 1)
        self.assertEqual(item["body"], "Hand-edited draft notes")
        self.assertFalse(item["draft"])
        self.assertEqual([call[:2] for call in self.api.writes], [("PATCH", "/releases/1")])
        self.assertEqual(self.api.calls.count(("GET", "/releases/1", None)), 3)

    def test_verification_limit_fails_without_repeating_publication(self):
        for visibility in ("direct_missing_reads", "direct_draft_reads"):
            with self.subTest(visibility=visibility):
                self.api = FakeAPI()
                setattr(self.api, visibility, 100)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaisesRegex(release.ReleaseError, "cannot be verified.*5.*rerun"):
                        self.run_release()
                self.assertEqual(self.api.calls.count(("GET", "/releases/1", None)), 5)
                self.assertEqual(sleep.call_args_list, [mock.call(1)] * 4)
                self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)
                self.assertEqual(len(self.api.releases), 1)
                self.assertFalse(self.api.releases[0]["draft"])

    def test_verification_http_errors_fail_without_retrying_or_republishing(self):
        for status in (401, 403, 429, 500):
            with self.subTest(status=status):
                self.api = FakeAPI()
                self.api.error_next("GET", "/releases/1", status)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaises(release.APIError) as raised:
                        self.run_release()
                self.assertEqual(raised.exception.status, status)
                sleep.assert_not_called()
                self.assertEqual(self.api.calls.count(("GET", "/releases/1", None)), 1)
                self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)

    def test_post_write_tag_conflict_fails_without_republishing(self):
        def change_tag(api, data):
            api.tags[TAG] = {"type": "commit", "sha": OTHER_SHA}
        self.api.at_next("POST", "/releases", change_tag)
        with mock.patch.object(release.time, "sleep") as sleep:
            with self.assertRaisesRegex(release.ReleaseError, "expected " + SHA):
                self.run_release()
        sleep.assert_not_called()
        self.assertEqual(self.api.tags[TAG]["sha"], OTHER_SHA)
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)

    def test_missing_post_write_tag_is_rechecked_within_verification_limit(self):
        def hide_tag(api, data):
            api.tags.pop(TAG)
            def schedule_restore(api, data):
                api.at_next("GET", "/git/ref/tags/" + TAG, restore_tag)
            def restore_tag(api, data):
                api.tags[TAG] = {"type": "commit", "sha": SHA}
            api.at_next("GET", "/git/ref/tags/" + TAG, schedule_restore)
        self.api.at_next("POST", "/releases", hide_tag)
        with mock.patch.object(release.time, "sleep") as sleep:
            self.assertEqual(self.run_release(), "published")
        sleep.assert_called_once_with(1)
        self.assertEqual(self.api.tags[TAG]["sha"], SHA)
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)

    def test_permanently_missing_post_write_tag_does_not_accept_release_response(self):
        def hide_tag(api, data):
            api.tags.pop(TAG)
        self.api.at_next("POST", "/releases", hide_tag)
        with mock.patch.object(release.time, "sleep") as sleep:
            with self.assertRaisesRegex(release.ReleaseError, "cannot be verified.*5.*rerun"):
                self.run_release()
        self.assertEqual(sleep.call_args_list, [mock.call(1)] * 4)
        self.assertNotIn(TAG, self.api.tags)
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)

    def test_publication_response_requires_valid_identity_and_normal_release(self):
        invalid = [
            [], {},
            {"id": True}, {"id": 0}, {"id": "1"},
            {"tag_name": "v9.0.0"},
            {"draft": True}, {"draft": 0}, {"draft": None},
            {"prerelease": True}, {"prerelease": 0}, {"prerelease": None},
        ]
        for changes in invalid:
            with self.subTest(changes=changes):
                self.api = FakeAPI()
                response = {"id": 1, "tag_name": TAG, "draft": False, "prerelease": False}
                if isinstance(changes, dict) and changes:
                    response.update(changes)
                else:
                    response = changes
                self.api.at_next("POST", "/releases", lambda api, data: response)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaises(release.ReleaseError):
                        self.run_release()
                sleep.assert_not_called()
                self.assertEqual(len([call for call in self.api.writes if call[:2] == ("POST", "/releases")]), 1)
                self.assertNotIn(("GET", "/releases/1", None), self.api.calls)

    def test_draft_publication_response_cannot_select_another_release_id(self):
        self.matching_tag()
        self.api.add_release(draft=True)
        self.api.at_next("PATCH", "/releases/1", lambda api, data: {
            "id": 2, "tag_name": TAG, "draft": False, "prerelease": False,
        })
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertEqual([call[:2] for call in self.api.writes], [("PATCH", "/releases/1")])
        self.assertFalse(any(path == "/releases/2" for _, path, _ in self.api.calls))

    def test_direct_verification_rejects_invalid_metadata(self):
        invalid = [
            [], {},
            {"id": 2}, {"id": True}, {"tag_name": "v9.0.0"},
            {"draft": 0}, {"draft": None},
            {"prerelease": True}, {"prerelease": None},
        ]
        for changes in invalid:
            with self.subTest(changes=changes):
                self.api = FakeAPI()
                response = {"id": 1, "tag_name": TAG, "draft": False, "prerelease": False}
                if isinstance(changes, dict) and changes:
                    response.update(changes)
                else:
                    response = changes
                self.api.at_next("GET", "/releases/1", lambda api, data: response)
                with mock.patch.object(release.time, "sleep") as sleep:
                    with self.assertRaises(release.ReleaseError):
                        self.run_release()
                sleep.assert_not_called()
                self.assertEqual(self.api.calls.count(("GET", "/releases/1", None)), 1)
                self.assertEqual(len(self.api.releases), 1)

    def test_rerun_after_delayed_verification_preserves_release_and_tag_without_writes(self):
        for draft in (False, True):
            with self.subTest(draft=draft):
                self.api = FakeAPI()
                if draft:
                    self.matching_tag()
                    self.api.add_release(draft=True, body="Retained draft notes")
                self.api.list_missing_reads = 1
                self.api.direct_missing_reads = 1
                with mock.patch.object(release.time, "sleep"):
                    self.assertEqual(self.run_release(), "published")
                before = copy.deepcopy((self.api.tags, self.api.releases))
                self.api.calls.clear()
                self.api.list_missing_reads = 0
                self.assertEqual(self.run_release(), "already-published")
                self.assertEqual((self.api.tags, self.api.releases), before)
                self.assert_no_writes()

    def test_pr_with_unused_version_passes_without_writes(self):
        self.run_release("eligibility-pr")
        self.assert_no_writes()
        self.assertEqual(self.api.tags, {})
        self.assertEqual(self.api.releases, [])

    def test_main_check_with_unused_version_passes_without_writes(self):
        self.run_release("eligibility-main")
        self.assert_no_writes()

    def test_invalid_version_metadata_never_writes(self):
        cases = [
            ('[package]\nname="smolsandbox"\n', lock()),
            (manifest(), 'version = 4\n'),
            (manifest(), lock("0.10.0")),
            ('[package]\nversion =', lock()),
            (manifest(), '[[package]\nname ='),
            (manifest("01.11.0"), lock("01.11.0")),
            (manifest("0.011.0"), lock("0.011.0")),
            (manifest("0.11.00"), lock("0.11.00")),
            (manifest("0.11"), lock("0.11")),
            (manifest("0.11.0-beta.1"), lock("0.11.0-beta.1")),
            (manifest("0.11.0+meta"), lock("0.11.0+meta")),
            (manifest("-1.11.0"), lock("-1.11.0")),
            (manifest("0.11.0\n"), lock("0.11.0\n")),
            (manifest("０.11.0"), lock("０.11.0")),
            ('[package]\nname="smolsandbox"\nversion=11\n', lock()),
            (manifest(), lock() + lock().split("version = 4\n", 1)[1]),
            (manifest(), lock() + 'source="registry+https://example.invalid"\n'),
        ]
        for source, locked in cases:
            with self.subTest(source=source, lock=locked):
                self.api = FakeAPI()
                self.api.files[SHA] = {"Cargo.toml": source, "Cargo.lock": locked}
                with self.assertRaises((release.ReleaseError, ValueError)):
                    self.run_release()
                self.assert_no_writes()

    def test_parser_accepts_zero_and_large_components(self):
        for version in ("0.0.0", "10.200.3000"):
            with self.subTest(version=version):
                self.assertEqual(release.parse_version(manifest(version), lock(version)), version)

    def test_pr_rejects_tag_even_at_synthetic_merge_sha(self):
        self.matching_tag()
        with self.assertRaises(release.ReleaseError):
            self.run_release("eligibility-pr")
        self.assert_no_writes()

    def test_pr_rejects_draft_and_published_releases_without_tags(self):
        for draft in (True, False):
            with self.subTest(draft=draft):
                self.api = FakeAPI()
                self.api.add_release(draft=draft)
                with self.assertRaises(release.ReleaseError):
                    self.run_release("eligibility-pr")
                self.assert_no_writes()

    def test_pr_finds_conflicting_draft_after_first_release_page(self):
        for index in range(100):
            self.api.add_release(tag=f"v9.0.{index}")
        self.api.add_release(draft=True)
        with self.assertRaises(release.ReleaseError):
            self.run_release("eligibility-pr")
        self.assertIn(("GET", "/releases?per_page=100&page=2", None), self.api.calls)
        self.assert_no_writes()

    def test_matching_tag_without_release_creates_only_missing_release(self):
        self.matching_tag()
        self.run_release()
        self.assertFalse(any(path == "/git/refs" for _, path, _ in self.api.writes))
        self.assertEqual(len(self.api.releases), 1)

    def test_matching_tag_is_valid_for_main_eligibility(self):
        self.matching_tag()
        self.run_release("eligibility-main")
        self.assert_no_writes()

    def test_matching_published_release_is_preserved_byte_for_byte(self):
        self.matching_tag()
        self.api.add_release(name="Retained title", body="Hand-edited release notes")
        before = copy.deepcopy(self.api.releases)
        self.run_release()
        self.assert_no_writes()
        self.assertEqual(self.api.releases, before)

    def test_matching_draft_retains_existing_notes_and_publishes_normal_release(self):
        self.matching_tag()
        item = self.api.add_release(draft=True)
        self.run_release()
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(item["body"], "Retained release notes")
        self.assertFalse(item["draft"])
        self.assertFalse(item["prerelease"])
        self.assertEqual(item["name"], TAG)
        self.assertEqual(item["make_latest"], "legacy")
        self.assertEqual([path for _, path, _ in self.api.writes], ["/releases/1"])

    def test_matching_prerelease_draft_recovers_as_normal_release(self):
        self.matching_tag()
        item = self.api.add_release(draft=True, prerelease=True)
        self.run_release()
        self.assertFalse(item["draft"])
        self.assertFalse(item["prerelease"])
        self.assertEqual(item["body"], "Retained release notes")

    def test_empty_draft_notes_are_generated_before_publication(self):
        self.matching_tag()
        item = self.api.add_release(draft=True, body="")
        self.run_release()
        self.assertEqual(item["body"], "Generated release notes")
        self.assertFalse(item["draft"])
        self.assertEqual([path for _, path, _ in self.api.writes], ["/releases/generate-notes", "/releases/1"])

    def test_conflicting_tag_fails_without_writes_in_main_and_publish(self):
        self.matching_tag(sha=OTHER_SHA)
        self.api.add_release()
        original = copy.deepcopy((self.api.tags, self.api.releases))
        for mode in ("eligibility-main", "publish"):
            with self.subTest(mode=mode):
                with self.assertRaises(release.ReleaseError):
                    self.run_release(mode)
                self.assert_no_writes()
                self.assertEqual((self.api.tags, self.api.releases), original)

    def test_release_without_verifiable_tag_is_preserved(self):
        for draft in (False, True):
            with self.subTest(draft=draft):
                self.api = FakeAPI()
                self.api.add_release(draft=draft)
                before = copy.deepcopy(self.api.releases)
                with self.assertRaises(release.ReleaseError):
                    self.run_release()
                self.assert_no_writes()
                self.assertEqual(self.api.releases, before)

    def test_nested_annotated_tag_resolves_to_validated_commit(self):
        self.api.tags[TAG] = {"type": "tag", "sha": "a" * 40}
        self.api.annotations["a" * 40] = {"type": "tag", "sha": "b" * 40}
        self.api.annotations["b" * 40] = {"type": "commit", "sha": SHA}
        self.run_release()
        self.assertEqual(len(self.api.releases), 1)
        self.assertFalse(any(path == "/git/refs" for _, path, _ in self.api.writes))

    def test_annotated_tag_pointing_elsewhere_fails_without_writes(self):
        self.api.tags[TAG] = {"type": "tag", "sha": "a" * 40}
        self.api.annotations["a" * 40] = {"type": "commit", "sha": OTHER_SHA}
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assert_no_writes()

    def test_tag_pointing_to_noncommit_fails_without_writes(self):
        self.api.tags[TAG] = {"type": "tree", "sha": SHA}
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assert_no_writes()

    def test_existing_prerelease_is_not_reported_as_normal_published_release(self):
        self.matching_tag()
        self.api.add_release(prerelease=True)
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assert_no_writes()

    def test_lookup_errors_fail_without_writes(self):
        for path in ("/contents/Cargo.toml", "/contents/Cargo.lock", "/git/ref/tags/" + TAG, "/releases"):
            for status in (401, 403, 429, 500):
                with self.subTest(path=path, status=status):
                    self.api = FakeAPI()
                    self.api.error_next("GET", path, status)
                    with self.assertRaises((release.APIError, release.ReleaseError)):
                        self.run_release()
                    self.assert_no_writes()

    def test_ruleset_rejection_does_not_create_release_or_remove_tag(self):
        self.api.error_next("POST", "/git/refs", 403)
        with self.assertRaises((release.APIError, release.ReleaseError)):
            self.run_release()
        self.assertEqual(self.api.tags, {})
        self.assertEqual(self.api.releases, [])
        self.assertEqual([path for _, path, _ in self.api.writes], ["/git/refs"])

    def test_failed_release_creation_leaves_tag_for_recovery(self):
        self.api.error_next("POST", "/releases", 500)
        with self.assertRaises((release.APIError, release.ReleaseError)):
            self.run_release()
        self.assertEqual(self.api.tags[TAG], {"type": "commit", "sha": SHA})
        self.assertEqual(self.api.releases, [])
        self.api.calls.clear()
        self.run_release()
        self.assertEqual(len(self.api.releases), 1)
        self.assertFalse(any(path == "/git/refs" for _, path, _ in self.api.writes))

    def test_tag_creation_race_converges_only_for_matching_commit(self):
        for racing_sha in (SHA, OTHER_SHA):
            with self.subTest(racing_sha=racing_sha):
                self.api = FakeAPI()
                def create_tag(api, data):
                    api.tags[TAG] = {"type": "commit", "sha": racing_sha}
                    raise release.APIError(422, "Reference already exists")
                self.api.at_next("POST", "/git/refs", create_tag)
                if racing_sha == SHA:
                    self.run_release()
                    self.assertEqual(len(self.api.releases), 1)
                else:
                    with self.assertRaises(release.ReleaseError):
                        self.run_release()
                    self.assertEqual(self.api.releases, [])
                self.assertEqual(self.api.tags[TAG]["sha"], racing_sha)

    def test_release_creation_race_preserves_matching_server_release(self):
        self.matching_tag()
        def create_release(api, data):
            api.add_release(body="Concurrent publication notes")
            raise release.APIError(422, "Release already exists")
        self.api.at_next("POST", "/releases", create_release)
        self.run_release()
        self.assertEqual(len(self.api.releases), 1)
        self.assertEqual(self.api.releases[0]["body"], "Concurrent publication notes")
        self.assertFalse(any(method == "PATCH" for method, _, _ in self.api.writes))

    def test_conflicting_release_creation_race_fails_without_modifying_racing_release(self):
        self.matching_tag()
        def create_conflict(api, data):
            api.tags[TAG] = {"type": "commit", "sha": OTHER_SHA}
            api.add_release(body="Unrelated publication")
            raise release.APIError(422, "Release already exists")
        self.api.at_next("POST", "/releases", create_conflict)
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertEqual(self.api.releases[0]["body"], "Unrelated publication")
        self.assertFalse(any(method == "PATCH" for method, _, _ in self.api.writes))

    def test_duplicate_error_without_created_state_is_failure(self):
        for path in ("/git/refs", "/releases"):
            with self.subTest(path=path):
                self.api = FakeAPI()
                self.api.error_next("POST", path, 422)
                with mock.patch.object(release.time, "sleep"), self.assertRaises((release.APIError, release.ReleaseError)):
                    self.run_release()
                self.assertEqual(self.api.releases, [])

    def test_different_versions_keep_own_sha_when_older_attempt_finishes_last(self):
        self.api.files[OTHER_SHA] = {"Cargo.toml": manifest("0.12.0"), "Cargo.lock": lock("0.12.0")}
        self.run_release("eligibility-main", SHA)
        self.run_release("eligibility-main", OTHER_SHA)
        self.run_release("publish", OTHER_SHA)
        self.run_release("publish", SHA)
        self.assertEqual(self.api.tags[TAG]["sha"], SHA)
        self.assertEqual(self.api.tags["v0.12.0"]["sha"], OTHER_SHA)
        self.assertEqual(len(self.api.releases), 2)
        self.assertTrue(all(item["make_latest"] == "legacy" for item in self.api.releases))

    def test_conflict_appearing_after_eligibility_is_caught_before_publication(self):
        self.run_release("eligibility-pr")
        self.run_release("eligibility-main")
        self.matching_tag(sha=OTHER_SHA)
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assert_no_writes()

    def test_successful_rerun_creates_no_duplicate_and_preserves_release(self):
        self.run_release()
        before = copy.deepcopy((self.api.tags, self.api.releases))
        self.api.calls.clear()
        self.run_release()
        self.assertEqual((self.api.tags, self.api.releases), before)
        self.assert_no_writes()


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.api = release.GitHubAPI("test-token", "owner/repository")

    def test_only_explicit_optional_404_means_absent(self):
        for status in (401, 403, 404, 409, 422, 429, 500, 503):
            for optional in (False, True):
                with self.subTest(status=status, optional=optional):
                    error = urllib.error.HTTPError("https://api.github.example", status, "failure", {}, io.BytesIO(b"API failed"))
                    with mock.patch("urllib.request.urlopen", side_effect=error):
                        if status == 404 and optional:
                            self.assertIsNone(self.api.request("GET", "/git/ref/tags/v0.11.0", allow_missing=True))
                        else:
                            with self.assertRaises(release.APIError) as raised:
                                self.api.request("GET", "/git/ref/tags/v0.11.0", allow_missing=optional)
                            self.assertEqual(raised.exception.status, status)

    def test_transport_sends_token_json_and_api_version(self):
        with mock.patch("urllib.request.urlopen") as open_url:
            open_url.return_value.__enter__.return_value = io.BytesIO(b'{"ok":true}')
            self.assertEqual(self.api.request("POST", "/git/refs", {"ref": "refs/tags/v0.11.0", "sha": SHA}), {"ok": True})
            request = open_url.call_args.args[0]
            self.assertEqual(request.full_url, "https://api.github.com/repos/owner/repository/git/refs")
            self.assertEqual(request.get_header("Authorization"), "Bearer test-token")
            self.assertEqual(request.get_header("X-github-api-version"), "2022-11-28")
            self.assertEqual(json.loads(request.data), {"ref": "refs/tags/v0.11.0", "sha": SHA})

    def test_network_and_invalid_json_fail(self):
        for failure in (urllib.error.URLError("offline"), TimeoutError("timed out")):
            with self.subTest(failure=str(failure)), mock.patch("urllib.request.urlopen", side_effect=failure):
                with self.assertRaises(release.ReleaseError):
                    self.api.request("GET", "/releases")
        with mock.patch("urllib.request.urlopen") as open_url:
            open_url.return_value.__enter__.return_value = io.BytesIO(b"not JSON")
            with self.assertRaises(release.ReleaseError):
                self.api.request("GET", "/releases")

    def test_cli_failure_exits_nonzero_and_writes_summary_without_network(self):
        with tempfile.TemporaryDirectory() as directory:
            summary = Path(directory) / "summary.md"
            environment = dict(os.environ, GITHUB_TOKEN="", GITHUB_REPOSITORY="owner/repository", RELEASE_MODE="eligibility-pr", RELEASE_SHA=SHA, GITHUB_STEP_SUMMARY=str(summary))
            result = subprocess.run([sys.executable, str(SCRIPT)], env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 1)
            self.assertIn("GITHUB_TOKEN is required", result.stdout)
            self.assertIn("Release check failed", summary.read_text())

    def test_conflict_summary_contains_candidate_tag_url_and_failure(self):
        api = FakeAPI()
        api.tags[TAG] = {"type": "commit", "sha": OTHER_SHA}
        item = api.add_release()
        with tempfile.TemporaryDirectory() as directory:
            summary = Path(directory) / "summary.md"
            environment = dict(os.environ, GITHUB_TOKEN="test-token", GITHUB_REPOSITORY="owner/repository", RELEASE_MODE="eligibility-pr", RELEASE_SHA=SHA, GITHUB_STEP_SUMMARY=str(summary))
            with mock.patch.dict(os.environ, environment, clear=True), mock.patch.object(release, "GitHubAPI", return_value=api), mock.patch("sys.stdout", new_callable=io.StringIO):
                self.assertEqual(release.main(), 1)
            text = summary.read_text()
            for expected in ("Candidate version: 0.11.0", "Selected tag: v0.11.0", item["html_url"], "Release check failed"):
                self.assertIn(expected, text)
            self.assertEqual(api.writes, [])


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / ".github/workflows/rust.yml").read_text()
        source = cls.workflow.split("jobs:\n", 1)[1]
        jobs = list(re.finditer(r"(?m)^  ([a-z][a-z-]*):\n", source))
        cls.jobs = {
            item.group(1): source[item.end():jobs[index + 1].start() if index + 1 < len(jobs) else len(source)]
            for index, item in enumerate(jobs)
        }

    def guard(self, job, event, ref, eligibility="skipped", native="skipped", cancelled=False, base="main"):
        source = self.jobs[job]
        match = re.search(r"(?m)^    if: >-\n((?:      .*\n)+)", source)
        self.assertIsNotNone(match, f"Missing job guard for {job}")
        expression = " ".join(match.group(1).strip().removeprefix("${{").removesuffix("}}").split())
        values = {
            "github.event_name": event,
            "github.ref": ref,
            "github.base_ref": base,
            "needs.release-eligibility.result": eligibility,
            "needs.native.result": native,
        }
        expression = expression.replace("always()", "True").replace("cancelled()", str(cancelled))
        def replace(match):
            token = match.group(0)
            if token.startswith("'") or token in ("True", "False"):
                return token
            self.assertIn(token, values, f"Unknown workflow expression token: {token}")
            return repr(values[token])
        expression = re.sub(r"'[^']*'|[a-zA-Z_][a-zA-Z0-9_.-]*", replace, expression)
        expression = expression.replace("&&", " and ").replace("||", " or ")
        expression = re.sub(r"!(?!=)", " not ", expression)
        return bool(eval(expression, {"__builtins__": {}}, {}))

    def test_native_job_guard_excludes_prs_feature_refs_tags_and_failed_main_eligibility(self):
        for event in ("push", "pull_request", "workflow_dispatch", "release"):
            for ref in ("refs/heads/main", "refs/heads/develop", "refs/heads/feature", "refs/tags/v0.11.0", "refs/pull/1/merge"):
                for eligibility in ("success", "failure", "skipped", "cancelled"):
                    with self.subTest(event=event, ref=ref, eligibility=eligibility):
                        expected = event in ("push", "workflow_dispatch") and ref in ("refs/heads/main", "refs/heads/develop")
                        if event == "push" and ref == "refs/heads/main":
                            expected = expected and eligibility == "success"
                        self.assertEqual(self.guard("native", event, ref, eligibility), expected)
        self.assertFalse(self.guard("native", "push", "refs/heads/main", "success", cancelled=True))

    def test_release_guard_requires_main_push_and_all_validation_success(self):
        for event in ("push", "pull_request", "workflow_dispatch", "release"):
            for ref in ("refs/heads/main", "refs/heads/develop", "refs/heads/feature", "refs/tags/v0.11.0"):
                for eligibility in ("success", "failure", "skipped", "cancelled"):
                    for native in ("success", "failure", "skipped", "cancelled"):
                        with self.subTest(event=event, ref=ref, eligibility=eligibility, native=native):
                            expected = event == "push" and ref == "refs/heads/main" and eligibility == native == "success"
                            self.assertEqual(self.guard("release", event, ref, eligibility, native), expected)
        self.assertIn("needs: [release-eligibility, native]", self.jobs["release"])
        self.assertIn("needs: [release-eligibility]", self.jobs["native"])

    def test_eligibility_runs_only_main_prs_and_main_pushes_without_dependencies(self):
        for event in ("push", "pull_request", "workflow_dispatch"):
            for ref in ("refs/heads/main", "refs/heads/develop", "refs/pull/1/merge"):
                for base in ("main", "develop"):
                    with self.subTest(event=event, ref=ref, base=base):
                        expected = (event == "push" and ref == "refs/heads/main") or (event == "pull_request" and base == "main")
                        self.assertEqual(self.guard("release-eligibility", event, ref, base=base), expected)
        metadata = self.jobs["release-eligibility"]
        self.assertIn("name: Release version eligibility", metadata)
        for excluded in ("    needs:", "uses:", "cargo ", "pip install", "subprocess", "pull_request_target", "github.event.pull_request.draft"):
            self.assertNotIn(excluded, metadata)

    def test_trigger_branches_pr_activities_and_permission_scopes(self):
        triggers = self.workflow.split("permissions:\n", 1)[0]
        self.assertRegex(triggers, r"push:\n    branches: \[main, develop\]")
        self.assertRegex(triggers, r"pull_request:\n    branches: \[main\]")
        self.assertIn("types: [opened, reopened, synchronize, edited, ready_for_review]", triggers)
        for excluded in ("paths:", "tags:", "pull_request_target", "concurrency:"):
            self.assertNotIn(excluded, self.workflow)
        self.assertIn("permissions:\n  contents: read", self.workflow)
        self.assertEqual(self.workflow.count("contents: write"), 1)
        self.assertIn("contents: write", self.jobs["release"])
        self.assertIn("GITHUB_TOKEN: ${{ github.token }}", self.jobs["release"])
        self.assertEqual(self.workflow.count("target: x86_64-unknown-linux-gnu"), 1)
        self.assertEqual(self.workflow.count("target: aarch64-unknown-linux-gnu"), 1)
        self.assertEqual(self.workflow.count("target: aarch64-apple-darwin"), 1)
        self.assertEqual(self.workflow.count("target: x86_64-pc-windows-msvc"), 1)
        self.assertEqual(self.workflow.count("target: aarch64-pc-windows-msvc"), 1)
        self.assertIn("cargo build --locked --release --bin smolsbx --target ${{ matrix.target }}", self.jobs["native"])
        self.assertIn("ref: ${{ github.sha }}", self.jobs["release"])
        self.assertIn("ref: ${{ github.sha }}", self.jobs["native"])
        self.assertIn("RELEASE_SHA: ${{ github.sha }}", self.workflow)

    def test_native_matrix_has_exact_five_runner_target_pairs(self):
        pairs = re.findall(r"- runner: ([^\n]+)\n\s+target: ([^\n]+)", self.jobs["native"])
        expected = {
            ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
            ("ubuntu-24.04-arm", "aarch64-unknown-linux-gnu"),
            ("macos-15", "aarch64-apple-darwin"),
            ("windows-2025", "x86_64-pc-windows-msvc"),
            ("windows-11-arm", "aarch64-pc-windows-msvc"),
        }
        self.assertEqual(len(pairs), 5)
        self.assertEqual(set(pairs), expected)
        self.assertNotIn("continue-on-error:", self.jobs["native"])

    def test_arm64_native_host_is_selected_and_verified_before_full_check_sequence(self):
        native = self.jobs["native"]
        self.assertIn("SMOLSBX_EXPECTED_RUST_HOST: ${{ matrix.target }}", native)
        self.assertIn("rustup toolchain install stable-aarch64-pc-windows-msvc --profile minimal --component rustfmt --component clippy", native)
        self.assertIn("echo 'RUSTUP_TOOLCHAIN=stable-aarch64-pc-windows-msvc' >> \"$GITHUB_ENV\"", native)
        for line in (
            'echo "Runner OS: $NATIVE_RUNNER_OS"',
            'echo "Runner architecture: $NATIVE_RUNNER_ARCH"',
            'echo "Runner image: ${ImageOS:-unknown} ${ImageVersion:-unknown}"',
            'rustc -vV | grep -Fx "host: $SMOLSBX_EXPECTED_RUST_HOST"',
            'cargo -vV | grep -Fx "host: $SMOLSBX_EXPECTED_RUST_HOST"',
            'test "$NATIVE_RUNNER_ARCH" = ARM64',
        ):
            self.assertIn(line, native)
        checks = (
            "cargo fmt --all -- --check",
            "cargo clippy --locked --all-targets --target ${{ matrix.target }} -- -D warnings",
            "cargo clippy --locked --features test-adapter --all-targets --target ${{ matrix.target }} -- -D warnings",
            "cargo test --locked --all-targets --target ${{ matrix.target }}",
            "cargo test --locked --features test-adapter --all-targets --target ${{ matrix.target }}",
            "cargo build --locked --release --bin smolsbx --target ${{ matrix.target }}",
            "cargo run --locked --example check_baseline_independence --target ${{ matrix.target }}",
        )
        positions = [native.index(command) for command in checks]
        self.assertEqual(positions, sorted(positions))
        self.assertLess(native.index('rustc -vV | grep -Fx'), positions[0])
        self.assertLess(native.index('RUSTUP_TOOLCHAIN=stable-aarch64-pc-windows-msvc'), positions[0])
        example = (ROOT / "examples/check_baseline_independence.rs").read_text()
        for expected in ('SMOLSBX_EXPECTED_RUST_HOST', 'cargo(root, &["-vV"])', 'baseline-independence {name} must use native host', '.env("PATH", "")'):
            self.assertIn(expected, example)

    def test_arm64_production_smoke_requires_offline_cli_and_preserved_state(self):
        native = self.jobs["native"]
        smoke = native.split("      - name: Check Windows ARM64 offline CLI and production host rejection\n", 1)[1].split("      - uses:", 1)[0]
        self.assertIn("if: matrix.target == 'aarch64-pc-windows-msvc'", smoke)
        self.assertIn("target/aarch64-pc-windows-msvc/release/smolsbx.exe", smoke)
        self.assertIn("PATH='', SMOLVM_BIN=str(workspace / 'missing-smolvm.exe')", smoke)
        for expected in ("['--help']", "['--version']", "['create-config', '--harness', 'codex'", "unsupported SmolVM host: windows/aarch64", "assert result.returncode == 1", "assert snapshot(native_state) == original_state", "assert all(not (workspace / name).exists()", "call == ['machine', 'ls', '--json']"):
            self.assertIn(expected, smoke)
        self.assertNotIn("SMOLVM_TEST_HOME", smoke)
        script = textwrap.dedent(smoke.split("          python - <<'PY'\n", 1)[1].split("          PY\n", 1)[0])
        compile(script, "ARM64 production smoke", "exec")

    def test_inline_eligibility_program_matches_canonical_script(self):
        workflow = (ROOT / ".github/workflows/rust.yml").read_text()
        match = re.search(r"(?m)^([ ]*)# BEGIN RELEASE SCRIPT\n(.*?)^\1# END RELEASE SCRIPT\s*$", workflow, re.DOTALL | re.MULTILINE)
        self.assertIsNotNone(match, "Workflow must delimit the inline eligibility program")
        inline = textwrap.dedent(match.group(2))
        self.assertEqual(inline, SCRIPT.read_text())
        compile(inline, "workflow eligibility program", "exec")


def record_api_traces(destination):
    instances = []
    active_test = [None]
    original = FakeAPI

    class RecordingAPI(original):
        def __init__(self):
            super().__init__()
            instances.append((active_test[0], self))

    class RecordingResult(unittest.TextTestResult):
        def startTest(self, test):
            active_test[0] = test.id()
            super().startTest(test)

    globals()["FakeAPI"] = RecordingAPI
    try:
        suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
        result = unittest.TextTestRunner(resultclass=RecordingResult, verbosity=1).run(suite)
    finally:
        globals()["FakeAPI"] = original
    payload = {
        "command": "python3 .github/scripts/test_release.py --record-api-traces <destination>",
        "result": "passed" if result.wasSuccessful() else "failed",
        "tests_run": result.testsRun,
        "release_sha": SHA,
        "other_sha": OTHER_SHA,
        "scenarios": [
            {"test": test, "calls": api.calls, "tags": api.tags, "releases": api.releases}
            for test, api in instances
        ],
    }
    Path(destination).write_text(json.dumps(payload, indent=2) + "\n")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--record-api-traces":
        sys.exit(record_api_traces(sys.argv[2]))
    unittest.main()
