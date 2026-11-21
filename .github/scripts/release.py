"""Check release eligibility and publish one immutable GitHub release."""

import base64
import binascii
import hashlib
import html
import io
import json
import os
from pathlib import Path
import re
import stat
import sys
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import zipfile
import zlib


VERIFICATION_ATTEMPTS = 5
VERIFICATION_DELAY_SECONDS = 1
BINARY_TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


class ReleaseError(Exception):
    pass


class APIError(ReleaseError):
    def __init__(self, status, message):
        self.status = status
        super().__init__(f"GitHub API HTTP {status}: {message}")


class AssetRedirectHandler(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        if request.get_method() != "GET" or urllib.parse.urlsplit(new_url).scheme != "https":
            raise ReleaseError("Unexpected redirect while transferring a release asset")
        redirected = super().redirect_request(request, response, code, message, headers, new_url)
        if redirected is not None:
            # Asset downloads redirect to signed storage URLs without credentials.
            redirected.remove_header("Authorization")
        return redirected


class GitHubAPI:
    def __init__(self, token, repository, api_url="https://api.github.com"):
        if not token:
            raise ReleaseError("GITHUB_TOKEN is required")
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ReleaseError("GITHUB_REPOSITORY must identify owner/repository")
        self.base_url = f"{api_url.rstrip('/')}/repos/{repository}"
        self.repository = repository
        self.token = token

    def request(self, method, path, data=None, allow_missing=False):
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "Content-Type": "application/json",
            "User-Agent": "smolsandbox-release",
        }
        request = urllib.request.Request(
            self.base_url + path,
            data=None if data is None else json.dumps(data).encode("utf-8"),
            headers=headers,
            method=method,
        )
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if allow_missing and error.code == 404:
                return None
            detail = error.read(2000).decode("utf-8", errors="replace")
            raise APIError(error.code, f"{method} {path}: {detail}") from error
        except (urllib.error.URLError, TimeoutError, ValueError) as error:
            raise ReleaseError(f"GitHub API {method} {path} failed: {error}") from error

    def transfer_asset(self, method, url, data=None):
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/octet-stream" if method == "GET" else "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "smolsandbox-release",
        }
        if data is not None:
            headers["Content-Type"] = "application/zip"
        request = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            opener = urllib.request.build_opener(AssetRedirectHandler())
            with opener.open(request, timeout=60) as response:
                return response.read() if method == "GET" else json.load(response)
        except urllib.error.HTTPError as error:
            detail = error.read(2000).decode("utf-8", errors="replace")
            raise APIError(error.code, f"{method} release asset: {detail}") from error
        except (urllib.error.URLError, TimeoutError, ValueError, OSError) as error:
            raise ReleaseError(f"GitHub release asset {method} failed: {error}") from error

    def upload_asset(self, release, name, data):
        upload_url = release.get("upload_url")
        if not isinstance(upload_url, str):
            raise ReleaseError(f"Release has no upload URL for {name}")
        url = urllib.parse.urlsplit(upload_url.split("{", 1)[0])
        api_host = urllib.parse.urlsplit(self.base_url)
        allowed_hosts = {api_host.netloc}
        if api_host.hostname == "api.github.com":
            allowed_hosts.add("uploads.github.com")
        expected_path = f"/repos/{self.repository}/releases/{release['id']}/assets"
        if (url.scheme != "https" or url.netloc not in allowed_hosts or url.username is not None
                or url.query or url.fragment or not url.path.endswith(expected_path)):
            raise ReleaseError(f"Release has an invalid upload URL for {name}")
        destination = urllib.parse.urlunsplit(url) + "?" + urllib.parse.urlencode({"name": name})
        return self.transfer_asset("POST", destination, data)

    def download_asset(self, asset_id):
        return self.transfer_asset("GET", f"{self.base_url}/releases/assets/{asset_id}")


