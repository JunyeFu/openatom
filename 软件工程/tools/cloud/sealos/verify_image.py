"""Finite checks against the built Linux image and a new, real TLS PostgreSQL instance.

Requires the existing Python cryptography library; never installs anything.
Keeps new containers, their volume and network after normal stop. No existing DB is used.
"""
import argparse
import datetime as dt
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
from urllib.error import HTTPError
from urllib.request import urlopen, Request

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / '.work/v2-implementation-20261026/sealos-container'


def run(args, *, input=None, check=True):
    result = subprocess.run(args, input=input, text=True, encoding='utf-8', capture_output=True)
    if check and result.returncode:
        # Do not echo command/environment, which can contain private deployment data.
        raise RuntimeError(f'command failed with exit {result.returncode}; inspect scoped logs')
    return result


def restrict(path):
    path.mkdir(parents=True, exist_ok=False)
    if os.name == 'nt':
        script = '''$a=[System.Security.AccessControl.DirectorySecurity]::new();
$a.SetAccessRuleProtection($true,$false);
$s=[System.Security.Principal.WindowsIdentity]::GetCurrent().User;
$i=[System.Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit';
foreach($p in @($s,[System.Security.Principal.SecurityIdentifier]::new('S-1-5-18'))){$a.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($p,'FullControl',$i,'None','Allow'))};
[System.IO.Directory]::SetAccessControl($env:SF_SEALOS_PRIVATE,$a)'''
        subprocess.run(['powershell', '-NoProfile', '-NonInteractive', '-Command', script],
                       env=dict(os.environ, SF_SEALOS_PRIVATE=str(path)), check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def certificates(path, hostname):
    now = dt.datetime.now(dt.timezone.utc)
    ca_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'sf-sealos-local-test-only')])
    ca = (x509.CertificateBuilder().subject_name(name).issuer_name(name)
          .public_key(ca_key.public_key()).serial_number(x509.random_serial_number())
          .not_valid_before(now - dt.timedelta(minutes=5)).not_valid_after(now + dt.timedelta(days=3))
          .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
          .sign(ca_key, hashes.SHA256()))
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    server = (x509.CertificateBuilder()
              .subject_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, hostname)]))
              .issuer_name(ca.subject).public_key(key.public_key()).serial_number(x509.random_serial_number())
              .not_valid_before(now - dt.timedelta(minutes=5)).not_valid_after(now + dt.timedelta(days=3))
              .add_extension(x509.SubjectAlternativeName([x509.DNSName(hostname)]), critical=False)
              .sign(ca_key, hashes.SHA256()))
    (path / 'ca.pem').write_bytes(ca.public_bytes(serialization.Encoding.PEM))
    (path / 'server.crt').write_bytes(server.public_bytes(serialization.Encoding.PEM))
    (path / 'server.key').write_bytes(key.private_bytes(serialization.Encoding.PEM,
        serialization.PrivateFormat.TraditionalOpenSSL, serialization.NoEncryption()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', default='sf-cloud-service:sealos-20261027')
    parser.add_argument('--postgres-image', default='postgres:18')
    args = parser.parse_args()
    stamp = dt.datetime.now().strftime('%Y%m%d-%H%M%S')
    prefix = f'sf-sealos-{stamp}'
    directory = WORK / stamp
    directory.mkdir(parents=True)
    private = directory / 'private'
    restrict(private)
    checks, containers = [], []
    receipt = {'started_at': dt.datetime.now(dt.timezone.utc).isoformat(), 'checks': checks,
               'image': args.image, 'directory': str(directory), 'containers': containers,
               'scope': 'new isolated Linux container component checks; not Sealos deployment or full acceptance'}

    def record(name, actual, expected):
        checks.append({'name': name, 'actual': actual, 'expected': expected, 'pass': actual == expected})
        (directory / 'runtime-results.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps(checks[-1]), flush=True)
        if actual != expected:
            raise AssertionError(name)

    def container(suffix, image, extra, command=()):
        name = prefix + '-' + suffix
        result = run(['docker', 'run', '--detach', '--name', name,
                      '--label', 'sf.batch=sealos-container-20261027', *extra, image, *command])
        containers.append({'name': name, 'id': result.stdout.strip()})
        return name

    def wait_exit(name):
        return int(run(['docker', 'wait', name]).stdout.strip())

    def log(name):
        result = run(['docker', 'logs', name])
        (directory / f'{name}.log').write_text(result.stdout + result.stderr, encoding='utf-8')

    def sql(statement):
        return run(['docker', 'exec', '-i', pg, 'psql', '-X', '-q', '-v', 'ON_ERROR_STOP=1',
                    '-U', 'sfcontainer', '-d', 'sfcontainer', '-At'], input=statement).stdout.strip()

    def http(path, token=None):
        headers = {} if token is None else {'Authorization': 'Bearer ' + token}
        try:
            with urlopen(Request(base + path, headers=headers), timeout=8) as response:
                return response.status, response.read()
        except HTTPError as error:
            return error.code, error.read()

    try:
        info = json.loads(run(['docker', 'image', 'inspect', args.image]).stdout)[0]
        receipt['image_id'] = info['Id']
        record('image platform', info['Os'] + '/' + info['Architecture'], 'linux/amd64')
        record('image runs as nonroot', info['Config']['User'], '10001:10001')
        record('direct process entrypoint', info['Config']['Entrypoint'], ['/opt/superfactory/bin/sf-cloud-service'])
        missing = container('missing-config', args.image, [])
        record('missing configuration exits', wait_exit(missing), 1)
        log(missing)

        hostname = prefix + '-pg'
        certificates(private, hostname)
        password = secrets.token_hex(24)
        fixtures = json.loads((ROOT / 'packages/domain/tests/fixtures/cloud_wire_cases.json').read_text(encoding='utf-8-sig'))
        b = next(x['value'] for x in fixtures if x['definition'] == 'CandidateBinding' and x['label'] == 'positive')
        bindings = {'projects': [{'cloud_project_id': 'example', 'local_project_id': 'example', 'owner_scope_ref': 'host'}],
            'device_bindings': [{'binding_id': b['binding_id'], 'cloud_project_id': 'example', 'device_id': b['device_id'],
              'principal_scope_ref': 'device-principal', 'definition_ref': b['device_module_ref'],
              'capability_map': {'action': 'process_unit', 'operation_ref': b['operation_ref'], 'execution_capacity_ref': b['execution_capacity_ref']}}],
            'datasets': [{'dataset_id': 'telemetry-fixture', 'cloud_project_id': 'example', 'source_mode': 'software_in_loop', 'purpose': 'telemetry_only'},
                         {'dataset_id': 'load-fixture', 'cloud_project_id': 'example', 'source_mode': 'generated_load', 'purpose': 'generated_load'}]}
        (directory / 'component-bindings.json').write_text(json.dumps(bindings), encoding='utf-8')
        principals = []
        for principal, role in [('host', 'host_gateway'), ('reader', 'scoped_reader'), ('operator', 'operator'),
                                ('device-principal', 'device'), ('producer', 'generated_load_producer')]:
            principals.append({'token': secrets.token_hex(32), 'principal_id': principal, 'role': role,
                'project_ids': ['example'], 'device_ids': [b['device_id']],
                'dataset_ids': ['load-fixture'] if role == 'generated_load_producer' else ['telemetry-fixture', 'load-fixture']})
        (private / 'postgres.env').write_text(f'POSTGRES_USER=sfcontainer\nPOSTGRES_DB=sfcontainer\nPOSTGRES_PASSWORD={password}\n', encoding='utf-8')
        def env_file(name, host):
            target = private / name
            # A hostile sslmode=disable must not override the service's required TLS connector.
            target.write_text(f'DATABASE_URL=postgresql://sfcontainer:{password}@{host}:5432/sfcontainer?sslmode=disable\n'
                + 'SF_CLOUD_AUTH_TOKENS_JSON=' + json.dumps(principals, separators=(',', ':')) + '\n', encoding='utf-8')
            return str(target)
        app_env = env_file('app.env', hostname)
        wrong_env = env_file('wrong-host.env', 'wrong-pg-name')
        network = prefix + '-net'
        volume = prefix + '-pgdata'
        receipt.update(network=network, volume=volume, postgres_image=args.postgres_image)
        run(['docker', 'network', 'create', '--label', 'sf.batch=sealos-container-20261027', network])
        run(['docker', 'volume', 'create', '--label', 'sf.batch=sealos-container-20261027', volume])
        # postgres copies its test key into its own Linux volume; Windows bind-mount modes are unsuitable for PG server keys.
        pg_command = ['bash', '-ceu', 'mkdir -p /var/lib/postgresql/tls; cp /run/test-tls/server.* /var/lib/postgresql/tls/; chown -R postgres:postgres /var/lib/postgresql/tls; chmod 600 /var/lib/postgresql/tls/server.key; exec /usr/local/bin/docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/var/lib/postgresql/tls/server.crt -c ssl_key_file=/var/lib/postgresql/tls/server.key -c ssl_min_protocol_version=TLSv1.2']
        pg = container('pg', args.postgres_image, ['--network', network, '--network-alias', hostname,
            '--network-alias', 'wrong-pg-name', '--publish', '127.0.0.1::5432', '--env-file', str(private / 'postgres.env'),
            '--mount', f'type=bind,source={private},target=/run/test-tls,readonly',
            '--mount', f'type=volume,source={volume},target=/var/lib/postgresql'], pg_command)
        for _ in range(80):
            # initdb's temporary Unix-socket-only server is not the final TLS listener.
            result = run(['docker', 'exec', pg, 'pg_isready', '-h', '127.0.0.1', '-U', 'sfcontainer', '-d', 'sfcontainer'], check=False)
            if result.returncode == 0:
                break
            time.sleep(.25)
        record('real PostgreSQL ready', result.returncode, 0)
        receipt['server_version'] = sql('SHOW server_version;')
        record('PostgreSQL requires minimum TLS 1.2', sql('SHOW ssl_min_protocol_version;'), 'TLSv1.2')
        common = ['--network', network, '--env-file', app_env,
            '--mount', f'type=bind,source={directory / "component-bindings.json"},target=/opt/superfactory/config/demo-bindings.json,readonly']
        trusted = [*common, '--mount', f'type=bind,source={private / "ca.pem"},target=/run/sf-ca/ca.pem,readonly',
                   '--env', 'SSL_CERT_FILE=/run/sf-ca/ca.pem']
        app = container('app', args.image, [*trusted, '--publish', '127.0.0.1::8080'])
        address = run(['docker', 'port', app, '8080/tcp']).stdout.strip()
        receipt['app_loopback_port'] = address
        receipt['postgres_loopback_port'] = run(['docker', 'port', pg, '5432/tcp']).stdout.strip()
        base = 'http://' + address
        for _ in range(60):
            try:
                status, body = http('/healthz')
                break
            except OSError:
                time.sleep(.1)
        record('unmigrated real database is not ready', [status, len(body)], [503, 0])
        migration = container('migrate', args.image, trusted, ['migrate'])
        record('migrate real database over trusted TLS', wait_exit(migration), 0)
        log(migration)
        record('migration table count', int(sql("SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE';")), 16)
        status, body = http('/healthz')
        record('health after migration', [status, len(body)], [204, 0])
        record('business requires bearer credential', http('/v2/projects/example/datasets')[0], 401)
        reader = next(x['token'] for x in principals if x['role'] == 'scoped_reader')
        record('scoped reader can read actual dataset catalog', http('/v2/projects/example/datasets', reader)[0], 200)
        ssl_rows = sql("SELECT version,cipher,bits FROM pg_stat_ssl WHERE ssl AND pid<>pg_backend_pid();")
        receipt['tls_sessions'] = ssl_rows.splitlines()
        record('service uses encrypted PG sessions despite sslmode=disable', bool(ssl_rows), True)
        untrusted = container('untrusted-ca', args.image, common, ['migrate'])
        record('untrusted database certificate rejected', wait_exit(untrusted), 1)
        log(untrusted)
        wrong = container('wrong-host', args.image, [*trusted, '--env-file', wrong_env], ['migrate'])
        record('trusted certificate with wrong DNS hostname rejected', wait_exit(wrong), 1)
        log(wrong)
        again = container('migrate-again', args.image, trusted, ['migrate'])
        record('idempotent migration in same new DB', wait_exit(again), 0)
        log(again)
        for _ in range(60):
            health = run(['docker', 'inspect', '--format', '{{.State.Health.Status}}', app]).stdout.strip()
            if health == 'healthy':
                break
            time.sleep(.5)
        record('Docker actual healthcheck', health, 'healthy')
        run(['docker', 'stop', '--time', '30', app])
        record('SIGTERM graceful exit', int(run(['docker', 'inspect', '--format', '{{.State.ExitCode}}', app]).stdout.strip()), 0)
        record('database remains usable after service stop', sql('SELECT 1;'), '1')
        receipt['status'] = 'passed'
    except Exception as error:
        receipt['status'] = 'failed'
        receipt['error'] = str(error)
        raise
    finally:
        for entry in reversed(containers):
            name = entry['name']
            state = run(['docker', 'inspect', '--format', '{{.State.Running}}', name], check=False).stdout.strip()
            if state == 'true':
                run(['docker', 'stop', '--time', '30', name])
            log(name)
            entry['final'] = run(['docker', 'inspect', '--format', '{{.State.Status}} exit={{.State.ExitCode}}', name]).stdout.strip()
        receipt['finished_at'] = dt.datetime.now(dt.timezone.utc).isoformat()
        (directory / 'runtime-results.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        print(json.dumps({'status': receipt['status'], 'receipt': str(directory / 'runtime-results.json')}), flush=True)


if __name__ == '__main__':
    main()
