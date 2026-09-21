import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

import check_changesets
import publish


class ChangesetTests(unittest.TestCase):
    def test_both_packages(self):
        self.assertEqual(check_changesets.entries('---\nbaxe: patch\nbaxe-derive: minor\n---\nImprove macros.'), {'baxe', 'baxe-derive'})

    def test_invalid_or_ignored_entries(self):
        for header in ['"baxe": patch', 'unknown: patch', 'baxe: pach', '', 'baxe: 0.1.7']:
            with self.subTest(header=header), self.assertRaises(ValueError):
                check_changesets.entries(f'---\n{header}\n---\nDescription')

    def test_description_required(self):
        with self.assertRaises(ValueError):
            check_changesets.entries('---\nbaxe: patch\n---\n')

    def test_old_changeset_does_not_cover_new_code(self):
        responses = ['base\n', 'crates/derive/src/lib.rs\n', '.changeset/old.md\n',
                     '---\nbaxe: patch\nbaxe-derive: patch\n---\nOld change']
        with patch.object(check_changesets, 'git', side_effect=responses):
            with self.assertRaisesRegex(ValueError, 'Add a changeset for: baxe, baxe-derive'):
                check_changesets.check('base', 'head')

    def test_derive_change_also_requires_public_crate(self):
        responses = ['base\n', 'crates/derive/src/lib.rs\n.changeset/new.md\n',
                     '.changeset/new.md\n', '---\nbaxe-derive: patch\n---\nFix macro']
        with patch.object(check_changesets, 'git', side_effect=responses):
            with self.assertRaisesRegex(ValueError, 'Add a changeset for: baxe$'):
                check_changesets.check('base', 'head')

    def test_release_metadata_only_is_exempt(self):
        before = '[package]\nversion = "0.1.6"\n[dependencies]\nbaxe-derive = { path = "../derive", version = "0.1.6" }\naxum = "0.7"'
        after = before.replace('0.1.6', '0.1.7')
        self.assertEqual(check_changesets.manifest_without_release_versions(before), check_changesets.manifest_without_release_versions(after))
        self.assertNotEqual(check_changesets.manifest_without_release_versions(before), check_changesets.manifest_without_release_versions(after.replace('0.7', '0.8')))


class PublishTests(unittest.TestCase):
    def test_order_and_partial_release_retry(self):
        packages = [('baxe-derive', '0.1.7'), ('baxe', '0.1.7')]
        with patch.object(publish, 'published', return_value=False), patch.object(publish.subprocess, 'run') as run:
            publish.publish(packages)
            self.assertEqual([args.args[0] for args in run.call_args_list], [
                ['cargo', 'publish', '-p', 'baxe-derive', '--locked'],
                ['cargo', 'publish', '-p', 'baxe', '--locked'],
            ])
        with patch.object(publish, 'published', side_effect=[True, False]), patch.object(publish.subprocess, 'run') as run:
            publish.publish(packages)
            run.assert_called_once_with(['cargo', 'publish', '-p', 'baxe', '--locked'], cwd=publish.ROOT, check=True)

    def test_registry_failure_stops_publication(self):
        error = urllib.error.HTTPError('url', 503, 'unavailable', {}, None)
        with patch.object(publish.urllib.request, 'urlopen', side_effect=error), patch.object(publish.subprocess, 'run') as run:
            with self.assertRaises(urllib.error.HTTPError):
                publish.publish([('baxe', '0.1.7')])
            run.assert_not_called()

    def test_only_404_means_absent(self):
        error = urllib.error.HTTPError('url', 404, 'missing', {}, None)
        with patch.object(publish.urllib.request, 'urlopen', side_effect=error):
            self.assertFalse(publish.published('baxe', '0.1.7'))
        for yanked in [False, True]:
            payload = io.BytesIO(json.dumps({'version': {'num': '0.1.7', 'yanked': yanked}}).encode())
            with patch.object(publish.urllib.request, 'urlopen', return_value=payload):
                if yanked:
                    with self.assertRaises(ValueError):
                        publish.published('baxe', '0.1.7')
                else:
                    self.assertTrue(publish.published('baxe', '0.1.7'))

    def test_plan_requires_prepared_changesets_and_notes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / '.changeset').mkdir()
            pending = root / '.changeset/pending.md'
            pending.write_text('pending')
            with self.assertRaises(ValueError):
                publish.plan(root)
            pending.unlink()
            for directory, name in zip(publish.DIRECTORIES, ['baxe-derive', 'baxe']):
                target = root / directory
                target.mkdir(parents=True)
                (target / 'Cargo.toml').write_text(f'[package]\nname = "{name}"\nversion = "0.1.7"')
                (target / 'CHANGELOG.md').write_text('## 0.1.7 (2026-09-21)\n')
            self.assertEqual(publish.plan(root), [('baxe-derive', '0.1.7'), ('baxe', '0.1.7')])
            (root / 'crates/core/CHANGELOG.md').write_text('## 0.1.6\n')
            with self.assertRaises(ValueError):
                publish.plan(root)


if __name__ == '__main__':
    unittest.main()