def load_binary_assets(tag):
    directory = os.environ.get("RELEASE_ASSET_DIR")
    if not directory:
        raise ReleaseError("RELEASE_ASSET_DIR is required for all four binary targets")
    root = Path(directory)
    assets = {}
    for target in BINARY_TARGETS:
        name = f"smolsbx-{tag}-{target}.zip"
        executable = "smolsbx.exe" if "windows" in target else "smolsbx"
        try:
            path = root / name
            if path.is_symlink() or not path.is_file():
                raise ReleaseError(f"Missing binary ZIP for {target}: {name}")
            data = path.read_bytes()
            with zipfile.ZipFile(io.BytesIO(data)) as package:
                entries = package.infolist()
                if len(entries) != 1 or entries[0].filename != executable or entries[0].is_dir():
                    raise ReleaseError(f"Binary ZIP for {target} must contain only root file {executable}: {name}")
                entry = entries[0]
                mode = entry.external_attr >> 16
                if stat.S_IFMT(mode) not in (0, stat.S_IFREG) or entry.file_size == 0:
                    raise ReleaseError(f"Binary ZIP for {target} has no regular executable: {name}")
                if "windows" not in target and (entry.create_system != 3 or mode & 0o111 != 0o111):
                    raise ReleaseError(f"Binary ZIP for {target} does not preserve Unix executable permissions: {name}")
                package.read(entry)
            assets[name] = data
        except (OSError, ValueError, RuntimeError, EOFError, zipfile.BadZipFile, zlib.error) as error:
            raise ReleaseError(f"Cannot validate binary ZIP for {target}: {name}: {error}") from error
    try:
        actual = {path.name for path in root.iterdir()}
    except OSError as error:
        raise ReleaseError(f"Cannot inspect binary asset directory: {error}") from error
    if actual != set(assets):
        raise ReleaseError(f"Unexpected binary release files: {', '.join(sorted(actual - set(assets)))}")
    return assets


def list_binary_assets(api, release_id):
    assets = {}
    page = 1
    while True:
        batch = api.request("GET", f"/releases/{release_id}/assets?per_page=100&page={page}")
        if not isinstance(batch, list):
            raise ReleaseError("GitHub release asset listing returned invalid metadata")
        for asset in batch:
            if not isinstance(asset, dict) or not isinstance(asset.get("name"), str):
                raise ReleaseError("GitHub release asset listing returned an invalid asset")
            if asset["name"] in assets:
                raise ReleaseError(f"Duplicate release asset: {asset['name']}")
            assets[asset["name"]] = asset
        if len(batch) < 100:
            return assets
        page += 1


def verify_binary_asset(api, asset, name, expected):
    if (not isinstance(asset, dict) or asset.get("name") != name
            or type(asset.get("id")) is not int or asset["id"] <= 0
            or asset.get("state") != "uploaded" or asset.get("size") != len(expected)):
        raise ReleaseError(f"Release asset {name} has conflicting or incomplete metadata; existing assets are preserved")
    if api.download_asset(asset["id"]) != expected:
        raise ReleaseError(f"Release asset {name} has conflicting contents; existing assets are preserved")


