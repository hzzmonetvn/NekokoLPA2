from copy import deepcopy
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[2] / 'scripts'
sys.path.insert(0, str(SCRIPTS))
import publish_update_feeds as feeds
import update_config_json as writer


class FeedPublisherTest(unittest.TestCase):
    def setUp(self):
        prefix = 'ee.nekoko.nlpa2.open-2.4.0-600'
        self.names = [f'{prefix}-{tail}' for tail in [
            'android.apk', 'rooted-arm64-v8a.apk']]
        self.release = {'tag_name': 'v2.4.0+600', 'draft': False, 'prerelease': False,
                        'html_url': 'https://github.com/iebb/NekokoLPA2/releases/tag/v2.4.0%2B600',
                        'assets': [{'name': name, 'size': 100, 'state': 'uploaded',
                                    'browser_download_url': 'https://github.com/iebb/NekokoLPA2/releases/download/v2.4.0%2B600/' + name}
                                   for name in self.names]}
        self.certs = {name: [feeds.COMMUNITY_CERT] for name in self.names if name.endswith('.apk')}

    def test_all_public_channels_point_to_their_own_artifacts(self):
        payloads = feeds.plan(self.release, 'open', self.certs)
        self.assertEqual(len(payloads), 5)
        normal = next(p for p in payloads if p['platform'] == 'android' and p['identifier'].startswith('open-' + feeds.COMMUNITY_CERT))
        rooted = next(p for p in payloads if p['identifier'].startswith('open-rooted-'))
        self.assertTrue(normal['data']['download_url'].endswith('-android.apk'))
        self.assertTrue(rooted['data']['download_url'].endswith('-rooted-arm64-v8a.apk'))
        self.assertFalse(any(p['identifier'] == 'std' for p in payloads))

    def test_legacy_store_android_uses_a_chooser_not_the_wrong_apk(self):
        payloads = feeds.plan(self.release, 'open', self.certs)
        legacy = next(p for p in payloads if p['platform'] == 'android' and p['identifier'].startswith('store-'))
        self.assertEqual(legacy['data']['download_url'], self.release['html_url'])
        self.assertIn('rooted', legacy['data']['changelog'])

    def test_missing_or_unuploaded_assets_fail_before_publishing(self):
        for missing in [True, False]:
            release = deepcopy(self.release)
            if missing:
                release['assets'].pop()
            else:
                release['assets'][-1]['state'] = 'starter'
            with self.assertRaises(ValueError):
                feeds.plan(release, 'open', self.certs)

    def test_open_rejects_a_vendor_signed_apk(self):
        self.certs[self.names[0]] = ['d1c0f48b370e74d4ea4770ed4c3cd70a3198d31f']
        with self.assertRaises(ValueError):
            feeds.plan(self.release, 'open', self.certs)

    def test_release_url_cannot_redirect_to_a_different_artifact(self):
        self.release['assets'][-1]['browser_download_url'] = 'https://example.com/wrong'
        with self.assertRaises(ValueError):
            feeds.plan(self.release, 'open', self.certs)

    def test_drafts_prereleases_and_non_app_tags_are_rejected(self):
        for field, value in [('draft', True), ('prerelease', True), ('tag_name', 'nightly')]:
            release = {**self.release, field: value}
            with self.assertRaises(ValueError):
                feeds.plan(release, 'open', self.certs)

    def test_android_metadata_requires_signatures(self):
        with self.assertRaises(ValueError):
            writer.metadata('android', 'open', '2.4.0', 600, 'https://example.com/app.apk')

    def test_worker_failures_fail_publication_without_leaking_credentials(self):
        with patch.dict(os.environ, {'UPDATE_WORKER_URL': 'https://worker.example/nlpa2?key=secret', 'UPDATE_WORKER_KEY': 'secret'}):
            with patch.object(writer, 'urlopen', side_effect=RuntimeError('secret')):
                with self.assertRaisesRegex(RuntimeError, 'metadata write') as raised:
                    writer.publish({'platform': 'windows', 'identifier': 'open', 'data': {'build': 600}})
        self.assertNotIn('secret', str(raised.exception))


if __name__ == '__main__':
    unittest.main()
