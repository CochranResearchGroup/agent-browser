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
    parser.add_argument('--tests')
    parser.add_argument('--runtime-cli', help='Absolute source executable for the task-owned runtime-host probe')
    parser.add_argument('--runtime-ui-bundle', help='Absolute prepared viewport bundle for shared-desktop selection')
    parser.add_argument('--runtime-controller-fence', action='store_true', help='Exercise public controller takeover against the actual external route fence')
    parser.add_argument('--runtime-eof', action='store_true', help='Exercise actual native Unix EOF while browser work is running')
    parser.add_argument('--runtime-peer-read', action='store_true', help='Exercise peer title read without native focus changes')
    parser.add_argument('--runtime-bootstrap', action='store_true', help='Let the public source CLI create its private host')
    parser.add_argument('--runtime-public-background', action='store_true', help='Create and observe an authorized target through public HTTP')
    parser.add_argument('--runtime-managed-session', action='store_true', help='Exercise managed session admission through the native protocol')
    parser.add_argument('--runtime-managed-restart', action='store_true', help='Check managed task Properties after an exact source host restart')
    parser.add_argument('--runtime-managed-recovery', action='store_true', help='Recover the same retained handoff after real task expiry and source host restart')
    parser.add_argument('--runtime-managed-operator', action='store_true', help='Exercise authenticated native selection, existing handoff, compact UI and queued authority fencing')
    parser.add_argument('--runtime-managed-replay', action='store_true', help='Observe a managed tab creation replay after last task closure')
    parser.add_argument('--runtime-cross-client-focus', nargs='?', const='ordering', choices=['ordering','authority','effect-authority','timeout','cancel','independent'], help='Select one desktop through actual HTTP clients during renderer work')
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
    assert bool(args.tests) != bool(args.runtime_cli), 'Choose a test executable or runtime CLI'
    assert not args.runtime_controller_fence or args.runtime_cli, 'Controller fence requires source runtime mode'
    assert not args.runtime_eof or args.runtime_cli, 'Native EOF requires source runtime mode'
    assert not args.runtime_eof or not (args.runtime_controller_fence or args.runtime_ui_bundle), 'Native EOF has separate presentation setup'
    assert not args.runtime_peer_read or (args.runtime_cli and not (args.runtime_eof or args.runtime_controller_fence or args.runtime_ui_bundle)), 'Peer read has separate source runtime setup'
    assert not args.runtime_bootstrap or (args.runtime_cli and not (args.runtime_eof or args.runtime_controller_fence or args.runtime_ui_bundle or args.runtime_peer_read)), 'CLI bootstrap has separate source runtime setup'
    assert not args.runtime_public_background or (args.runtime_cli and not (args.runtime_eof or args.runtime_controller_fence or args.runtime_ui_bundle or args.runtime_peer_read or args.runtime_bootstrap)), 'Public background has separate source runtime setup'
    assert not args.runtime_managed_session or (args.runtime_cli and not (args.runtime_eof or args.runtime_controller_fence or args.runtime_ui_bundle or args.runtime_peer_read or args.runtime_bootstrap or args.runtime_public_background)), 'Managed session has separate source runtime setup'
    assert not args.runtime_managed_replay or args.runtime_managed_session, 'Managed replay requires managed session setup'
    assert not args.runtime_managed_operator or args.runtime_managed_session, 'Managed operator proof requires managed session setup'
    assert not args.runtime_managed_restart or args.runtime_managed_session, 'Managed restart requires managed session setup'
    assert not args.runtime_managed_recovery or args.runtime_managed_session, 'Managed recovery requires managed session setup'
    assert not args.runtime_cross_client_focus or (args.runtime_cli and not (args.runtime_eof or args.runtime_controller_fence or args.runtime_ui_bundle or args.runtime_peer_read or args.runtime_bootstrap or args.runtime_public_background or args.runtime_managed_session)), 'Cross-client focus has separate source runtime setup'
    if args.runtime_ui_bundle:
        assert args.runtime_cli and Path(args.runtime_ui_bundle).is_absolute() and Path(args.runtime_ui_bundle).is_file()
    if args.runtime_cli:
        assert not any([args.service_selection, args.http_selection, args.retained_selection,
                        args.native_dispatch, args.multi_window, args.ui_runner]), 'Runtime startup is a separate fixture mode'
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
    for value in [args.tests or args.runtime_cli, args.chrome]:
        assert Path(value).is_absolute() and Path(value).is_file()
    if not args.namespace_worker:
        command = ['unshare', '--user', '--map-current-user', '--keep-caps', '--mount', '--net',
                   '--propagation', 'private', 'python3', str(Path(__file__).resolve()),
                   '--tests' if args.tests else '--runtime-cli', args.tests or args.runtime_cli, '--chrome', args.chrome,
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
        if args.runtime_ui_bundle:
            command.extend(['--runtime-ui-bundle', args.runtime_ui_bundle])
        if args.runtime_controller_fence:
            command.append('--runtime-controller-fence')
        if args.runtime_eof:
            command.append('--runtime-eof')
        if args.runtime_peer_read:
            command.append('--runtime-peer-read')
        if args.runtime_bootstrap:
            command.append('--runtime-bootstrap')
        if args.runtime_public_background:
            command.append('--runtime-public-background')
        if args.runtime_managed_session:
            command.append('--runtime-managed-session')
        if args.runtime_managed_replay:
            command.append('--runtime-managed-replay')
        if args.runtime_managed_operator:
            command.append('--runtime-managed-operator')
        if args.runtime_managed_restart:
            command.append('--runtime-managed-restart')
        if args.runtime_managed_recovery:
            command.append('--runtime-managed-recovery')
        if args.runtime_cross_client_focus:
            command.extend(['--runtime-cross-client-focus',args.runtime_cross_client_focus])
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
            task_env = kwargs.pop('task_env', env)
            child = subprocess.Popen(command, start_new_session=True, env=task_env, **kwargs)
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
                        announcement = b''
                        announcement_deadline = time.monotonic() + 15
                        while b'\n' not in announcement:
                            remaining_seconds = announcement_deadline - time.monotonic()
                            assert remaining_seconds > 0 and select.select([read_fd], [], [], remaining_seconds)[0], 'Xvfb readiness timed out'
                            chunk = os.read(read_fd, 64)
                            assert chunk, 'Xvfb readiness ended before its complete line'
                            announcement += chunk
                            assert len(announcement) <= 64, 'Xvfb readiness announcement exceeded its bound'
                        display_number = announcement.decode().strip()
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
                if args.runtime_cli:
                    env['P237_NATIVE_MANAGED_RECOVERY'] = '1' if args.runtime_managed_recovery else '0'
                    from lib.p237_runtime_host_fixture import run_runtime_host_fixture
                    runtime_started = time.monotonic()
                    run_runtime_host_fixture(spawn, env, root, artifacts,
                                             args.runtime_cli, args.chrome, workflow_log,
                                             args.runtime_ui_bundle, args.runtime_controller_fence, args.runtime_eof, args.runtime_peer_read, args.runtime_bootstrap, args.runtime_public_background, args.runtime_managed_session, args.runtime_managed_replay, args.runtime_cross_client_focus, args.runtime_managed_operator, args.runtime_managed_restart)
                    result = 0
                    (artifacts / 'test-result.json').write_text(json.dumps({
                        'mode':'source-runtime-host','returncode':0,
                        'elapsedSeconds':time.monotonic()-runtime_started},indent=2)+'\n')
                else:
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
            # Preserve UI diagnostics even when the physical oracle rejects.
            # The private config and synthetic cookie are deliberately excluded.
            for name in ['ui-selection-observation.json', 'ui-selection-progress.json',
                         'ui-selection-cleanup.json']:
                evidence = root / name
                if evidence.exists():
                    shutil.copy2(evidence, artifacts / name)
            sessions = {child.pid for child in children}
            host_owner = root / 'source-host-owned-session.json'
            if host_owner.exists():
                owner = json.loads(host_owner.read_text())
                current = Path(f"/proc/{owner['pid']}")
                try:
                    raw = (current/'stat').read_text(); fields = raw[raw.rfind(')')+2:].split()
                    environment = (current/'environ').read_bytes().split(b'\0')
                    verified = (fields[19] == owner['start'] and int(fields[3]) == owner['session']
                                and os.readlink(current/'exe') == args.runtime_cli
                                and f'HOME={root / "home"}'.encode() in environment
                                and f'AGENT_BROWSER_SOCKET_DIR={root / "runtime-sockets"}'.encode() in environment
                                and b'AGENT_BROWSER_RUNTIME_HOST_PROCESS=1' in environment)
                except FileNotFoundError:
                    verified = True
                assert verified, 'source-created host identity changed; cleanup authority unproven'
                sessions.add(owner['session'])
                shutil.copy2(host_owner,artifacts/host_owner.name)
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
            root_processes = []
            for process in Path('/proc').iterdir():
                if not process.name.isdecimal():
                    continue
                try:
                    if str(root).encode() in (process / 'cmdline').read_bytes():
                        root_processes.append(int(process.name))
                except OSError:
                    continue
            (artifacts / 'cleanup.json').write_text(json.dumps({
                'taskCreatedSessions': sorted(sessions), 'remaining': remaining,
                'privateRootProcesses': root_processes, 'privateRoot': str(root),
                'unprovenUiOwners': unproven_ui_owners,
                'scope': 'exact task-created OS sessions, including verified UI ownership'}, indent=2) + '\n')
            assert not remaining, 'task-owned native fixture processes remain'
            assert not root_processes, 'a process still references the private task root; no broad cleanup is authorized'
            assert not unproven_ui_owners, 'UI process identity changed; cleanup authority unproven'
    output = (artifacts / 'workflow.log').read_text()
    print(output)
    if args.runtime_cli:
        assert result == 0 and 'runtime-host-workflow: passed' in output
        return
    assert 'running 1 test' in output, 'physical test selection did not run the intended test'
    assert result == 0 and 'test result: ok. 1 passed; 0 failed;' in output, 'physical native selection workflow failed'
    assert (artifacts / 'browser-identity.json').is_file(), 'browser ownership identity missing'


if __name__ == '__main__':
    main()
