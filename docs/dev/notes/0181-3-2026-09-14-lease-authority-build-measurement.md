# Plan 0181 Lease Authority Build Measurement

Date: 2026-09-14

Product lane: PL-PLATFORM

Disposition: active-input

Owning plan or work item: Plan 0181 and issue #99

Related lanes: P181, P182

## Scope And Frozen Inputs

This receipt records the complete P181 P6 benchmark packet. It compares
baseline `16d4fb22dfd8cf96cd7e65edfd0b66fe54953945` with joined candidate
`dd7f2f1c36f0d409910cce2c5e539f5c59530f0c`. The candidate includes current
`main`, the P182 adjacent-revision convergence fixture, and the tracked
workspace lockfile registration for `agent-browser-lease-authority`.

The host was Ubuntu 24.04 under WSL2 kernel 6.6.87.2 on an AMD Ryzen 9
7950X3D with 20 online CPUs. The toolchain was rustc 1.94.1
`e408947bf 2026-03-25`, LLVM 21.1.8, and Cargo 1.94.1
`29ea6fb6 2026-03-24`. Every compiling invocation ran serially through
`scripts/ci/cargo-safe.sh` with eight Cargo jobs,
`AGENT_BROWSER_CARGO_CACHE=off`, and
`AGENT_BROWSER_CARGO_FAST_LINKER=off`. The wrapper retained its 20 GiB
`MemoryHigh`, 24 GiB `MemoryMax`, 4 GiB `MemorySwapMax`, and aggregate slice
limits. Cold checks also set `CARGO_INCREMENTAL=0`.

Detached worktrees and isolated target directories kept baseline and candidate
artifacts separate. Each measured warm run changed one benchmark-only comment
token in the owning authority source, then used the already primed target.
Focused order alternated AB and BA across five pairs. No benchmark token is
present in the candidate branch.

## Commands And Selection

The baseline focused command was:

```text
scripts/ci/cargo-safe.sh test --timings --manifest-path cli/Cargo.toml service_lease_authority -- --test-threads=1
```

It ran 106 legacy filtered tests. The candidate focused command was:

```text
scripts/ci/cargo-safe.sh test --timings -p agent-browser-lease-authority --manifest-path Cargo.toml -- --test-threads=1
```

It ran 108 crate tests: 103 mapped crate invariants and five additive identity
tests. Three retained CLI adapter invariants passed separately through
`scripts/ci/rust-tests.sh --focused service_lease_authority_adapter`; that
validation compiled in 3 minutes 33 seconds and ran the tests in 0.02 seconds.
It is correctness evidence, not a P6 timed sample.

Downstream runs used `cargo check --workspace` through the wrapper. Cold runs
used the same workspace check in fresh targets with incremental compilation
disabled. Cargo timing reports remain under each isolated target's
`cargo-timings/` directory. Raw `/usr/bin/time -v` receipts remain at
`/tmp/agent-browser-p181-benchmark-receipts-dd7f2f1c` on the measurement host.

## Focused Edit Loop

| Pair | Baseline wall | Baseline max RSS KiB | Candidate wall | Candidate max RSS KiB |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 166.79 s | 3,774,116 | 14.67 s | 512,464 |
| 2 | 157.06 s | 3,380,888 | 13.55 s | 320,644 |
| 3 | 150.53 s | 3,374,796 | 12.04 s | 317,108 |
| 4 | 146.64 s | 3,376,016 | 12.11 s | 317,312 |
| 5 | 146.74 s | 3,375,400 | 12.01 s | 317,756 |
| Median | 150.53 s | 3,376,016 | 12.11 s | 317,756 |

The candidate median was 8.05 percent of baseline, a measured wall-time
reduction of 91.95 percent. Median maximum RSS was 9.41 percent of baseline, a
reduction of 90.59 percent. All ten measured invocations exited successfully
and stayed inside the wrapper resource contract.

This is strong evidence that the independently compiled crate sharply shortens
the practical authority edit loop. It is not the plan's formal acceleration
promotion receipt because the frozen commands did not execute a literally
identical test selection. The candidate ran five additive tests while the
three retained adapter tests were validated separately. No sample allowance
remains to replace the frozen comparison, so P181 reports the measured
improvement and withholds the stricter promotion claim.

## Downstream And Cold Safety

| Pair | Baseline wall | Baseline max RSS KiB | Candidate wall | Candidate max RSS KiB |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 8.39 s | 1,494,372 | 8.07 s | 1,396,620 |
| 2 | 8.76 s | 1,515,124 | 8.45 s | 1,393,240 |
| 3 | 8.75 s | 1,515,304 | 8.20 s | 1,393,540 |
| Median | 8.75 s | 1,515,124 | 8.20 s | 1,393,540 |

The candidate downstream median was 6.29 percent faster and used 8.02 percent
less median maximum RSS. It did not breach the plan's 10 percent regression
limit. Pair 2 required the packet's single replacement allowance: a candidate
command was accidentally started against the baseline target, admitted, and
interrupted with exit 130 after 33.99 seconds. Its receipt remains preserved
and excluded. The correct baseline replacement then restored the baseline
source in that target and passed. Pairs 1 and 3 independently show no
downstream regression.

| Cold variant | Wall | Maximum RSS KiB |
| --- | ---: | ---: |
| Baseline | 57.36 s | 2,624,060 |
| Candidate | 54.22 s | 2,432,160 |

