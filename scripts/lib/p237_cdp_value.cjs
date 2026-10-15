// Independent readback from an exact task-owned Chrome page, without attaching
// through the source runtime or replacing its command transport.
'use strict';
const socket = new WebSocket(process.argv[2]);
const deadline = setTimeout(() => {
  socket.close();
  process.exitCode = 1;
}, 5000);
socket.addEventListener('open', () => socket.send(JSON.stringify({
  id: 1, method: 'Runtime.evaluate',
  params: { expression: process.argv[3], returnByValue: true },
})));
socket.addEventListener('message', (event) => {
  const reply = JSON.parse(event.data);
  if (reply.id !== 1) return;
  clearTimeout(deadline);
  if (reply.error || reply.result?.exceptionDetails) process.exitCode = 1;
  else process.stdout.write(JSON.stringify(reply.result.result.value) + '\n');
  socket.close();
});
socket.addEventListener('error', () => {
  clearTimeout(deadline);
  process.exitCode = 1;
});
