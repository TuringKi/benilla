#!/usr/bin/env python3
"""Build/publish the web client and manage a private-network HTTPS deployment."""
import argparse
import gzip
import ipaddress
import json
import os
from pathlib import Path
import shutil
import shlex
import signal
import socket
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def run(*args, **kwargs):
    return subprocess.run([str(x) for x in args], check=True, **kwargs)


def atomic(path, text):
    tmp = path.with_name(path.name + '.tmp')
    tmp.write_text(text)
    tmp.chmod(0o600)
    tmp.replace(path)


def quote(value):
    value = str(value)
    if any(ord(c) < 32 for c in value):
        raise ValueError('Control characters are not allowed in configuration paths')
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"').replace('$', '\\$') + '"'


def address(value):
    return str(ipaddress.ip_address(value))


def endpoint(host, port):
    return f'[{host}]:{port}' if ':' in host else f'{host}:{port}'


def port(value):
    value = int(value)
    if not 1 <= value <= 65535:
        raise argparse.ArgumentTypeError('Port must be between 1 and 65535')
    return value


def process_start(pid):
    try:
        # Match process identity, not only a PID that the OS could have reused.
        fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return None if fields[0] == 'Z' else fields[19]
    except (OSError, IndexError):
        return None


def alive(record):
    return bool(record and process_start(record['pid']) == record['start'])


def stop_host(record):
    if alive(record):
        os.kill(record['pid'], signal.SIGTERM)
        for _ in range(100):
            if not alive(record):
                return
            time.sleep(.05)
        raise RuntimeError('Managed host did not stop; no other process was killed')


def nginx_running(state):
    try:
        pid = int((state / 'nginx.pid').read_text())
        command = Path(f'/proc/{pid}/cmdline').read_bytes().decode(errors='replace')
        return str(state / 'nginx.conf') in command and 'nginx' in command
    except (OSError, ValueError):
        return False


def read_record(state):
    try:
        return json.loads((state / 'deployment.json').read_text())
    except FileNotFoundError:
        return {}


def tls(state, ips, ca_cert=None, ca_key=None):
    folder = state / 'tls'
    folder.mkdir(mode=0o700, exist_ok=True)
    cert, key = folder / 'ca.crt', folder / 'ca.key'
    if bool(ca_cert) != bool(ca_key):
        raise ValueError('--ca-cert and --ca-key must be supplied together')
    for source, target in [(ca_cert, cert), (ca_key, key)]:
        if source and Path(source).resolve() != target:
            shutil.copyfile(source, target)
            target.chmod(0o600)
    if cert.exists() != key.exists():
        raise ValueError('Incomplete CA pair; refusing to replace an existing CA')
    if not cert.exists():
        run('openssl', 'genrsa', '-out', key, '3072', stdout=subprocess.DEVNULL)
        run('openssl', 'req', '-x509', '-new', '-key', key, '-sha256', '-days', '3650',
            '-subj', '/CN=Wenilla Private Network CA', '-out', cert)
    server_key = folder / 'server.key'
    if not server_key.exists():
        run('openssl', 'genrsa', '-out', server_key, '2048', stdout=subprocess.DEVNULL)
    extension = folder / 'server.ext'
    atomic(extension, 'basicConstraints=critical,CA:FALSE\n'
           'keyUsage=critical,digitalSignature,keyEncipherment\n'
           'extendedKeyUsage=serverAuth\nsubjectAltName=' +
           ','.join('IP:' + ip for ip in ips) + '\n')
    run('openssl', 'req', '-new', '-key', server_key, '-subj', '/CN=Wenilla Server',
        '-out', folder / 'server.csr')
    run('openssl', 'x509', '-req', '-in', folder / 'server.csr', '-CA', cert, '-CAkey', key,
        '-CAcreateserial', '-days', '825', '-sha256', '-extfile', extension,
        '-out', folder / 'server.crt')
    for private in (key, server_key):
        private.chmod(0o600)
    return folder


def nginx_config(state, ips, backend, backend_port, http_port, https_port, diagnostics_port=None):
    http = '\n'.join(f'        listen {endpoint(ip, http_port)};' for ip in ips)
    https = '\n'.join(f'        listen {endpoint(ip, https_port)} ssl;' for ip in ips)
    diagnostics = '' if diagnostics_port is None else f'        location = /ui-report {{ client_max_body_size 64k; proxy_pass http://{endpoint(backend, diagnostics_port)}; }}\n'
    return f'''worker_processes 1;
pid {quote(state / 'nginx.pid')};
error_log {quote(state / 'nginx-error.log')};
events {{ worker_connections 1024; }}
http {{
    access_log {quote(state / 'nginx-access.log')};
    map $http_upgrade $connection_upgrade {{ default upgrade; '' close; }}
    server {{
{http}
        server_name _;
        location = /lan-ca.crt {{ proxy_pass http://{endpoint(backend, backend_port)}/lan-ca.crt; }}
        location / {{ return 302 https://$host:{https_port}$request_uri; }}
    }}
    server {{
{https}
        server_name _;
        ssl_certificate {quote(state / 'tls/server.crt')};
        ssl_certificate_key {quote(state / 'tls/server.key')};
        ssl_protocols TLSv1.2 TLSv1.3;
{diagnostics}        location / {{
            proxy_pass http://{endpoint(backend, backend_port)};
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection $connection_upgrade;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_read_timeout 3600s;
            proxy_send_timeout 3600s;
            proxy_buffering off;
        }}
    }}
}}
'''


