#!/usr/bin/env python3
"""Read only the button mask of one positively identified owned fixture process."""
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys

stage = 'request'
try:
    request = json.load(sys.stdin)
    stage = 'process_identity'
    identity = request['processIdentity']
    process = Path('/proc') / str(identity['pid'])
    raw = (process / 'stat').read_text()
    assert raw[raw.rfind(')') + 2:].split()[19] == identity['start_token']
    assert os.readlink(process / 'exe') == identity['executable']
    assert process.stat().st_uid == identity['uid']
    assert Path('/proc/sys/kernel/random/boot_id').read_text().strip() == identity['boot_id']
    stage = 'owned_display_environment'
    selected = request['environment']
    assert set(selected) == {'DISPLAY', 'XAUTHORITY'}
    assert all(isinstance(value, str) and value for value in selected.values())
    displays = [':' + str(number) for number in request['displays']]
    assert selected['DISPLAY'] in displays or selected['DISPLAY'] in [value + '.0' for value in displays]
    stage = 'provided_auth_path'
    if 'XAUTHORITY' in selected:
        authority = Path(selected['XAUTHORITY']).resolve(strict=True)
        assert authority.is_relative_to(Path(request['ownedRoot']).resolve(strict=True))
    code = '''
import ctypes,json,os
x=ctypes.CDLL('libX11.so.6')
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
x.XCloseDisplay.argtypes=[ctypes.c_void_p]
x.XInternAtom.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_int];x.XInternAtom.restype=ctypes.c_ulong
x.XFree.argtypes=[ctypes.c_void_p]
x.XGetWindowProperty.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_long,ctypes.c_long,ctypes.c_int,ctypes.c_ulong,ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte))]
x.XQueryPointer.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_uint)]
d=x.XOpenDisplay(os.environ['DISPLAY'].encode());assert d
try:
 atom=x.XInternAtom(d,b'_NET_WM_PID',1);assert atom
 kind=ctypes.c_ulong();fmt=ctypes.c_int();count=ctypes.c_ulong();remaining=ctypes.c_ulong();data=ctypes.POINTER(ctypes.c_ubyte)()
 assert x.XGetWindowProperty(d,WINDOW,atom,0,1,0,6,ctypes.byref(kind),ctypes.byref(fmt),ctypes.byref(count),ctypes.byref(remaining),ctypes.byref(data))==0
 try:assert kind.value==6 and fmt.value==32 and count.value==1 and ctypes.cast(data,ctypes.POINTER(ctypes.c_ulong)).contents.value==EXPECTED_PID
 finally:
  if data:x.XFree(data)
 root=ctypes.c_ulong();child=ctypes.c_ulong();rx=ctypes.c_int();ry=ctypes.c_int();wx=ctypes.c_int();wy=ctypes.c_int();mask=ctypes.c_uint()
 assert x.XQueryPointer(d,WINDOW,ctypes.byref(root),ctypes.byref(child),ctypes.byref(rx),ctypes.byref(ry),ctypes.byref(wx),ctypes.byref(wy),ctypes.byref(mask))
 print(json.dumps({'buttonMask':mask.value&0x1f00,'button1Down':bool(mask.value&0x100)}))
finally:x.XCloseDisplay(d)
    '''.replace('WINDOW', str(int(request['windowId']))).replace('EXPECTED_PID', str(int(identity['pid'])))
    stage = 'window_process_and_pointer_query'
    result = subprocess.run([sys.executable, '-c', code], env=selected, text=True, capture_output=True, timeout=5)
    assert result.returncode == 0, 'owned_pointer_query_failed'
    assert len(result.stdout) < 1024
    value = json.loads(result.stdout)
    stage = 'process_identity_after_query'
    current = (process / 'stat').read_text()
    assert current[current.rfind(')') + 2:].split()[19] == identity['start_token']
    assert os.readlink(process / 'exe') == identity['executable']
    assert process.stat().st_uid == identity['uid']
    value.update(status='observed', windowId=request['windowId'], processIdentityVerified=True, windowProcessVerified=True, inputSent=False, device="core_pointer")
    print(json.dumps(value))
except Exception:
    print(json.dumps({'status':'failed','stage':stage,'inputSent':False}))
