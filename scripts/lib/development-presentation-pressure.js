import { readFileSync } from 'node:fs';
import { cpus } from 'node:os';

const MIN_AVAILABLE_MEMORY_BYTES = 8 * 1024 ** 3;
const MIN_FREE_SWAP_BYTES = 1024 ** 3;
const MIN_IDLE_CPU_FRACTION = 0.1;
const MIN_IDLE_CPU_CORES = 1;
const MAX_IO_WAIT_FRACTION = 0.1;
const MAX_FILE_HANDLE_FRACTION = 0.8;
const CPU_SAMPLE_MS = 1_000;

/** Read one bounded Linux host-pressure snapshot for elastic presentation admission. */
export function sampleDevelopmentPresentationPressure({
  readFile = readFileSync,
  cpuCount = cpus().length,
  wait = waitMilliseconds,
} = {}) {
  const sampleErrors = [];
  const firstCpu = readText(readFile, '/proc/stat', sampleErrors);
  wait(CPU_SAMPLE_MS);
  const secondCpu = readText(readFile, '/proc/stat', sampleErrors);
  const cpu = cpuDelta(firstCpu, secondCpu);
  const meminfo = readText(readFile, '/proc/meminfo', sampleErrors);
  const loadavg = readText(readFile, '/proc/loadavg', sampleErrors);
  const fileNr = readText(readFile, '/proc/sys/fs/file-nr', sampleErrors);
  const fileParts = fileNr.trim().split(/\s+/).map(Number);
  return {
    memoryAvailableBytes: meminfoBytes(meminfo, 'MemAvailable'),
    swapFreeBytes: meminfoBytes(meminfo, 'SwapFree'),
    swapTotalBytes: meminfoBytes(meminfo, 'SwapTotal'),
    loadOne: finiteNumber(Number(loadavg.split(/\s+/)[0]), Number.POSITIVE_INFINITY),
    cpuCount: Math.max(1, finiteNumber(cpuCount, 1)),
    cpuSampleAvailable: cpu.available,
    cpuIdleFraction: cpu.idleFraction,
    ioWaitFraction: cpu.ioWaitFraction,
    cpuSampleMs: CPU_SAMPLE_MS,
    fileHandlesAllocated: finiteNumber(fileParts[0], 0),
    fileHandlesMaximum: finiteNumber(fileParts[2], 0),
    sampleErrors: [...new Set(sampleErrors)],
  };
}

/**
 * Evaluate elastic presentation capacity from one typed host-pressure snapshot.
 * Fresh CPU headroom is authoritative when available. Load average is retained
 * as a diagnostic and as a fail-closed fallback for platforms without a usable
 * CPU sample.
 */
export function evaluateDevelopmentPresentationPressure(descriptor, readings) {
  const reasons = [];
  const cpuCount = Math.max(1, finiteNumber(readings.cpuCount, 1));
  const requiredIdleCpuCores = Math.max(
    MIN_IDLE_CPU_CORES,
    cpuCount * MIN_IDLE_CPU_FRACTION,
  );
  const cpuSampleAvailable = readings.cpuSampleAvailable === true &&
    Number.isFinite(readings.cpuIdleFraction) &&
    Number.isFinite(readings.ioWaitFraction);
  const cpuIdleFraction = cpuSampleAvailable
    ? clampFraction(readings.cpuIdleFraction)
    : null;
  const ioWaitFraction = cpuSampleAvailable
    ? clampFraction(readings.ioWaitFraction)
    : null;
  const idleCpuCores = cpuIdleFraction === null ? null : cpuIdleFraction * cpuCount;

  if (readings.memoryAvailableBytes < MIN_AVAILABLE_MEMORY_BYTES) {
    reasons.push('memory_reserve');
  }
  if (readings.swapTotalBytes > 0 && readings.swapFreeBytes < MIN_FREE_SWAP_BYTES) {
    reasons.push('swap_reserve');
  }
  if (cpuSampleAvailable) {
    if (idleCpuCores < requiredIdleCpuCores) reasons.push('cpu_capacity');
    if (ioWaitFraction > MAX_IO_WAIT_FRACTION) reasons.push('io_pressure');
  } else if (readings.loadOne > cpuCount * (1 - MIN_IDLE_CPU_FRACTION)) {
    reasons.push('cpu_load');
  }
  if (readings.fileHandlesMaximum > 0 &&
      readings.fileHandlesAllocated / readings.fileHandlesMaximum > MAX_FILE_HANDLE_FRACTION) {
    reasons.push('file_handle_reserve');
  }

  return {
    admittedMaximum: reasons.length === 0 ? descriptor.hardMaxSlots : descriptor.warmSlots,
    reasons,
    readings: {
      ...readings,
      cpuCount,
      cpuSampleAvailable,
      cpuAdmissionSource: cpuSampleAvailable ? 'sampled_idle_headroom' : 'load_average_fallback',
      cpuIdleFraction,
      idleCpuCores,
      requiredIdleCpuCores,
      ioWaitFraction,
    },
  };
}

function finiteNumber(value, fallback) {
  return Number.isFinite(value) ? value : fallback;
}

function clampFraction(value) {
  return Math.max(0, Math.min(1, value));
}

function cpuDelta(first, second) {
  const before = cpuTicks(first);
  const after = cpuTicks(second);
  if (!before || !after) {
    return { available: false, idleFraction: null, ioWaitFraction: null };
  }
  const total = after.total - before.total;
  const idle = after.idle - before.idle;
  const ioWait = after.ioWait - before.ioWait;
  if (total <= 0 || idle < 0 || ioWait < 0) {
    return { available: false, idleFraction: null, ioWaitFraction: null };
  }
  return {
    available: true,
    idleFraction: idle / total,
    ioWaitFraction: ioWait / total,
  };
}

function cpuTicks(contents) {
  const line = contents.split(/\r?\n/).find((candidate) => candidate.startsWith('cpu '));
  const ticks = line?.trim().split(/\s+/).slice(1, 9).map(Number);
  if (!ticks || ticks.length < 8 || ticks.some((value) => !Number.isFinite(value))) return null;
  return {
    total: ticks.reduce((sum, value) => sum + value, 0),
    idle: ticks[3],
    ioWait: ticks[4],
  };
}

function meminfoBytes(contents, field) {
  const line = contents.split(/\r?\n/)
    .find((candidate) => candidate.startsWith(`${field}:`));
  const kibibytes = Number(line?.match(/:\s+([0-9]+)/)?.[1] || 0);
  return kibibytes * 1024;
}

function readText(readFile, path, sampleErrors) {
  try {
    return readFile(path, 'utf8');
  } catch {
    sampleErrors.push(`read_unavailable:${path}`);
    return '';
  }
}

function waitMilliseconds(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