def ensure_binary_assets(api, release, tag, sha, expected, report):
    release = publication_metadata(release, tag)
    release_id = release["id"]
    existing = list_binary_assets(api, release_id)
    unexpected = set(existing) - set(expected)
    if unexpected:
        raise ReleaseError(f"Unexpected release assets are preserved: {', '.join(sorted(unexpected))}")
    # Check every existing asset before uploading any missing target.
    for name, asset in existing.items():
        try:
            verify_binary_asset(api, asset, name, expected[name])
        except ReleaseError as error:
            raise ReleaseError(f"Binary release verification failed for {name}: {error}") from error
    for name, data in expected.items():
        if name not in existing:
            if resolve_tag(api, tag) != sha:
                raise ReleaseError(f"Tag {tag} changed before binary upload for {name}")
            try:
                try:
                    asset = api.upload_asset(release, name, data)
                except APIError as error:
                    if error.status not in (409, 422):
                        raise
                    asset = list_binary_assets(api, release_id).get(name)
                    if asset is None:
                        raise error
                verify_binary_asset(api, asset, name, data)
            except ReleaseError as error:
                raise ReleaseError(f"Binary upload failed for {name}: {error}; rerun the original workflow") from error
            existing[name] = asset
        report(f"Verified binary asset {name}, ID {existing[name]['id']}, SHA-256 {hashlib.sha256(data).hexdigest()}")
    final = list_binary_assets(api, release_id)
    if set(final) != set(expected):
        missing = ", ".join(sorted(set(expected) - set(final))) or "none"
        unexpected = ", ".join(sorted(set(final) - set(expected))) or "none"
        raise ReleaseError(
            f"Binary release {tag} does not contain exactly the four required ZIPs; "
            f"missing: {missing}; unexpected: {unexpected}; rerun the original workflow"
        )
    for name, data in expected.items():
        if final[name]["id"] != existing[name]["id"]:
            raise ReleaseError(f"Release asset {name} changed during verification; existing assets are preserved")
        verify_binary_asset(api, final[name], name, data)
    if resolve_tag(api, tag) != sha:
        raise ReleaseError(f"Tag {tag} changed during binary release verification")
    report(f"Verified complete binary release {tag}: all four target ZIPs")


def parse_version(manifest_text, lock_text):
    try:
        manifest = tomllib.loads(manifest_text)
        lock = tomllib.loads(lock_text)
    except (tomllib.TOMLDecodeError, TypeError) as error:
        raise ReleaseError(f"Invalid version metadata TOML: {error}") from error
    package = manifest.get("package")
    if not isinstance(package, dict) or package.get("name") != "smolsandbox":
        raise ReleaseError("Cargo.toml must declare package smolsandbox")
    version = package.get("version")
    if not isinstance(version, str) or not re.fullmatch(
        r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version
    ):
        raise ReleaseError("Cargo.toml package.version must use MAJOR.MINOR.PATCH without leading zeros")
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise ReleaseError("Cargo.lock has no root smolsandbox package")
    roots = [
        entry for entry in packages
        if isinstance(entry, dict)
        and entry.get("name") == "smolsandbox"
        and "source" not in entry
    ]
    if len(roots) != 1 or roots[0].get("version") != version:
        raise ReleaseError("Cargo.lock must contain exactly one matching root smolsandbox version")
    return version


def read_metadata(api, path, sha):
    result = api.request("GET", f"/contents/{path}?ref={urllib.parse.quote(sha, safe='')}")
    if not isinstance(result, dict) or result.get("encoding") != "base64":
        raise ReleaseError(f"Cannot read {path} as base64 file metadata at {sha}")
    try:
        return base64.b64decode(result["content"], validate=False).decode("utf-8")
    except (KeyError, TypeError, ValueError, binascii.Error, UnicodeDecodeError) as error:
        raise ReleaseError(f"Invalid file content for {path} at {sha}") from error


def resolve_tag(api, tag):
    encoded_tag = urllib.parse.quote(tag, safe="")
    reference = api.request("GET", f"/git/ref/tags/{encoded_tag}", allow_missing=True)
    commit = None
    if reference is not None:
        obj = reference.get("object", {})
        visited = set()
        while obj.get("type") == "tag":
            sha = obj.get("sha")
            if not isinstance(sha, str) or sha in visited or len(visited) >= 64:
                raise ReleaseError(f"Cannot resolve annotated tag {tag}")
            visited.add(sha)
            obj = api.request("GET", f"/git/tags/{urllib.parse.quote(sha, safe='')}").get("object", {})
        if obj.get("type") != "commit" or not isinstance(obj.get("sha"), str):
            raise ReleaseError(f"Tag {tag} does not resolve to a commit")
        commit = obj["sha"]
    return commit


