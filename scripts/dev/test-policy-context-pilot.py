"""Validate the adopted pilot with the installed governance-context engine.
Run with the installed governance-context venv Python and a repository path.
"""
from pathlib import Path
import json, shutil, sys, tempfile
from governance_context.files import Files
root=Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()
manifest=json.loads((root/'.governance/policy-context.json').read_text())
checks=0
for profile in manifest['profiles']:
 for kind in profile['task_kinds']:
  packet=Files(root).policy(profile['id'],kind)
  assert packet['status']=='current', (profile['id'],packet)
  assert not packet['fallback_required']
  checks+=1
with tempfile.TemporaryDirectory(prefix='policy-pilot-') as directory:
 target=Path(directory)
 for item in manifest['governance_inventory']:
  dest=target/item['path'];dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(root/item['path'],dest)
 (target/'.governance').mkdir();shutil.copyfile(root/'.governance/policy-context.json',target/'.governance/policy-context.json')
 profile=manifest['profiles'][0]['id']
 def fallback(kind='documentation'):
  result=Files(target).policy(profile,kind);assert result['fallback_required'] and result['status']!='current';return result
 fallback('unsupported');checks+=1
 source=target/'AGENTS.md';original=source.read_bytes();source.write_bytes(original+b'\nChanged source\n');fallback();checks+=1;source.write_bytes(original)
 extra=target/'docs/dev/policies/9999-added.md';extra.write_text('# Added\n');fallback();checks+=1;extra.unlink()
 source.unlink();fallback();checks+=1;source.write_bytes(original)
 assert Files(target).policy(profile,'documentation')['status']=='current';checks+=1
print(json.dumps({'status':'passed','checks':checks,'profiles':len(manifest['profiles']),'inventory_files':len(manifest['governance_inventory'])}))
