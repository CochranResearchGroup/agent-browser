export function assertTerminalRetirementState(terminalState, browserId) {
  const terminalBrowser = terminalState.browsers?.[browserId];
  if (terminalBrowser !== undefined) {
    throw new Error(
      `Terminal Service State is incoherent: terminal browser operational row remains ${JSON.stringify(terminalBrowser)}`,
    );
  }
  if (terminalState.browserProcessIdentities?.[browserId]) {
    throw new Error('Terminal Service State is incoherent: browser process identity remains');
  }

  const lifecycle = terminalState.runtimeOwnerRegistry?.lifecycleRecords?.[browserId];
  if (
    lifecycle?.lifecycleState !== 'terminal' ||
    lifecycle?.cleanupObligationState !== 'satisfied'
  ) {
    throw new Error(`Terminal Service State terminal lifecycle is incoherent: ${JSON.stringify(lifecycle)}`);
  }

  const receipts = Object.values(terminalState.abandonedBrowserRetirements ?? {}).filter(
    (transaction) => transaction.plan?.browserId === browserId && transaction.receipt,
  );
  if (receipts.length !== 1) {
    throw new Error(`Expected one terminal retirement receipt: ${JSON.stringify(receipts)}`);
  }
}
