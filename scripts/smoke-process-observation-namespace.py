#!/usr/bin/env python3
"""Opt-in Linux user-systemd identity probe with an owned synthetic process.

A sleep executable named chrome stands in for a browser: this checks executable
and process-start observation across sibling PrivateTmp user namespaces, not
CDP, browser authorization, or installed production acceptance. All artifacts
must live outside private /tmp and outside the product repository.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
import uuid


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, capture_output=True, timeout=20, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--artifact-dir', required=True, type=Path)
    args = parser.parse_args()
    artifact = args.artifact_dir.resolve()
    repo = Path(__file__).resolve().parents[1]
    if artifact.is_relative_to(Path('/tmp')) or artifact.is_relative_to(repo):
        parser.error('artifact-dir must be a private directory outside /tmp and the repository')
    artifact.mkdir(parents=True, exist_ok=False, mode=0o700)
    binary = args.binary.resolve(strict=True)
    synthetic = artifact / 'chrome'
    shutil.copy2('/bin/sleep', synthetic)
    home = artifact / 'home'
    profile = home / '.agent-browser/runtime-profiles/synthetic'
    user_data = profile / 'user-data'
    user_data.mkdir(parents=True)
    unit = 'agent-browser-observer-smoke-' + uuid.uuid4().hex
    receipt = {'binarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'cases': {}}
    try:
        run(['systemd-run', '--user', '--quiet', '--collect', '--unit', unit,
             '--property=PrivateTmp=true', '--property=NoNewPrivileges=true',
             '--property=RuntimeMaxSec=90s', str(synthetic), '90'])
        pid = 0
        deadline = time.monotonic() + 5
        while not pid and time.monotonic() < deadline:
            pid = int(run(['systemctl', '--user', 'show', unit, '--property=MainPID', '--value']).stdout.strip())
        if not pid:
            raise AssertionError('synthetic process did not start')
        start = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
        boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
        identity = {'pid': pid, 'startToken': f'linux:{boot}:{start}',
                    'executablePath': str(synthetic), 'browserFamily': 'chrome'}
        state = {'runtimeProfile': 'synthetic', 'userDataDir': str(user_data),
                 'browserPid': pid, 'processIdentity': identity, 'headed': False,
                 'launchMode': 'synthetic_identity_only'}
        state_path = profile / 'runtime-state.json'

        def probe(label, executable, expected):
            state_path.write_text(json.dumps(state))
            result = run(['systemd-run', '--user', '--quiet', '--wait', '--pipe', '--collect',
                          '--unit', unit + '-' + label, '--property=PrivateTmp=true',
                          '--property=NoNewPrivileges=true', '--property=RuntimeMaxSec=15s',
                          '--setenv=HOME=' + str(home), str(executable), '--json',
                          'runtime', 'status', 'synthetic'])
            value = json.loads(result.stdout)
            (artifact / (label + '.json')).write_text(json.dumps(value, indent=2) + '\n')
            assert value.get('browserAlive') is expected, (label, value)
            receipt['cases'][label] = {'browserAlive': value['browserAlive'], 'passed': True}

        if args.baseline:
            receipt['baselineSha256'] = hashlib.sha256(args.baseline.read_bytes()).hexdigest()
            probe('baseline', args.baseline.resolve(), False)
        probe('exact', binary, True)
        identity['startToken'] = f'linux:{boot}:0'
        probe('wrong-start', binary, False)
        identity['startToken'] = f'linux:{boot}:{start}'
        identity['executablePath'] = '/synthetic/wrong-executable'
        probe('wrong-executable', binary, False)
    finally:
        stopped = subprocess.run(['systemctl', '--user', 'stop', unit], text=True, capture_output=True, timeout=10)
        receipt['cleanupStopSucceeded'] = stopped.returncode == 0
        observation = subprocess.run(['systemctl', '--user', 'show', unit, '--property=MainPID', '--value'], text=True, capture_output=True, timeout=10)
        receipt['remainingMainPid'] = observation.stdout.strip() or '0'
        (artifact / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    assert receipt['cleanupStopSucceeded'] and receipt['remainingMainPid'] == '0', receipt
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
