# Shared-profile download scope and CSV cancellation

Date: 2026-09-07. Read-only consumer investigation against Agent Browser source `7e329476b5e69fa3c9f6c12f5608647d92712431`. No browser connection, download-setting change, runtime mutation, or source repair was performed for this note. This is a separate download incident from note 0156's tab ownership and release investigation.

## Reported symptom and evidence boundary

The SoyLei Plan0137 owner reported that clicking the actual **Export filtered CSV** control in its attributed Default-profile tab emitted a Playwright download named `communication-activity.csv`, followed by `failure() == "canceled"`. The generated URL was a Blob URL. The owner reported the same symptom in an earlier managed attempt. Persistent service and Playwright CDP connections were both retained; this investigation did not call either connection.

The cancellation is observed, but its cause is not established. In the installed Playwright source at `~/.dev-browser/node_modules/playwright-core/lib/server/chromium/crBrowser.js:224`, a Chrome `Browser.downloadProgress` event with state `canceled` becomes download failure `canceled`. It is not evidence, by itself, that CSV data generation or `saveAs` filename handling failed.

## Source findings

- `cli/src/native/service_file_transfer.rs:412` validates a download selector and an allowed output directory. Its default `captureMode` is `fetch`, which evaluates the selected element's `href` in the target page and fetches it with credentials included (`:645`). It does not click an export button, execute its application handler, or construct an authenticated form POST.
- Explicit `captureMode: "browser"` calls `BrowserManager::set_download_behavior` before clicking (`service_file_transfer.rs:433`). `cli/src/native/browser.rs:2413` sends `Browser.setDownloadBehavior` with `allowAndName`, a directory, and events enabled, but **without `browserContextId`**. Sending the command through a target CDP session does not make this Browser-domain setting per-target. The [CDP contract](https://chromedevtools.github.io/devtools-protocol/tot/Browser/#method-setDownloadBehavior) states that omission selects the default browser context. The inspected implementation does not restore a previous setting afterward.
- The native capture loop accepts browser-level download begin/progress events without matching their frame or GUID to this requested target (`service_file_transfer.rs:478–513`). Page-level events are session-filtered; browser-level events are not. A concurrent peer download can therefore be a candidate for misattribution. No peer contamination was demonstrated in this incident.
- Installed Playwright `crBrowser.js:301` also sets download behavior during browser-context initialization. `chromium.js:80` creates a connection-specific artifact directory and removes it during disconnect cleanup. Competing context settings or artifact-directory lifetime are plausible diagnostic leads, not verified causes here.

## SoyLei export behavior

The canonical contractor plugin's `assets/contractor-admin.js:764` posts FormData to `adminConfig.ajaxUrl` with action `soylei_contractor_staff_communication_activity`, nonce, Company public ID, and query fields. It receives JSON `payload.data.activity`; there is no server CSV response or stable CSV download endpoint in this handler.

The export handler (`:795`) requests page 1 with `perPage:100`, then up to 50 pages, and constructs quoted CSV cells, filter metadata, headers, and event rows locally. It creates a Blob URL, clicks a temporary anchor, removes it, and synchronously revokes the URL (`:847–854`). Immediate revocation is a possible timing lead, not a proven product bug. No change to that lifetime has been tested here.

## Safe next bounded investigation

1. Retain the current exact service handle and target. Do not attach another Playwright client or reset the shared default context's download behavior merely to collect evidence.
2. Before one authorized real export click, attach passive response observation to that exact page. Retain only responses whose request is the expected admin AJAX POST, action, owned Company, query filters, and page sequence. Keep nonce, cookies, and returned customer data private; do not print request bodies.
3. Save bounded exact JSON response bodies and their hashes, page counts, filters, and event IDs. Verify the server's authorized result and pagination independently. Label this **server Activity JSON evidence**, not a CSV download or CSV-serialization pass.
4. Retain the actual download event, GUID, frame identity when available, URL class, filename, and terminal result. A canceled native save remains failed even if the JSON is correct. A page-local diagnostic capture of the actual Blob bytes would be separate serialization evidence and must not be substituted for a working user download.
5. Qualify native download capture only in a service-authorized isolated context/browser, or after the service owner supplies a scoped procedure that prevents default-context interference and correlates the exact download. The current inspected `file_transfer` browser mode does not provide that guarantee for a shared Default context. Do not switch profile identity, change peer settings, or create a duplicate Default browser to conceal this gap.

## Owner follow-up

Reproduce with two peer tabs/connections under service custody: attribute begin/progress/completion by exact requested frame and GUID, demonstrate peer downloads cannot satisfy this request, and establish an explicit download-policy ownership boundary. Independently compare the actual application Blob lifetime under an exclusive, correctly configured browser before assigning the cancellation to the product. No repair or native-download acceptance is claimed by this note.

## Follow-up: verified PrivateTmp visibility mismatch

The owner read the existing Playwright in-process server object through `browser._connection.toImpl(browser)` without reconnecting or changing settings. It reported `options.downloadsPath == options.artifactsDir == /tmp/playwright-artifacts-rRW5hp`, with the default context ID undefined. Installed `inProcessFactory.js:48–53` exposes this client-to-server object mapping.

Read-only `/proc` inspection then established:

- Owned Playwright Node PID 23010, start ticks 14985001, UID 1000, mount namespace `mnt:[4026532222]`.
- Owned Chrome PID 66046, unchanged start ticks 8078423, UID 1000, mount namespace `mnt:[4026535173]`.
- Node-visible `/proc/23010/root/tmp/playwright-artifacts-rRW5hp` exists, is not a symlink, is empty, has UID/GID 1000/1000, mode 0700, device 2096, inode 7968526.
- Chrome-visible `/proc/66046/root/tmp/playwright-artifacts-rRW5hp` is absent, including no symlink. Chrome's `/tmp` parent has UID/GID 1000/1000, mode 1777, device 2096, inode 3158782; the Node `/tmp` inode is 73729.

The configured download directory is therefore genuinely invisible to Chrome in its retained PrivateTmp namespace. This is stronger evidence than the earlier timing/settings hypotheses, but a successful native download after correction has not yet been observed by this investigation.

A bounded owner-reviewed diagnostic can recheck the exact PID/start token, namespace, selected directory, and absence, then exclusively create only that generated basename through the Chrome `/proc/PID/root/tmp` path with mode 0700 as UID 1000. This changes no download behavior, profile, or peer CDP settings. Do not use an existing arbitrary directory, follow a symlink, or create paths selected only by age. No directory was created by this investigation.

The filesystem mismatch also affects artifact retrieval: the same pathname in Node and Chrome denotes different backing directories. If Chrome completes the download, Playwright's ordinary local `saveAs` may still look in the empty Node-side directory. The exact completed GUID must be correlated to the requested frame and read through the Chrome namespace into an owned private receipt, or the service owner must provide a reviewed namespace-aware artifact bridge. Do not claim ordinary Playwright artifact transfer is repaired merely because directory creation prevents cancellation.

## Failed bounded directory repair: deleted namespace backing directory

The campaign owner subsequently attempted only the exact missing artifact directory creation. It failed with `ENOENT`, even though the previously inspected Chrome-visible `/tmp` parent remained stat-able. Follow-up owner readback found that `/proc/66046/root/tmp` (inode 3158782) has `st_nlink == 0`: the retained browser's `/tmp` is an unlinked backing directory. The requested artifact directory remains absent. No download setting changed and no successful repair occurred. The owner retained `artifact-namespace-repair-failed.json` under the private browser receipt.

The origin of the parent deletion is not proven. Cleanup of an earlier service generation's PrivateTmp directory is a hypothesis only. This is now a retained runtime namespace lifecycle problem for the Agent Browser owner to diagnose. Do not respond by restarting the retained browser, remounting its namespace, or changing shared-context download settings without a separately reviewed ownership-preserving repair. Native CSV download remains unaccepted.
