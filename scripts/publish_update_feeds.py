#!/usr/bin/env python3
"""Publish Android update feeds."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.request import Request, urlopen

from update_config_json import get_apk_signature, metadata, publish

REPOSITORY = 'iebb/NekokoLPA2'
COMMUNITY_CERT = 'c47350c7ba682b34a3e584a0d58463ea42b1ad73'
FEED = 'https://updates.lpa.ee/nlpa2.json'


def api(path):
    result = subprocess.run(['gh', 'api', f'repos/{REPOSITORY}/{path}'],
                            check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def plan(release, channel, certificates):
    tag = release['tag_name']
    if release['draft'] or release['prerelease'] or re.fullmatch(r'v\d+\.\d+\.\d+\+\d+', tag) is None:
        raise ValueError('A published stable app release is required')
    version, build = tag[1:].split('+')
    suffix = tag[1:].replace('+', '-')
    assets = {asset['name']: asset for asset in release['assets'] if asset['state'] == 'uploaded'}
    result = []

    def add(platform, variant, filename, signatures=()):
        asset = assets.get(filename)
        if not asset or asset['size'] <= 0:
            raise ValueError(f'Required release asset is missing: {filename}')
        expected = f'https://github.com/{REPOSITORY}/releases/download/{tag}/{filename}'
        if asset['browser_download_url'].replace('%2B', '+') != expected:
            raise ValueError('Release asset URL does not match the selected release')
        result.append(metadata(platform, variant, version, build,
                               asset['browser_download_url'], signatures, asset['size']))

    if channel in ('public', 'open'):
        prefix = f'ee.nekoko.nlpa2.open-{suffix}'
        for variant, filename in [
            ('open', f'{prefix}-android.apk'),
            ('open-rooted', f'{prefix}-rooted-arm64-v8a.apk'),
        ]:
            signatures = certificates[filename]
            if signatures != [COMMUNITY_CERT]:
                raise ValueError('Open APK must use the public community certificate')
            add('android', variant, filename, signatures)
            if variant == 'open':
                add('android-armeabi-v7a', variant, filename, signatures)
        # Before separate channels existed, both public Android flavors said
        # `store`. Their certificate/variant cannot distinguish rooted from
        # normal; give those legacy clients a chooser, never the wrong APK.
        legacy = metadata('android', 'store', version, build, release['html_url'], [COMMUNITY_CERT])
        legacy['data']['changelog'] = ('Open update available. Choose the Open Android APK or the Open rooted module matching your installation; vendor APKs use different signing/package identities.')
        result.append(legacy)
        result.append({**legacy, 'platform': 'android-armeabi-v7a'})
    else:
        raise ValueError('Unknown channel')
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--release-tag', default='')
    parser.add_argument('--channel', choices=['public', 'open'], default='public')
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    latest = api('releases/latest')
    tag = args.release_tag or latest['tag_name']
    # Feed writes cannot roll back a channel through a delayed/manual old run.
    if tag != latest['tag_name']:
        raise ValueError('Only the latest stable release may update the feed')
    release = latest
    certificates = {}
    with tempfile.TemporaryDirectory() as tmp:
        if args.channel in ('public', 'open'):
            suffix = tag[1:].replace('+', '-')
            for tail in ['android.apk', 'rooted-arm64-v8a.apk']:
                filename = f'ee.nekoko.nlpa2.open-{suffix}-{tail}'
                subprocess.run(['gh', 'release', 'download', tag, '--repo', REPOSITORY,
                                '--pattern', filename, '--dir', tmp], check=True)
                certificates[filename] = get_apk_signature(Path(tmp) / filename)
        payloads = plan(release, args.channel, certificates)
    with urlopen(Request(FEED, headers={'User-Agent': 'Mozilla/5.0'}), timeout=30) as response:
        current = json.load(response)
    for payload in payloads:
        previous = current.get(payload['platform'], {}).get(payload['identifier'], {})
        if isinstance(previous, dict) and previous.get('build', 0) > payload['data']['build']:
            raise ValueError('The feed already contains a newer build')
    for payload in payloads:
        if args.dry_run:
            print(json.dumps(payload))
        else:
            publish(payload)


if __name__ == '__main__':
    main()
