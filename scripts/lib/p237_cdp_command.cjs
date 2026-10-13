// Independent protocol observation from a task-owned Chrome endpoint.
'use strict';
const socket = new WebSocket(process.argv[2]);
const deadline = setTimeout(() => {
  socket.close();
  process.exitCode = 1;
}, 5000);
socket.addEventListener('open', () => socket.send(JSON.stringify({
  id: 1, method: process.argv[3], params: JSON.parse(process.argv[4]),
})));
socket.addEventListener('message', (event) => {
  const reply = JSON.parse(event.data);
  if (reply.id !== 1) return;
  clearTimeout(deadline);
  if (reply.error) process.exitCode = 1;
  process.stdout.write(JSON.stringify(reply) + '\n');
  socket.close();
});
socket.addEventListener('error', () => {
  clearTimeout(deadline);
  process.exitCode = 1;
});
