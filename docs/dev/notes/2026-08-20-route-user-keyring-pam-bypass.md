# Route-user keyring PAM bypass

## Incident

The five-minute workstation runtime interlock reconciled the existing
`agent-browser-rdp-a` and `agent-browser-rdp-b` route users through bare
`chpasswd`. On Ubuntu, bare `chpasswd` uses PAM. The host's `common-password`
stack includes `pam_gnome_keyring`, which emitted `no old password was entered`
and raised a keyring prompt even though every managed Chrome process used
`--password-store=basic --use-mock-keychain`.

The Chrome keyring posture and the Linux route-user password path are separate
authorities. A browser `basic_password_store` setting cannot suppress a PAM
prompt raised by privileged account maintenance.

## Repair contract

- The privileged helper uses
  `chpasswd --crypt-method SHA512 --sha-rounds 100000`, which preserves stdin
  password transport while bypassing PAM.
- `status-json` publishes `routeUserCredentialUpdate` with `pamBypassed=true`,
  `cryptMethod=SHA512`, and `shaRounds=100000`.
- The capability object intentionally uses `Credential` rather than `Password`.
  The installed 0.28.0 doctor redacts any output line containing `password`
  before parsing JSON, so the former object name made a valid status payload
  appear stale even though it contained no credential value.
- The helper remains in the established
  `2026-06-23.p44-route-desktop-v4` compatibility family so the selected
  installed generation can execute it safely while the new capability fields
  let current source reject prompt-producing predecessors.
- Both the Rust helper compatibility evaluator and the source installer require
  those fields, so the formerly compatible prompt-producing helper is no longer
  accepted as ready.
- The helper contract guard rejects a return to bare `chpasswd`.

## Validation and installed state

- RED: `pnpm test:rdp-route-xsession` failed on the missing non-PAM command.
- GREEN: `pnpm test:rdp-route-xsession`.
- GREEN: `pnpm test:install-privileges-clean-fixture`.
- GREEN: `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml remote_view_helper_contract -- --test-threads=1`.
- GREEN: `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml remote_view_helper_status_contract -- --test-threads=1`.
- RED then GREEN: `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml doctor_redaction_preserves_typed_route_user_credential_contract -- --test-threads=1`.
- GREEN: `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`.
- The installed root-owned helper remains pending an interactive `sudo -v`
  boundary. The user-level runtime-interlock timer was stopped while retaining
  its enabled state, preventing another five-minute run of the old helper until
  installation and live reconciliation proof are complete.
