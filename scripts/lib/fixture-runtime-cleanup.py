"""Retire nonce-bound Linux fixture hosts with Python 3.9+ pidfd support."""
import hashlib
import json
import os
import signal
import sys
import time
from pathlib import Path

home, nonce, expected_sha = sys.argv[1:]
assert home and nonce and len(expected_sha) == 64


def identity(pid):
    process = Path('/proc') / str(pid)
    fields = process.joinpath('stat').read_text().rsplit(') ', 1)[1].split()
    environment = dict(
        value.split(b'=', 1)
        for value in process.joinpath('environ').read_bytes().split(b'\0')
        if b'=' in value
    )
    executable = process.joinpath('exe').stat()
    return process, fields[19], fields[0], environment, (executable.st_dev, executable.st_ino)


retired = []
for process in Path('/proc').iterdir():
    if not process.name.isdigit():
        continue
    try:
        process, start, state, environment, inode = identity(process.name)
    except (OSError, ValueError):
        continue
    if (environment.get(b'HOME') != home.encode()
            or environment.get(b'AGENT_BROWSER_FIXTURE_RUN_ID') != nonce.encode()):
        continue
    assert environment.get(b'AGENT_BROWSER_RUNTIME_HOST') == b'1'
    digest = hashlib.sha256()
    with process.joinpath('exe').open('rb') as executable:
        for chunk in iter(lambda: executable.read(1024 * 1024), b''):
            digest.update(chunk)
    assert digest.hexdigest() == expected_sha
    try:
        descriptor = os.pidfd_open(int(process.name))
    except ProcessLookupError:
        continue
    try:
        _, current_start, _, current_environment, current_inode = identity(process.name)
        assert current_start == start and current_inode == inode and current_environment == environment
        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        retired.append({'pid': int(process.name), 'start': start})
    except ProcessLookupError:
        pass
    finally:
        os.close(descriptor)

residue = []
for _ in range(100):
    residue = []
    for record in retired:
        try:
            _, start, state, _, _ = identity(record['pid'])
            if start == record['start'] and state != 'Z':
                residue.append(record['pid'])
        except (OSError, ValueError):
            pass
    if not residue:
        break
    time.sleep(0.1)
assert not residue, residue
# A fresh census also catches a task-owned host created after initial enumeration.
for process in Path('/proc').iterdir():
    if not process.name.isdigit():
        continue
    try:
        _, _, state, environment, _ = identity(process.name)
    except (OSError, ValueError):
        continue
    if (state != 'Z' and environment.get(b'HOME') == home.encode()
            and environment.get(b'AGENT_BROWSER_FIXTURE_RUN_ID') == nonce.encode()):
        residue.append(int(process.name))
assert not residue, residue
print(json.dumps({'retired': retired, 'residue': residue}))