def release_state(api, tag):
    commit = resolve_tag(api, tag)
    releases = []
    page = 1
    while True:
        batch = api.request("GET", f"/releases?per_page=100&page={page}")
        if not isinstance(batch, list):
            raise ReleaseError("GitHub release listing returned invalid metadata")
        releases.extend(release for release in batch if release.get("tag_name") == tag)
        if len(batch) < 100:
            break
        page += 1
    if len(releases) > 1:
        raise ReleaseError(f"Multiple releases claim tag {tag}")
    return commit, releases[0] if releases else None


def checked_state(api, tag, sha, report, require_unused=False):
    commit, release = release_state(api, tag)
    if commit is not None:
        report(f"Tag: {tag} resolves to {commit}")
    if release is not None:
        report(f"Release: {release.get('html_url', '(URL unavailable)')}")
    if require_unused and (commit is not None or release is not None):
        raise ReleaseError(f"Version is already used by tag or release {tag}; a PR requires an unused version")
    if commit is not None and commit != sha:
        raise ReleaseError(f"Tag {tag} resolves to {commit}, expected {sha}; the existing tag is preserved")
    if release is not None and commit is None:
        raise ReleaseError(f"Release {tag} exists but its tag cannot be verified")
    if release is not None and release.get("prerelease", False) and not release.get("draft", False):
        raise ReleaseError(f"Existing release {tag} is a prerelease; publication requires a normal release")
    return commit, release


def publication_metadata(result, tag, expected_id=None):
    if not isinstance(result, dict):
        raise ReleaseError(f"Publication of {tag} returned invalid release metadata")
    release_id = result.get("id")
    if type(release_id) is not int or release_id <= 0:
        raise ReleaseError(f"Publication of {tag} has no positive numeric release ID")
    if expected_id is not None and release_id != expected_id:
        raise ReleaseError(f"Publication of {tag} returned release ID {release_id}, expected {expected_id}")
    if result.get("tag_name") != tag:
        raise ReleaseError(f"Publication of {tag} returned a different release tag")
    if type(result.get("draft")) is not bool or type(result.get("prerelease")) is not bool:
        raise ReleaseError(f"Publication of {tag} returned invalid draft or prerelease metadata")
    if result["prerelease"] and not result["draft"]:
        raise ReleaseError(f"Publication of {tag} is a prerelease; publication requires a normal release")
    return result


def verify_publication(api, tag, sha, result, report, expected_id=None):
    result = publication_metadata(result, tag, expected_id)
    if result["draft"]:
        raise ReleaseError(f"Publication of {tag} returned a draft instead of a normal release")
    release_id = result["id"]
    # Verify by ID up to five times, with one second between delayed reads.
    for attempt in range(1, VERIFICATION_ATTEMPTS + 1):
        commit = resolve_tag(api, tag)
        if commit is not None:
            report(f"Tag: {tag} resolves to {commit}")
        if commit is not None and commit != sha:
            raise ReleaseError(f"Tag {tag} resolves to {commit}, expected {sha}; the existing tag is preserved")
        current = api.request("GET", f"/releases/{release_id}", allow_missing=True)
        if current is not None:
            current = publication_metadata(current, tag, release_id)
            if not current["draft"] and commit == sha:
                return current
        reason = "release is not visible" if current is None else "release is still a draft"
        if commit is None:
            reason = "tag is not visible"
        report(f"Publication verification for {tag}, release {release_id}, attempt {attempt}/{VERIFICATION_ATTEMPTS}: {reason}")
        if attempt < VERIFICATION_ATTEMPTS:
            time.sleep(VERIFICATION_DELAY_SECONDS)
    raise ReleaseError(
        f"Publication of {tag}, release {release_id}, cannot be verified after "
        f"{VERIFICATION_ATTEMPTS} attempts; rerun the original workflow"
    )


