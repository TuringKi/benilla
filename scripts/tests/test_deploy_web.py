import importlib.util
from pathlib import Path
import socket
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'deploy-web.py'
spec = importlib.util.spec_from_file_location('deploy_web', MODULE)
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class DeploymentTests(unittest.TestCase):
    def test_render_keeps_redirect_host_and_websocket_headers(self):
        ip = socket.gethostbyname('localhost')
        conf = deploy.nginx_config(Path('/private/state with spaces'), [ip], ip, 9000, 9001, 9002)
        self.assertIn(f'listen {ip}:9001;', conf)
        self.assertIn('https://$host:9002$request_uri', conf)
        self.assertIn('proxy_set_header Upgrade $http_upgrade', conf)
        self.assertIn('proxy_set_header Connection $connection_upgrade', conf)
        self.assertIn('"/private/state with spaces/tls/server.key"', conf)

    def test_address_rejects_configuration_injection(self):
        for value in ('host; malicious', 'localhost', 'address\nlisten other'):
            with self.assertRaises(ValueError):
                deploy.address(value)
        with self.assertRaises(ValueError):
            deploy.quote('path\nserver')

    def test_published_alias_and_runtime_host_share_one_release(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source, state = root / 'source', root / 'state'
            source.mkdir()
            state.mkdir()
            (state / 'tls').mkdir()
            (state / 'tls/ca.crt').write_text('public test CA')
            (source / 'index.html').write_text('<script type="module">boot()</script>')
            (source / 'wenilla.js').write_text('export default function() {}')
            (source / 'wenilla_bg.wasm').write_bytes(b'wasm fixture')
            (source / 'render-test').symlink_to('.', target_is_directory=True)
            deploy.publish(source, state, 'game.example.test:3725')
            site = state / 'site'
            self.assertEqual((site / 'render-test').resolve(), site.resolve())
            self.assertIn('game.example.test:3725', (site / 'index.html').read_text())
            self.assertIn('new URLSearchParams', (site / 'index.html').read_text())
            old = site.resolve()
            deploy.publish(source, state, 'changed.example.test:3725')
            self.assertNotEqual(old, site.resolve())
            self.assertTrue(old.is_dir())
            self.assertIn('changed.example.test', (site / 'index.html').read_text())

    def test_stale_compressed_artifact_is_rejected(self):
        import gzip
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source, state = root / 'source', root / 'state'
            source.mkdir()
            (state / 'tls').mkdir(parents=True)
            (state / 'tls/ca.crt').write_text('public test CA')
            (source / 'index.html').write_text('<script type="module"></script>')
            (source / 'wenilla.js').write_text('new code')
            (source / 'wenilla.js.gz').write_bytes(gzip.compress(b'old code'))
            (source / 'wenilla_bg.wasm').write_bytes(b'wasm fixture')
            with self.assertRaisesRegex(ValueError, 'Stale gzip'):
                deploy.publish(source, state, 'game.example.test:3725')
            self.assertFalse((state / 'site').exists())


if __name__ == '__main__':
    unittest.main()
