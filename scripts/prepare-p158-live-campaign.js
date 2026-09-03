#!/usr/bin/env node

import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { isAbsolute, join, resolve } from 'node:path';

import { canonicalJson, sha256 } from './lib/p158-campaign-controller.js';
import {
  createP158LiveCampaignDescriptor,
  prepareP158RuntimeIdentityProbe,
  sealP158LiveBundleAssemblyConfiguration,
} from './lib/p158-live-campaign-assembly.js';

function argument(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1];
}

const inputPath = argument('--input');
const outputDirectory = argument('--output-dir');
if (!isAbsolute(inputPath ?? '') || !isAbsolute(outputDirectory ?? '')) {
  throw new Error('Usage: prepare-p158-live-campaign --input <absolute-json> --output-dir <absolute-directory>');
}
const input = JSON.parse(await readFile(inputPath, 'utf8'));
if (resolve(input.runRoot) !== resolve(outputDirectory) || input.production === true) {
  throw new Error('Preparation output must be the exact development campaign run root');
}
await mkdir(outputDirectory, { recursive: true });
const configuration = sealP158LiveBundleAssemblyConfiguration(input.assemblyConfiguration);
const configurationPath = join(outputDirectory, 'freeze', 'live-bundle-assembly-config.json');
await mkdir(join(outputDirectory, 'freeze'), { recursive: true });
await writeFile(configurationPath, canonicalJson(configuration), { flag: 'wx' });
const assemblyConfiguration = { path: configurationPath, sha256: sha256(await readFile(configurationPath)) };
if (!Array.isArray(input.runtimeIdentityProbeSpecifications) || input.runtimeIdentityProbeSpecifications.length === 0 ||
    input.runtimeIdentityProbes !== undefined) {
  throw new Error('Preparation requires fresh runtimeIdentityProbeSpecifications and refuses precomputed probe observations');
}
const runtimeIdentityProbes = await Promise.all(input.runtimeIdentityProbeSpecifications.map((specification) =>
  prepareP158RuntimeIdentityProbe({
    ...specification,
    runRoot: input.runRoot,
    candidateExecutablePath: input.candidateExecutablePath,
  })));
const descriptorInput = { ...input, runtimeIdentityProbes, assemblyConfiguration };
delete descriptorInput.runtimeIdentityProbeSpecifications;
const generated = createP158LiveCampaignDescriptor(descriptorInput);
const descriptorPath = join(outputDirectory, 'p158-live-campaign.json');
await writeFile(descriptorPath, canonicalJson(generated.descriptor), { flag: 'wx' });
process.stdout.write(`${JSON.stringify({
  descriptorPath,
  descriptorSha256: sha256(await readFile(descriptorPath)),
  configurationPath,
  configurationSha256: assemblyConfiguration.sha256,
})}\n`);