def publish(api, tag, sha, report):
    assets = load_binary_assets(tag)
    commit, release = checked_state(api, tag, sha, report)
    if commit is None:
        try:
            api.request("POST", "/git/refs", {"ref": f"refs/tags/{tag}", "sha": sha})
        except APIError as error:
            if error.status not in (409, 422):
                raise
            commit, release = checked_state(api, tag, sha, report)
            if commit is None:
                raise error
        commit, release = checked_state(api, tag, sha, report)
        if commit is None:
            raise ReleaseError(f"Created tag {tag} cannot be verified")
    for _ in range(4):
        if release is not None and not release.get("draft", False):
            ensure_binary_assets(api, release, tag, sha, assets, report)
            report(f"Publication already exists for {tag}; the release is preserved")
            return "already-published"
        body = release.get("body") if release else None
        if not body:
            notes = api.request("POST", "/releases/generate-notes", {
                "tag_name": tag, "target_commitish": sha,
            })
            body = notes.get("body")
            if not isinstance(body, str):
                raise ReleaseError("GitHub generated release notes have no body")
        # Inspect both resources again immediately before a publication write.
        current_commit, current = checked_state(api, tag, sha, report)
        if current_commit != sha:
            raise ReleaseError(f"Tag {tag} cannot be verified immediately before publication")
        if current is not None and not current.get("draft", False):
            release = current
            continue
        payload = {
            "tag_name": tag,
            "target_commitish": sha,
            "name": tag,
            "draft": False,
            "prerelease": False,
            "make_latest": "legacy",
        }
        if current is None or not current.get("body"):
            payload["body"] = body
        if current is not None:
            release_id = current.get("id")
            if type(release_id) is not int or release_id <= 0:
                raise ReleaseError(f"Draft release {tag} has no positive numeric ID")
            result = api.request("PATCH", f"/releases/{release_id}", payload)
        else:
            release_id = None
            try:
                result = api.request("POST", "/releases", payload)
            except APIError as error:
                if error.status not in (409, 422):
                    raise
                _, release = checked_state(api, tag, sha, report)
                if release is None:
                    raise error
                continue
        release = verify_publication(api, tag, sha, result, report, expected_id=release_id)
        ensure_binary_assets(api, release, tag, sha, assets, report)
        report(f"Published {tag}: {release.get('html_url', '(URL unavailable)')}")
        return "published"
    raise ReleaseError(f"Concurrent publication of {tag} did not converge; rerun the original workflow")


def run(mode, api, sha, report=print):
    if mode not in ("eligibility-pr", "eligibility-main", "publish"):
        raise ReleaseError(f"Unknown RELEASE_MODE: {mode}")
    if not re.fullmatch(r"[0-9a-fA-F]{40}", sha):
        raise ReleaseError("RELEASE_SHA must be the immutable event commit SHA")
    version = parse_version(
        read_metadata(api, "Cargo.toml", sha), read_metadata(api, "Cargo.lock", sha)
    )
    tag = f"v{version}"
    report(f"Candidate version: {version}")
    report(f"Selected tag: {tag}")
    report(f"Release commit: {sha}")
    report(f"Tag URL: https://github.com/{getattr(api, 'repository', os.environ.get('GITHUB_REPOSITORY', ''))}/tree/{tag}")
    if mode == "publish":
        return publish(api, tag, sha, report)
    checked_state(api, tag, sha, report, require_unused=mode == "eligibility-pr")
    report(f"Release version eligibility passed for {tag}")
    return "eligible"


def main():
    messages = []

    def report(message):
        messages.append(str(message))
        print(message)

    try:
        api = GitHubAPI(
            os.environ.get("GITHUB_TOKEN", ""),
            os.environ.get("GITHUB_REPOSITORY", ""),
            os.environ.get("GITHUB_API_URL", "https://api.github.com"),
        )
        run(os.environ.get("RELEASE_MODE", ""), api, os.environ.get("RELEASE_SHA", ""), report)
        return 0
    except (ReleaseError, KeyError, TypeError, AttributeError) as error:
        report(f"Release check failed: {error}")
        return 1
    finally:
        summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a", encoding="utf-8") as output:
                output.write("\n".join(f"<p>{html.escape(message)}</p>" for message in messages) + "\n")


if __name__ == "__main__":
    sys.exit(main())
