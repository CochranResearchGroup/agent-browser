#!/usr/bin/env python3
"""Opt-in minimal reopen diagnostic. Synthetic provider, never Native acceptance."""
import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time
from lib.p237_native_provider_fixture import start_native_provider_fixture


def wait(predicate, seconds=20):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.05)
    raise TimeoutError('diagnostic observation deadline')


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'chrome', 'results'):
        ap.add_argument('--' + name, type=Path, required=True)
    ap.add_argument('--display', type=int, required=True)
    args = ap.parse_args()
    args.results = args.results.resolve(); args.results.mkdir(mode=0o700, parents=True, exist_ok=False)
    assert not Path(f'/tmp/.X11-unix/X{args.display}').exists()
    assert not Path(f'/tmp/.X{args.display}-lock').exists()
    env = os.environ | {'HOME': str(args.results), 'DISPLAY': f':{args.display}',
                        'AGENT_BROWSER_EXECUTABLE_PATH': str(args.chrome.resolve())}
    wire = json.loads((Path(__file__).resolve().parents[1] / 'docs/dev/contracts/remote-view-application-r3.v1.fixture.json').read_text())
    target = wire['assignmentObservation']['target']
    server = None; child = None; display = None; failure = None; complete = False
    def start_display():
        process = subprocess.Popen(['Xvfb', f':{args.display}', '-screen', '0', '640x480x24', '-ac', '-nolisten', 'tcp'],
                                   stdout=subprocess.DEVNULL, stderr=(args.results / 'xvfb.log').open('a'))
        wait(lambda: subprocess.run(['xdpyinfo'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0)
        return process
    try:
        display = start_display()
        server = start_native_provider_fixture(env, args.results)
        cfg = {'binary': str(args.binary.resolve()), 'chrome': str(args.chrome.resolve()),
               'results': str(args.results), 'target': target, 'application': 'agent-browser',
               'controlOrigin': env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN'],
               'publicOrigin': env['AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN'], 'skipWarm': True}
        config = args.results / 'bridge-config.json'; config.write_text(json.dumps(cfg)); config.chmod(0o600)
        child = subprocess.Popen(['node', str(Path(__file__).with_name('smoke-hosted-browser-restart-runner.mjs')), str(config)],
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(args.results / 'agent.log').open('w'), text=True)
        restarted = False; deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if not select.select([child.stdout], [], [], 1)[0]:
                assert child.poll() is None, 'runner exited'
                continue
            line = child.stdout.readline(); assert line, 'runner pipe closed'
            request = json.loads(line)
            if request['operation'] == 'peers':
                value = {'scope': 'minimised_no_peer'}
            elif request['operation'] == 'restart':
                assert not restarted; restarted = True
                proof = request['proof']; pid = proof['pid']
                assert Path(f'/proc/{pid}/exe').resolve() == args.chrome.resolve()
                assert str(args.results / 'agent') in proof['profilePath']
                display.terminate(); display.wait(timeout=10)
                wait(lambda: not Path(f'/proc/{pid}').exists())
                display = start_display()
                (args.results / 'managed-provider-viewing-generation.json').write_text('20')
                before = {'desktopId': target['desktopId'], 'assignmentId': target['assignmentId'],
                          'lifecycleGeneration': 7, 'viewingGeneration': 19}
                value = {'state': 'ready_for_explicit_rebind', 'before': before,
                         'after': before | {'viewingGeneration': 20},
                         'launchBinding': {'readinessScope': 'live_resource'},
                         'applicationLoss': {'observed': 'lost'}, 'websiteEffectsReplayed': False}
            elif request['operation'] == 'reopened':
                proof = request['proof']
                found = subprocess.run(['xdotool', 'search', '--onlyvisible', '--pid', str(proof['pid'])], env=env, capture_output=True, text=True)
                assert found.stdout.strip()
                value = {'desktopId': target['desktopId'], 'viewingGeneration': 20,
                         'processId': proof['pid'], 'windowObserved': True}
            elif request['operation'] == 'complete':
                complete = request['passed']; child.wait(timeout=15)
                break
            else:
                raise ValueError('unrecognised diagnostic bridge operation')
            child.stdin.write(json.dumps({'ok': True, 'value': value}) + '\n'); child.stdin.flush()
        else:
            raise TimeoutError('diagnostic overall deadline')
    except Exception as error:
        failure = {'type': type(error).__name__, 'message': str(error)}
    finally:
        if child and child.poll() is None:
            try:
                child.stdin.write('{"ok":false}\n'); child.stdin.flush(); child.wait(timeout=30)
            except (BrokenPipeError, subprocess.TimeoutExpired):
                child.terminate(); child.wait(timeout=10)
        if display and display.poll() is None:
            display.terminate(); display.wait(timeout=10)
        if server:
            server.shutdown(); server.server_close(); server.viewer_fixture.shutdown(); server.viewer_fixture.server_close()
    receipt = {'scope': 'synthetic-provider-Xvfb diagnostic; not Native acceptance',
               'functionalPass': complete, 'failure': failure}
    path = args.results / 'probe-receipt.json'; path.write_text(json.dumps(receipt)); path.chmod(0o600)
    print(json.dumps(receipt))
    return 0 if complete and not failure else 1


if __name__ == '__main__':
    raise SystemExit(main())
