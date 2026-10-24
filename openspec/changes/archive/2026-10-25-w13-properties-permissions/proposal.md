# W13 — Properties and permissions

## Why

The listing exposes basic size/date/extension metadata, but users cannot inspect
full properties, calculate a folder's contents or change Unix permissions and
modified time from muCrab.

## What Changes

- Add Properties for the focused child, independently of unrelated marks.
- Show available local type, path, sizes, timestamps, owner/group IDs and names,
  Unix mode, link information and hard-link count without invented values.
- Calculate local folder logical bytes and entry counts asynchronously, with
  cancellation, link/mount boundaries and explicit partial/error totals.
- Review and apply non-recursive changes to the focused local entry's nine rwx
  permission bits and modified time, preserving unrelated metadata where supported.
- Show only available member metadata for ZIP entries, with edits disabled.

Inspection, size calculation and the two small metadata editors share one
properties UI and identity snapshot. Changing owner/group, recursive chmod,
ACLs/xattrs, special permission bits, birth/access-time editing, checksums and
archive/remote metadata mutation are deferred.

## Dependencies and Impact

Build on W10 entry identity and W07 mutation control. Reuse W12's bounded walker
if available, but do not require search or combine their workflows. Integrate W11
tab identities without tying properties to whichever tab becomes active later.
Unix metadata applies to existing Linux/macOS targets; unsupported fields remain
explicitly unavailable. W15 remote properties are a later provider extension.

## Verification

Recover Properties, ChangePermissions and ChangeDate actions/jobs and folder-size
callers from pinned binaries. Freeze original/native metadata normalization,
mutation/error fixtures and executable reserved cases before implementation.
Verify real permission/date changes as an unprivileged user on Linux and macOS;
record partial updates and unavailable host checks. No behavior is added by this
proposal and existing W10 verification limits remain open.
