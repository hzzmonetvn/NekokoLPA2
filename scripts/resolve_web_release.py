#!/usr/bin/env python3
"""Select an immutable, published app release for the Pages workflow."""

import base64
import json
import os
import re
import subprocess
from pathlib import Path
from urllib.parse import quote

TAG = re.compile(r"v\d+\.\d+\.\d+\+\d+")
VERSION = re.compile(r"\d+\.\d+\.\d+\+\d+")


def extract_version_from_gradle(gradle_text):
    name_match = re.search(r'^\s*versionName\s*=\s*"([^"]+)"', gradle_text, re.M)
    code_match = re.search(r'^\s*versionCode\s*=\s*(\d+)', gradle_text, re.M)
    if name_match and code_match:
        candidate = f"{name_match[1]}+{code_match[1]}"
        if VERSION.fullmatch(candidate):
            return candidate
    return None


def extract_version_from_pubspec(pubspec_text):
    match = re.search(r"^version:\s*(\d+\.\d+\.\d+\+\d+)\s*$", pubspec_text, re.M)
    return match[1] if match else None


def resolve_version_for_sha(sha, api):
    try:
        source = api(f"contents/android/app/build.gradle.kts?ref={sha}")
        gradle = base64.b64decode(source["content"]).decode()
        version = extract_version_from_gradle(gradle)
        if version is not None:
            return version
    except (KeyError, subprocess.CalledProcessError):
        pass

    try:
        source = api(f"contents/pubspec.yaml?ref={sha}")
        pubspec = base64.b64decode(source["content"]).decode()
        version = extract_version_from_pubspec(pubspec)
        if version is not None:
            return version
    except (KeyError, subprocess.CalledProcessError):
        pass

    return None


def select_release(event_name, event, repository, api):
    automatic = event_name != "workflow_dispatch"
    run = None
    if event_name == "workflow_run":
        run = event["workflow_run"]
        if (
            run["conclusion"] != "success"
            or run["event"] != "push"
            or run["head_branch"] != "release"
            or run["head_repository"]["full_name"] != repository
        ):
            raise ValueError("Only successful release builds from this repository qualify")
        version = resolve_version_for_sha(run["head_sha"], api)
        if version is None:
            raise ValueError("The completed build has no release version")
        tag = "v" + version
    elif event_name == "release":
        if event["release"]["draft"] or event["release"]["prerelease"]:
            return None
        tag = event["release"]["tag_name"]
    elif event_name == "workflow_dispatch":
        tag = event.get("inputs", {}).get("release_tag", "").strip()
    else:
        raise ValueError("Unsupported deployment event")

    latest = api("releases/latest")
    tag = tag or latest["tag_name"]
    if TAG.fullmatch(tag) is None:
        raise ValueError("Expected an app release tag such as v2.4.0+600")
    # A delayed old build/release event must not roll production backwards.
    if automatic and tag != latest["tag_name"]:
        return None
    release = latest if tag == latest["tag_name"] else api(f"releases/tags/{quote(tag, safe='')}")
    if release["draft"] or release["prerelease"] or release["tag_name"] != tag:
        raise ValueError("A published stable release is required")
    sha = api(f"commits/{quote(tag, safe='')}")["sha"]
    if re.fullmatch(r"[0-9a-f]{40}", sha) is None:
        raise ValueError("Invalid release commit")
    if run is not None and run["head_sha"] != sha:
        return None
    return {"tag": tag, "sha": sha, "version": tag[1:]}


def main():
    repository = os.environ["GITHUB_REPOSITORY"]
    if re.fullmatch(r"[\w.-]+/[\w.-]+", repository) is None:
        raise ValueError("Invalid repository")

    def api(path):
        result = subprocess.run(
            ["gh", "api", f"repos/{repository}/{path}"],
            check=True, capture_output=True, text=True,
        )
        return json.loads(result.stdout)

    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    selected = select_release(os.environ["GITHUB_EVENT_NAME"], event, repository, api)
    print(f"deploy={'true' if selected else 'false'}")
    for key, value in (selected or {}).items():
        print(f"{key}={value}")


if __name__ == "__main__":
    main()
