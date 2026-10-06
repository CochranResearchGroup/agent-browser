"""Check archive integrity and governance discovery with the installed engine."""
from pathlib import Path
import hashlib, json, re, sys
root=Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()
archive=root/'docs/history/2026-10-05-legacy-planning'
manifest=json.loads((archive/'manifest.json').read_text())
link=re.compile(r'(!?\[[^\]]*\]\()\s*(<[^>]+>|[^\s)]+)')
assert manifest['disposition']=='historical_only'
for entry in manifest['entries']:
 p=root/entry['archived_path']
 assert p.is_file(),entry['archived_path']
 assert hashlib.sha256(p.read_bytes()).hexdigest()==entry['archived_sha256'],entry['archived_path']
 assert hashlib.sha256(link.sub(lambda x:x[1]+'TARGET',p.read_text()).encode()).hexdigest()==entry['prose_sha256'],entry['archived_path']
 assert not (root/entry['original_path']).exists() or entry['original_path'] in ['RUNBOOK.md','ROADMAP.md','docs/dev/active-lanes.yaml'],entry['original_path']
assert len((root/'RUNBOOK.md').read_text().splitlines())<=200
plan=(root/'docs/dev/plans/0223-2026-10-04-finish-agent-browser-acceptance.md').read_text()
execution=re.search(r'^Execution: (ACTIVE|PAUSED)\b',plan,re.M)
assert execution, 'active plan must declare execution state'
assert f'P223 version 6 is {execution[1]}' in (root/'RUNBOOK.md').read_text()
assert 'Current Planning Context' in (root/'AGENTS.md').read_text()
# Query the real installed service's public discovery API with its default includes.
from governance_context.files import Files
files=Files(root)
visible=files.list(prefix='docs/history')
assert not visible['paths'],visible
plans=files.list(prefix='docs/dev/plans')
assert all(p.endswith('/README.md') or '/0223-' in p for p in plans['paths']),plans
print(json.dumps({'status':'passed','archived_files':len(manifest['entries']),'visible_archive_files':len(visible['paths']),'active_plan_files':plans['paths']}))