def publish(source, state, realm):
    for name in ('index.html', 'wenilla.js', 'wenilla_bg.wasm'):
        if not (source / name).is_file():
            raise ValueError(f'Missing web artifact: {name}; build first or use --build')
    releases = state / 'releases'
    releases.mkdir(exist_ok=True)
    release = releases / str(time.time_ns())
    shutil.copytree(source, release, symlinks=True)
    alias = release / 'render-test'
    if alias.is_symlink():
        alias.unlink()
    elif alias.exists():
        raise ValueError('render-test must be a symlink, not an independent deployment')
    alias.symlink_to('.', target_is_directory=True)
    page = release / 'index.html'
    marker = '<script type="module">'
    if marker not in page.read_text():
        raise ValueError('The web page is missing its module entry point')
    page.write_text(page.read_text().replace(marker,
        '<script>window.__wenilla_env = Object.assign(window.__wenilla_env || {}, '
        '{host: new URLSearchParams(location.search).get("host") || ' +
        json.dumps(realm) + '});</script>\n' + marker, 1))
    shutil.copyfile(state / 'tls/ca.crt', release / 'lan-ca.crt')
    for name in ('index.html.gz', 'index.html.br', 'lan-ca.crt.gz', 'lan-ca.crt.br'):
        (release / name).unlink(missing_ok=True)
    # Never leave a stale compressed sibling after updating the uncompressed file.
    for file in release.iterdir():
        if file.suffix in ('.js', '.wasm'):
            zipped = file.with_name(file.name + '.gz')
            if zipped.exists() and gzip.decompress(zipped.read_bytes()) != file.read_bytes():
                raise ValueError(f'Stale gzip artifact: {file.name}')
    link = state / 'site.new'
    link.unlink(missing_ok=True)
    link.symlink_to(release, target_is_directory=True)
    link.replace(state / 'site')


