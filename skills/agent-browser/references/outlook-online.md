# Outlook Online SiteGuide

Guide version 4. Use with an existing, authorized Outlook on the web profile.
The browser must show the requested mailbox and its Outlook application shell
before reading tenant data. In Iowa State, resolve the registered account through
Agent Browser profile lookup for service `outlook`. Reuse the durable named
profile with `profileMode: "existing"`. Read the
[Iowa State sign-in guide](authentication.md#iowa-state-outlook-sign-in) if
Microsoft requests authentication.

Use one Agent Browser task for the sequence. After each navigation, take a
fresh semantic snapshot and scope controls to the app rail or active workspace.
Use the `left-rail-appbar` navigation for Mail, Calendar, and People. Resolve
**More apps** beside the rail and require one enabled control. A second,
disabled **More apps** was present in Mail search. In Groups, content cards
also have **Mail** buttons. Stop if the account, origin, enabled control, or
destination is ambiguous. Keep private query values and results out of
shared evidence.

For each read, retain the exact browser task, requested account, visible
workspace, and query or date range privately. On a timeout, observe the same
task before issuing more input. If a transition changed the view, continue
from that view. If the effect remains unknown, report the unresolved step.

## Mail search

1. Select **Mail** in the app rail. Verify the account root in the folder tree.
2. Resolve the header combobox **Search for email, meetings, files and more.**
   Check the folder scope and filters. Use the requested scope or report the
   visible default, such as **All folders**.
3. Enter the query and press **Enter** while the search box owns focus. Verify
   the selected **Search** tab, the exact query, and the **Results** heading.
4. Read only result rows in the visible Results region. Wait for readable
   results or an explicit empty state before reporting matches or zero results.
   Count only rendered matches unless Outlook reports a total. Capture sender,
   subject, and date from rows only when requested. Opening a message can mark
   it read. When the task is search-only, return the row evidence without opening
   messages. If interrupted, inspect the current query and scope before
   submitting again.

## New mail

1. In the intended mailbox, select **New mail**. Locate the active composer
   by **Subject**, **Message body**, and its own **Send** and **Discard**
   actions. An empty composer showed **Send** disabled.
2. Resolve the chosen sender or mailbox. Add recipients in **To**, **Cc**, or
   **Bcc** as requested. Verify each resolved address in the correct field.
   Shared display names require an exact address before selection.
3. Enter the approved subject and body. If attachments are requested, use
   **Attach file** and wait for each intended attachment to appear on the
   composer. Compare filenames and inspect any upload error before proceeding.
4. For a draft request, verify the saved draft in **Drafts** by mailbox,
   recipients and subject. For an authorized send, review sender, recipient
   fields, subject, body and attachments before pressing **Send** once.
   Inspect acknowledgement and the exact message in **Sent Items** before
   reporting success. If the response is uncertain, look in **Sent Items**
   and **Drafts** before attempting more input.
5. For inspection, select **Discard** on the composer opened for the check.
   On **Discard message**, select **OK** and verify the Mail workspace returns.

## Calendar

1. Select **Calendar** in the app rail. Verify the main **calendar view**
   landmark, visible date range, and selected calendar names in the sidebar.
2. Use **Go to today**, **Go to previous week**, or **Go to next week** when
   shown. After navigation, verify the range changed as intended before
   reading event summaries. If another view is active, use its displayed
   date navigation controls and verify the actual range.
3. Read event summaries inside the calendar view. Bind a requested event by
   calendar, date, start time and title before opening its details. Recheck
   those fields when multiple calendars or recurring occurrences share a title.
   Report the range and any visible time zone. An empty visible range requires
   an observed empty state or a complete rendered range.
4. Use only the event's read surface. If Outlook offers **Edit**, **Delete**,
   or an RSVP, leave those controls alone and close the details after reading.

## Teams meeting composer

1. In **Calendar**, select **New event**. Verify the composer by its
   **Add title** field, calendar picker, and **Save** or **Send** action.
   Check the account in the calendar picker and the destination calendar
   before entering meeting information.
2. Enter the requested title and time range. Expand the date/time control to
   verify its dates and time zone if the summary is ambiguous. Use
   **Open Scheduling Assistant** to review a proposed time when appropriate.
   For an invitation, resolve each attendee to the requested address before
   selecting a suggestion.
3. In the composer, enable the **Microsoft Teams meeting** switch and verify
   its accessible state is checked. A checked switch establishes that the
   composer is configured for Teams. It does not verify a join link.
4. For an authorized creation, review the destination calendar, event
   title, dates, time zone, attendees, and Teams switch before using the
   visible **Save** or **Send** action once. If the response is uncertain,
   check the destination calendar for the exact event before attempting
   another submission.
5. Reopen the saved event by calendar, date, start time, and title. Verify
   a Teams join action or a Microsoft Teams join URL in the saved event.
   A visible switch or a successful click is insufficient to report link
   creation. For inspection only, click **Close**, then **OK** in the
   **Discard event** prompt. Verify the calendar view returns.

## Groups

1. Open **More apps** in the rail and select **Groups**, or use its pinned
   rail button. If the launcher is unlabeled or the ribbon version is
   disabled, navigate the existing task to
   `https://outlook.cloud.microsoft/groups/home`. Verify **Search or discover
   groups** in the Groups workspace.
2. Choose the authority for the question. **Discover groups** lists groups
   available to discover. For an existing group, use the expanded **Groups**
   tree and resolve the exact group label under it. Inspect the tree's
   **Filter and sort groups list** control when the requested group is absent.
   The observed filters are **All**, **Owner**, and **Member**. Record the
   current filter before concluding the group is missing.
3. Select the unique group tree item. Verify the expanded group branch and
   its selected folder against the requested label. The observed group opened a
   workspace with **Email**, **Files**, and **Members** tabs. The heading can
   name its open folder, so confirm the selected tree branch as well.
4. To read group member information, select the **Members** tab on the
   verified group workspace. Wait for that tab to become selected and inspect
   the member region. A button labeled with a member count proves only the
   displayed count. If the task asks whether the signed-in account belongs,
   look for an enabled **Leave group** control on the verified group workspace.
   For another account, require an exact identity in the loaded member list.
   A selected tab without rows or an explicit empty state leaves the list
   unverified.
5. **Join**, **Create group**, and content-level **Mail** buttons are separate
   actions. Group discovery, group navigation, and verified membership are
   different outcomes.

## New group

1. In the Groups workspace, select **New group**. Verify the composer by its
   required **Name** field, **Description**, **Edit group settings**, and
   **Create**. The empty composer showed **Create** disabled.
2. Enter only the approved name and description. Inspect **Default settings**
   and **Edit group settings**. The **Edit settings** panel exposes **Privacy**,
   **Language for group-related notifications**, and **Subscription**.
   Verify all three before creation. The observed default was **Private** with
   **Subscription** checked, which sends group conversations and events to
   members' inboxes. Review the assigned address if Outlook displays one.
   Treat visibility, subscription, and intended membership as explicit choices.
3. For an authorized creation, check for an existing matching group and
   review the final settings before selecting **Create** once. Afterward,
   locate the exact group in the Groups tree and verify its name, address and
   privacy when visible. Handle a follow-up member prompt only within the
   authorized membership scope. If the reply is uncertain, inspect the tree
   and search before repeating creation.
4. For inspection, use **Discard** on the unsaved composer and verify that
   the Groups workspace returns.

## Edit group and membership

1. Select the exact group in the Groups tree. Verify its name and address or
   another stable identifier when names are ambiguous. In the group workspace,
   check that **Edit group** is enabled. Use the **Owner** filter to locate
   owned groups, then recheck the exact group's controls. One member group
   exposed **Edit group** as disabled while **Add members** was enabled.
   An owned group exposed two enabled **Edit group** controls. Scope the
   selection to the group ribbon.
2. The edit form exposes **Name**, **Description**, and **Privacy**. Verify
   its current values against the selected group. For an authorized change,
   update the requested fields, review them, and select **Save** once.
   Reopen the same group and verify the requested values. For inspection,
   select **Discard** and verify the group workspace returns. A disabled or
   missing edit action is a blocked edit in the current account or view.
3. Select **Members** to inspect membership. For additions, use the intended
   group's **Add members** action. Verify **Add members / View all members**,
   enter an address in **Add members**, and verify each resolved person before
   selecting **Add**. The empty panel showed **Add** disabled and offered
   **Close**. For removal or role changes, use the exact member row and verify
   the current role before changing it.
4. For a request to join, open the exact discovered group and inspect **Join**
   and any approval prompt. Treat a pending request as pending until membership
   is visible. For a request to leave, verify the signed-in account and group
   identity before **Leave group**. **Invite others** is a separate workflow
   from adding someone directly. Report the actual invite or membership state.
5. After an authorized submission, reopen the group and verify the exact
   added, removed, or role-changed identity where the roster is accessible.
   For the signed-in account, verify the resulting membership control or
   explicit state. A member count alone is insufficient. If input is
   uncertain, reconcile the group and roster before retrying.

## People and contact lists

1. Select **People** in the rail. Verify the **Your contacts** heading and the
   selected **Your contacts** tree item. Accessible labels may append a count.
2. Use **Search contacts and directory** for a name or address. Inspect the
   **Results** heading and search query together. **Results (0)** confirms an
   empty result for the active query. For a positive result, read the source
   label and compare the exact address. The observed exact-address search
   returned two rows carrying that same address under different field labels.
   Use the collection or source and the requested identity fields to resolve
   duplicates. **Work** and **Other** label address fields, not the row's
   source. Keep both candidates visible when the source and identity evidence
   cannot distinguish them.
3. For contact groups, select **Your contact lists** and verify that heading
   before reading list names. Contact lists are People objects. Microsoft 365
   Groups use the separate Groups workspace.
4. Return only the requested contacts or lists. Creating, editing, and deleting
   contacts or lists are outside this guide.

## Verify the outcome

| Request | Evidence to return |
| --- | --- |
| Search mail | Active query, folder scope, visible filters, and matching rows or an explicit empty state |
| Draft or send authorized mail | Exact composer or saved draft, or the matching sent message, with the intended sender, recipients, and subject |
| Read calendar | View range, selected calendars, and matching event evidence including the displayed time zone when available |
| Create an authorized Teams meeting | Exact saved event on the intended calendar and a verified Teams join action or join URL |
| Read a group | Exact group label, selected group branch, and the requested tab's loaded state |
| Create or edit an authorized group | Exact group identity and confirmed settings after reopening |
| Change authorized group membership | Exact member identity and verified resulting roster or role |
| Find a contact or contact list | Active search or collection, exact matching identity, and personal contact, directory, or list source |

Report the portion Outlook actually rendered when the view is filtered or
virtualized. Report **Blocked** for an account or access failure and
**Adaptation required** when a new layout prevents identifying the requested
control or evidence. A successful tool response only confirms dispatch.
Verify the visible postcondition before reporting completion.

## Qualification

Live Iowa State Outlook web, 2026-11-27 UTC, Agent Browser generation
`0.28.0-d9b2d98fe36a-aae0a7f148d7` with semantic snapshots, task-scoped
clicks, fill and Native Enter. Qualified application origin
`https://outlook.cloud.microsoft`. Source base `bc91cd60d`.

- Mail returned visible result rows for a bounded query with **All folders**
  selected, and a second query entered the Results view. **New mail** exposed
  the recipient, subject, body, attachment and send controls. The empty
  inspection draft was discarded through **Discard message → OK**.
  Draft saving and message delivery remain unverified.
- Calendar displayed the selected calendars and advanced the visible week.
  **New event** opened the intended account's composer. The Teams switch
  changed from unchecked to checked. The unsaved composer showed no join
  action. Saving, invitation delivery, and join-link verification remain open.
- People rendered **Results (0)** for a synthetic query and **Results (2)**
  for an exact-address query. Both rows exposed the same address.
  **Your contact lists** opened from the tree.
- **More apps → Groups** opened **Discover groups**. Selecting one group
  tree item opened its group workspace. The **Members** tab became selected
  and the group header showed a member count and **Leave group**. The complete
  member roster was not verified. **New group** exposed the name, description,
  privacy, notification language, and subscription fields. The account had
  **Edit group** disabled on the sampled group, while **Add members**
  opened a picker. An owned group opened its edit form from the ribbon and
  returned to the group workspace through **Discard**. No membership change
  or group save was submitted.
- A calendar event button was selected, but the snapshot did not establish
  an event-detail panel. Calendar detail verification remains open.

Use these agent-facing instructions with the standard browser task API.
Executable routine registration and calendar-sync acceptance are separate work.
