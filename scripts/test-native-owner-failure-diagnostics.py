import ast, pathlib
p=pathlib.Path(__file__).with_name('smoke-hosted-browser-matrix-native.py')
tree=ast.parse(p.read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='native_owner_failure_diagnostic'];assert len(nodes)==1,'missing bounded owner failure diagnostic'
namespace={};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(p),'exec'),namespace);reason=namespace['native_owner_failure_diagnostic']
def envelope(message):return {'schema_version':1,'code':'input_effect_uncertain','message':message,'retryable':False}
for token in ['native_x11_hold_already_pressed','input_failed','native_x11_control_unavailable','input_argv_limit']:
 assert reason(envelope(token))=={'reasonCode':token,'parserStage':'classified'}
 assert reason(envelope(token+':/private/path SELECT token=secret'))=={'reasonCode':token,'parserStage':'classified'}
for bad in ['/private/path','SELECT secret FROM credentials','native_x11_unknown:secret','native_x11_request /secret','x'*4097,3,None]:assert reason(envelope(bad))['reasonCode']=='unclassified'
for bad in [{},[],None,{'schemaVersion':1,'code':'input_effect_uncertain','message':'input_failed','retryable':False},{'error':envelope('input_failed')},dict(envelope('input_failed'),schema_version=True),dict(envelope('input_failed'),retryable=0),dict(envelope('input_failed'),retryable=True),dict(envelope('input_failed'),code='unknown')]:assert reason(bad)=={'reasonCode':'unclassified','parserStage':'schema_rejected'}
print('Native owner failure closed-code diagnostics and private-suffix negatives PASS')

assert reason(envelope('/private/path'))['parserStage']=='unknown_token'
assert reason(envelope(None))['parserStage']=='message_rejected'