def up(args, state):
    ips = list(dict.fromkeys(address(ip) for ip in args.listen_ip))
    if any(ipaddress.ip_address(ip).is_unspecified for ip in ips):
        raise ValueError('Use explicit private-network interface addresses, not wildcard binds')
    if not args.game_host or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-:[]' for c in args.game_host):
        raise ValueError('Invalid game host')
    if len({args.backend_port, args.http_port, args.https_port}) != 3:
        raise ValueError('Backend, HTTP and HTTPS ports must differ')
    backend = socket.gethostbyname('localhost')
    data = Path(args.data).resolve()
    if not data.exists():
        raise ValueError('Game data path does not exist')
    addons = Path(args.addons).resolve() if args.addons else None
    if addons and not addons.is_dir():
        raise ValueError('AddOns path does not exist')
    source = Path(args.www).resolve()
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    if not target.is_absolute():
        target = ROOT / target
    host_bin = Path(args.host_bin).resolve() if args.host_bin else target.resolve() / 'release/wenilla-host'
    for tool in ('openssl', args.nginx_bin):
        if not shutil.which(tool):
            raise ValueError(f'Required executable not found: {tool}')
    if args.build:
        if args.host_bin:
            raise ValueError('--build uses Cargo output; omit --host-bin')
        source = state / 'build'
        env = os.environ.copy()
        env['WEB_DIST'] = str(source)
        run('bash', ROOT / 'scripts/web-build.sh', cwd=ROOT, env=env)
        run('cargo', 'build', '--release', '-p', 'wenilla-host', cwd=ROOT)
    if not host_bin.is_file():
        raise ValueError('Host executable not found; pass --host-bin or use --build')
    tls(state, ips, args.ca_cert, args.ca_key)
    config = state / 'nginx.conf'
    atomic(config, nginx_config(state, ips, backend, args.backend_port, args.http_port, args.https_port, args.diagnostics_port))
    nginx = [args.nginx_bin, '-p', str(state) + '/', '-c', str(config)]
    run(*nginx, '-t')
    old = read_record(state)
    command = [str(host_bin), '--www', str(state / 'site'), '--data', str(data),
               '--bind', endpoint(backend, args.backend_port), '--upstream', args.game_host,
               '--auth-port', str(args.auth_port), '--world-port', str(args.world_port)]
    if addons:
        command += ['--addons', str(addons)]
    publish(source, state, endpoint(args.game_host, args.auth_port))
    record = old.get('host')
    if not alive(record) or old.get('command') != command:
        stop_host(record)
        with (state / 'host.log').open('ab', buffering=0) as log:
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log, stderr=log, start_new_session=True)
        record = {'pid': child.pid, 'start': process_start(child.pid)}
    url = 'http://' + endpoint(backend, args.backend_port) + '/'
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    for _ in range(80):
        if not alive(record):
            raise RuntimeError('Host exited; inspect the private host.log')
        try:
            with opener.open(url, timeout=1) as response:
                if response.status == 200:
                    break
        except OSError:
            time.sleep(.1)
    else:
        raise RuntimeError('Host did not become ready')
    try:
        if nginx_running(state):
            run(*nginx, '-s', 'reload')
        else:
            run(*nginx)
    except Exception:
        if record != old.get('host'):
            stop_host(record)
        raise
    atomic(state / 'deployment.json', json.dumps({'host': record, 'command': command, 'nginx': nginx,
           'listen_ips': ips, 'http_port': args.http_port, 'https_port': args.https_port}, indent=2))
    # Save a restart command privately; it contains runtime addresses, never source defaults.
    restart = [sys.executable, str(ROOT / 'scripts/deploy-web.py'), 'up', '--state-dir', str(state),
               '--game-host', args.game_host, '--data', str(data), '--www', str(source),
               '--host-bin', str(host_bin), '--backend-port', str(args.backend_port),
               '--http-port', str(args.http_port), '--https-port', str(args.https_port),
               '--auth-port', str(args.auth_port), '--world-port', str(args.world_port),
               '--nginx-bin', args.nginx_bin]
    for ip in ips:
        restart += ['--listen-ip', ip]
    if addons:
        restart += ['--addons', str(addons)]
    if args.diagnostics_port:
        restart += ['--diagnostics-port', str(args.diagnostics_port)]
    atomic(state / 'start.local.sh', '#!/usr/bin/env bash\nset -euo pipefail\nexec ' + shlex.join(restart) + '\n')
    for ip in ips:
        print('Play: https://' + endpoint(ip, args.https_port) + '/?host=' + urllib.parse.quote(endpoint(args.game_host, args.auth_port), safe=''))
        print('CA: http://' + endpoint(ip, args.http_port) + '/lan-ca.crt')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    for action in ('up', 'status', 'stop'):
        p = sub.add_parser(action)
        p.add_argument('--state-dir', default=str(ROOT / '.web-deploy'))
        if action != 'up':
            continue
        p.add_argument('--listen-ip', action='append', required=True, help='Repeat for LAN and VPN interfaces')
        p.add_argument('--game-host', required=True)
        p.add_argument('--data', required=True)
        p.add_argument('--addons')
        p.add_argument('--www', default=str(ROOT / 'web/dist'))
        p.add_argument('--host-bin')
        p.add_argument('--backend-port', type=port, default=8090)
        p.add_argument('--http-port', type=port, default=8093)
        p.add_argument('--https-port', type=port, default=8443)
        p.add_argument('--auth-port', type=port, default=3724)
        p.add_argument('--world-port', type=port, default=8085)
        p.add_argument('--diagnostics-port', type=port, help='Optional existing UI diagnostic receiver')
        p.add_argument('--ca-cert')
        p.add_argument('--ca-key')
        p.add_argument('--nginx-bin', default='nginx')
        p.add_argument('--build', action='store_true')
    args = parser.parse_args()
    state = Path(args.state_dir).resolve()
    # Private runtime configuration is allowed only outside the checkout or in an ignored path.
    if state.is_relative_to(ROOT):
        ignored = subprocess.run(['git', 'check-ignore', '-q', str(state / 'deployment.json')], cwd=ROOT).returncode == 0
        if not ignored:
            parser.error('The state directory must be outside the checkout or ignored by Git')
    os.umask(0o077)
    state.mkdir(mode=0o700, parents=True, exist_ok=True)
    state.chmod(0o700)
    if args.action == 'up':
        up(args, state)
    else:
        record = read_record(state)
        if args.action == 'status':
            print(json.dumps({'host_running': alive(record.get('host')), 'nginx_running': nginx_running(state)}, indent=2))
        else:
            if nginx_running(state):
                run(*record['nginx'], '-s', 'quit')
            stop_host(record.get('host'))
            print('Managed deployment stopped')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, RuntimeError, OSError, subprocess.CalledProcessError) as error:
        print('Deployment failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
