"""Opt-in physical target/window selection on a task-owned X11 desktop."""
import argparse
import json
import os
from pathlib import Path
import select
import signal
import shutil
import subprocess
import tempfile
import time


def session_members(session_ids):
    members = []
    for entry in Path('/proc').iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            raw = (entry / 'stat').read_text()
            fields = raw[raw.rfind(')') + 2:].split()
            if fields[0] != 'Z' and int(fields[3]) in session_ids:
                members.append({'pid': int(entry.name), 'group': int(fields[2]),
                                'session': int(fields[3])})
        except (OSError, ValueError, IndexError):
            continue
    return members


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tests', required=True)
    parser.add_argument('--chrome', required=True)
    parser.add_argument('--artifact-dir', required=True)
    parser.add_argument('--service-selection', action='store_true')
    parser.add_argument('--http-selection', action='store_true')
    parser.add_argument('--retained-selection', action='store_true')
    parser.add_argument('--native-dispatch', action='store_true')
    parser.add_argument('--multi-window', action='store_true', help='Select targets across two windows in each owned browser')
    parser.add_argument('--ui-runner', help='Absolute task-owned Node runner for UI selection ordering')
    parser.add_argument('--ui-bundle', help='Absolute prepared viewport fixture bundle')
    parser.add_argument('--namespace-worker', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--namespace-parent-proof-fd', type=int, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.multi_window:
        assert not args.retained_selection and not args.ui_runner, 'Multi-window native dispatch is a separate fixture mode'
        args.native_dispatch = True
    assert bool(args.ui_runner) == bool(args.ui_bundle), 'UI runner and bundle are required together'
    if args.ui_runner:
        assert not args.retained_selection, 'UI fixture uses prepared native dispatch, not retained mode'
        assert Path(args.ui_bundle).is_absolute() and Path(args.ui_bundle).is_file()
        assert Path(args.ui_runner).is_absolute() and Path(args.ui_runner).is_file()
        args.native_dispatch = True
    if args.retained_selection or args.native_dispatch:
        args.http_selection = True
    if args.http_selection:
        args.service_selection = True
    for value in [args.tests, args.chrome]:
        assert Path(value).is_absolute() and Path(value).is_file()
    if not args.namespace_worker:
        command = ['unshare', '--user', '--map-current-user', '--keep-caps', '--mount', '--net',
                   '--propagation', 'private', 'python3', str(Path(__file__).resolve()),
                   '--tests', args.tests, '--chrome', args.chrome,
                   '--artifact-dir', args.artifact_dir, '--namespace-worker']
        if args.service_selection:
            command.append('--service-selection')
        if args.http_selection:
            command.append('--http-selection')
        if args.retained_selection:
            command.append('--retained-selection')
        if args.native_dispatch:
            command.append('--native-dispatch')
        if args.multi_window:
            command.append('--multi-window')
        if args.ui_runner:
            command.extend(['--ui-runner', args.ui_runner, '--ui-bundle', args.ui_bundle])
        read_fd, write_fd = os.pipe()
        proof = {'parentPid': os.getpid(), 'namespaces': {
            namespace: os.readlink(f'/proc/self/ns/{namespace}')
            for namespace in ['user', 'mnt', 'net']}}
        try:
            os.write(write_fd, json.dumps(proof).encode())
            os.close(write_fd)
            write_fd = None
            command.extend(['--namespace-parent-proof-fd', str(read_fd)])
            result = subprocess.run(command, start_new_session=True, pass_fds=(read_fd,)).returncode
        finally:
            os.close(read_fd)
            if write_fd is not None:
                os.close(write_fd)
        raise SystemExit(result)
    assert args.namespace_parent_proof_fd is not None, 'parent namespace proof missing'
    with os.fdopen(args.namespace_parent_proof_fd, 'r') as proof_stream:
        parent_proof = json.loads(proof_stream.read(4096))
    assert parent_proof['parentPid'] == os.getppid(), 'namespace worker parent changed'
    namespace_proof = {}
    for namespace in ['user', 'mnt', 'net']:
        current = os.readlink(f'/proc/self/ns/{namespace}')
        parent = parent_proof['namespaces'][namespace]
        assert current != parent, f'private {namespace} namespace was not established'
        namespace_proof[namespace] = {'worker': current, 'parent': parent}
    artifacts = Path(args.artifact_dir)
    assert artifacts.is_absolute()
    artifacts.mkdir(mode=0o700, parents=True, exist_ok=False)
    (artifacts / 'namespace-proof.json').write_text(json.dumps(namespace_proof, indent=2) + '\n')
    children = []
    result = None
    with tempfile.TemporaryDirectory(prefix='p237-native-selection-') as temporary:
        root = Path(temporary)
        sockets = root / 'x11-sockets'
        sockets.mkdir(mode=0o1777)
        sockets.chmod(0o1777)
        subprocess.run(['mount', '--bind', str(sockets), '/tmp/.X11-unix'], check=True)
        subprocess.run(['ip', 'link', 'set', 'lo', 'up'], check=True)
        env = os.environ.copy()
        for name in list(env):
            if name.startswith(('AGENT_BROWSER_', 'P237_')):
                env.pop(name)
        for name in ['HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME',
                     'XDG_CACHE_HOME', 'XDG_RUNTIME_DIR']:
            directory = root / name.lower()
            directory.mkdir(mode=0o700)
            env[name] = str(directory)
        env.pop('DBUS_SESSION_BUS_ADDRESS', None)
        env.pop('XAUTHORITY', None)
        env.update({'P237_NATIVE_SELECTION_ROOT': str(root),
                    'P237_NATIVE_SELECTION_CHROME': args.chrome,
                    'RUST_MIN_STACK': '16777216'})
        if args.multi_window:
            env['P237_NATIVE_SELECTION_MULTI_WINDOW'] = '1'
        if args.ui_runner:
            env['P237_NATIVE_SELECTION_UI_RUNNER'] = args.ui_runner
            env['P237_NATIVE_SELECTION_UI_BUNDLE'] = args.ui_bundle

        def spawn(command, **kwargs):
            child = subprocess.Popen(command, start_new_session=True, env=env, **kwargs)
            children.append(child)
            return child

        try:
            with (artifacts / 'desktop.log').open('w') as desktop_log, \
                    (artifacts / 'workflow.log').open('w') as workflow_log:
                def start_desktop():
                    read_fd, write_fd = os.pipe()
                    try:
                        xvfb = spawn(['Xvfb', '-displayfd', str(write_fd), '-screen', '0',
                                      '1280x800x24', '-nolisten', 'tcp'],
                                     pass_fds=(write_fd,), stdout=desktop_log, stderr=desktop_log)
                        os.close(write_fd)
                        write_fd = None
                        assert select.select([read_fd], [], [], 15)[0], 'Xvfb readiness timed out'
                        display_number = os.read(read_fd, 64).decode().strip()
                        assert display_number.isdecimal() and xvfb.poll() is None
                        desktop_display = ':' + display_number
                    finally:
                        os.close(read_fd)
                        if write_fd is not None:
                            os.close(write_fd)
                    env['DISPLAY'] = desktop_display
                    wm = spawn(['openbox', '--sm-disable'], stdout=desktop_log, stderr=desktop_log)
                    deadline = time.monotonic() + 15
                    while True:
                        ready = subprocess.run(['xprop', '-root', '_NET_SUPPORTING_WM_CHECK'],
                                               env=env, capture_output=True, text=True)
                        if ready.returncode == 0 and 'window id #' in ready.stdout:
                            break
                        assert xvfb.poll() is None and wm.poll() is None
                        assert time.monotonic() < deadline, 'task-owned window manager readiness timed out'
                        time.sleep(0.025)
                    return desktop_display
                primary_display = start_desktop()
                if args.service_selection:
                    env['P237_NATIVE_SELECTION_PEER_DISPLAY'] = start_desktop()
                env['DISPLAY'] = primary_display
                if args.http_selection:
                    env['P237_NATIVE_SELECTION_HTTP'] = '1'
                if args.retained_selection:
                    env['P237_NATIVE_SELECTION_RETAINED'] = '1'
                if args.native_dispatch:
                    env['P237_NATIVE_SELECTION_DISPATCH'] = '1'
                test = spawn([args.tests,
                              ('native::stream::http::tests::physical_service_selection_preserves_distinct_desktops'
                               if args.service_selection else
                               'native::browser::tests::physical_selection_activates_exact_target_window_and_maximizes'),
                              '--exact', '--ignored', '--nocapture', '--test-threads=1'],
                             stdout=workflow_log, stderr=subprocess.STDOUT)
                result = test.wait(timeout=90)
                (artifacts / 'test-result.json').write_text(json.dumps({'returncode':result,'testPid':test.pid},indent=2) + '\n')
                identity = root / 'browser-identity.json'
                if identity.exists():
                    (artifacts / 'browser-identity.json').write_text(identity.read_text())
                failure = root / 'physical-failure.json'
                if failure.exists():
                    (artifacts / 'physical-failure.json').write_text(failure.read_text())
                for name in ['ui-selection-observation.json', 'ui-selection-progress.json',
                             'ui-selection-cleanup.json']:
                    evidence = root / name
                    if evidence.exists():
                        shutil.copy2(evidence, artifacts / name)
                logs = root / 'home' / '.agent-browser' / 'tmp' / 'chrome-launches'
                for log in logs.glob('*.stderr.log'):
                    shutil.copy2(log, artifacts / log.name)
        finally:
            sessions = {child.pid for child in children}
            # The UI launcher creates its own Chrome session inside this task's
            # private temporary profile root. Retain its exact birth receipt so
            # a runner deadline can reclaim that session after its parent exits.
            unproven_ui_owners = []
            ui_owner = root / 'ui-selection-owned-session.json'
            if ui_owner.exists():
                owner = json.loads(ui_owner.read_text())
                current = Path(f"/proc/{owner['pid']}")
                try:
                    raw = (current / 'stat').read_text()
                    fields = raw[raw.rfind(')') + 2:].split()
                    command = (current / 'cmdline').read_bytes()
                    verified = (owner['profileRoot'] == str(root / 'ui-temporary') and
                                fields[19] == owner['start'] and
                                int(fields[3]) == owner['session'] and
                                b'--p237-ui-ordering-fixture' in command and
                                f"--user-data-dir={owner['profileRoot']}/".encode() in command)
                except FileNotFoundError:
                    verified = owner['profileRoot'] == str(root / 'ui-temporary')
                if verified:
                    sessions.add(owner['session'])
                else:
                    unproven_ui_owners.append(owner)
                shutil.copy2(ui_owner, artifacts / ui_owner.name)
            # Only descendants in sessions this driver created are eligible.
            for sig in [signal.SIGTERM, signal.SIGKILL]:
                groups = {member['group'] for member in session_members(sessions)}
                for group in groups:
                    try:
                        os.killpg(group, sig)
                    except ProcessLookupError:
                        pass
                deadline = time.monotonic() + 3
                while session_members(sessions) and time.monotonic() < deadline:
                    time.sleep(0.025)
            for child in children:
                child.wait(timeout=5)
            remaining = session_members(sessions)
            (artifacts / 'cleanup.json').write_text(json.dumps({
                'taskCreatedSessions': sorted(sessions), 'remaining': remaining,
                'unprovenUiOwners': unproven_ui_owners,
                'scope': 'exact task-created OS sessions, including verified UI ownership'}, indent=2) + '\n')
            assert not remaining, 'task-owned native fixture processes remain'
            assert not unproven_ui_owners, 'UI process identity changed; cleanup authority unproven'
    output = (artifacts / 'workflow.log').read_text()
    print(output)
    assert 'running 1 test' in output, 'physical test selection did not run the intended test'
    assert result == 0 and 'test result: ok. 1 passed; 0 failed;' in output, 'physical native selection workflow failed'
    assert (artifacts / 'browser-identity.json').is_file(), 'browser ownership identity missing'


if __name__ == '__main__':
    main()
