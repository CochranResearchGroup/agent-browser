// A bounded real renderer workload for task-owned cross-client focus evidence.
'use strict';
const { writeFileSync } = require('node:fs');
const { randomUUID } = require('node:crypto');
const marker = `p237-cross-client-busy:${randomUUID()}`;
const socket = new WebSocket(process.argv[2]);
const deadline = setTimeout(() => {
  socket.close();
  process.exitCode = 1;
}, 10000);
socket.addEventListener('open', () => socket.send(JSON.stringify({ id: 1, method: 'Runtime.enable' })));
socket.addEventListener('message', (event) => {
  const reply = JSON.parse(event.data);
  if (reply.method === 'Runtime.consoleAPICalled'
      && reply.params.args.some((argument) => argument.value === marker)) {
    writeFileSync(process.argv[3], JSON.stringify({ state: 'running', observedAtMs: Date.now() }) + '\n', { mode: 0o600 });
  }
  if (reply.id === 1) {
    if (reply.error) {
      clearTimeout(deadline);
      process.exitCode = 1;
      socket.close();
      return;
    }
    socket.send(JSON.stringify({ id: 2, method: 'Runtime.evaluate', params: {
      expression: `(() => { console.info(${JSON.stringify(marker)}); const stopAt = Date.now() + 5000; while (Date.now() < stopAt) {} return true; })()`,
      returnByValue: true,
    } }));
  }
  if (reply.id === 2) {
    clearTimeout(deadline);
    if (reply.error || reply.result?.exceptionDetails) process.exitCode = 1;
    process.stdout.write(JSON.stringify(reply) + '\n');
    socket.close();
  }
});
socket.addEventListener('error', () => {
  clearTimeout(deadline);
  process.exitCode = 1;
});