The directional cold candidate was 5.47 percent faster and used 7.31 percent
less maximum RSS. This single pair is safety evidence, not a general clean-build
performance claim.

The unmeasured focused primes were 299.97 seconds and 5,970,836 KiB for the
baseline and 484.51 seconds and 695,200 KiB for the candidate. Candidate prime
wall time includes more than seven minutes waiting for wrapper admission. The
unmeasured downstream primes were 66.23 seconds and 2,712,128 KiB for the
baseline and 62.11 seconds and 2,517,700 KiB for the candidate. Prime results
are preparation evidence only.

## Invocation And Correctness Ledger

The packet consumed exactly 23 admitted compiling invocations:

- four primes;
- ten focused measurements;
- six intended downstream measurements;
- one admitted and interrupted invalid downstream invocation, consuming the
  one replacement allowance; and
- two cold measurements.

No benchmark budget remains. Outside the timed packet, all 108 crate tests,
the three retained adapter tests, the five joined P182 `prepared_` tests,
formatting, and workspace strict Clippy passed locally. Native-Linux CI run
34801395695 passed the comprehensive Rust and workstation lanes at
source-equivalent checkpoint `38012fd2`. Run 34802213548 passed every Lease
Authority compartment at hardened checkpoint `3c7e2bcc` and failed only the
unrelated production-scale Service Store timing fixture at 502 ms against its
500 ms threshold; the immediately preceding run passed that fixture.

## Raw Receipt Digests

The following SHA-256 values make the temporary raw receipts attributable:

```text
8b57a0c4ba289ae70239e31a1a3b9ed40b1d5f78c8cb439f815e87cb59bc4180  cold-baseline.time
79bbfd0b86c25c2fcd2c0e95b58aae1d538c09643e038d0fc8e4bd0f22b41a3a  cold-candidate.time
7cd0ca9f60083f4411265486af0e93338309fa1be3b6cd115ec1feabc918ac36  downstream-pair1-baseline.time
e4609a113e6a4c937d0954fcb0bda5b26197a95363663e79e8a23b5d79f91525  downstream-pair1-candidate.time
04d9e565b3959cef12be583ba89b2a39034a4ee2df6585d7bd46ccb75682fc4d  downstream-pair2-baseline.time
9c8fc08869debbd15b6a3c1dfc4f48f13e52ab4a077906ce54d8042915865783  downstream-pair2-candidate.time
48f72926d817fb0c2ac1223476d37fac76c451566e4dfb01e49c18f4d7ef7d8a  downstream-pair2-invalid-candidate-in-baseline-target.time
f89d1584a8581f8136177d716580131ad1f418474a7e9a8900e7686593c962a7  downstream-pair3-baseline.time
53ffbfc9019f7f1407e12d57d3a495d5b0eaba271b6ba2edcb7bcfa3baf162b9  downstream-pair3-candidate.time
d85d02eb1da6332dd2bab9773605943a5cbb4aa9f4aac800b88f7bdcbd95d2ba  focused-pair1-baseline.time
5e93cc55d832c2b63d8764879e386fea6a9360c1dd2f07b6a772a34a0e3758d2  focused-pair1-candidate.time
7c6924460646ff171d7173150b9c342d583a77328628e24efed0c16721cf8e5d  focused-pair2-baseline.time
650bbb8af139ce5722099727dbf6ba9b4bb7e8672f6ec5014b9487fb32c73def  focused-pair2-candidate.time
be67a92b5d12609fc349098f627176e6b7c09d3d030a22fb9f8c7a471b86ecf3  focused-pair3-baseline.time
77ccf8097d7f704db9e9d7877af02e98924d407b3a76d90a6c4bff335115dda2  focused-pair3-candidate.time
df0e19b92aaa62131e3fc1f7933759f8cbb7c577ea2f99bd1777e90cf671542d  focused-pair4-baseline.time
a06a8811607df0a175da6c71ca200f1478429a52f9866601aee6e6d3d4aaac41  focused-pair4-candidate.time
a5c4506b8930d9fb8da2789d32dfe2e7199281af9fa5235dcfba16732b295a5b  focused-pair5-baseline.time
50ce6e846dc64be62e88c087cf06b5fe3516174f048e12419427fa60ecbb5132  focused-pair5-candidate.time
7c8d0a34c42acc4df91345c1c5fdb402a1e43f72925a441d1216421aeb1  prime-baseline-downstream.time
cc5696dab5cece02c9674a5b7e0c0dfc7b04f0d3b669b0447fa00d3826e7481a  prime-baseline-focused.time
89e3907b09d86e921a3c2c07e96af3d9f2d257d0af8f8c1d85305044ded97e65  prime-candidate-downstream.time
b2a0f51130946274d6aa1d6b5d5a8389db730126b8b3a5c4fa9dc49611ec13fd  prime-candidate-focused.time
```

## Disposition

P6 is complete at its exact invocation ceiling. Correctness, focused-loop
improvement, downstream safety, and directional cold safety are supported.
The strict acceleration-promotion claim is withheld because test-selection
identity was not exact. P181 remains open for target-platform CI, final review,
and PR integration. This note authorizes no runtime, browser, profile, provider,
installation, production, release, or Plan 0144 acceptance effect.
