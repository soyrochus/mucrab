# W14 — Built-in text and image viewer

## Why

W09 opens files in external applications, including retained archive copies.
Quick inspection still requires launching another app even for simple text or
images. A read-only viewer makes the two-pane workflow more useful.

## What Changes

- Add an explicit internal View action for the focused regular file, separate
  from W09 default-application Open.
- Preview bounded text with scrolling, find, selection and text copy, and PNG/JPEG
  images with fit/actual-size viewing, zoom and pan.
- Prepare local/link-target or supported ZIP-member content in the background
  with stale-request isolation, cancellation and bounded decoding/allocation.
- Provide clear unsupported/error/limit feedback and an explicit Open Externally
  action through W09.

Text and image presentation share the same read-only viewer lifecycle and belong
in one change. Editing/saving, hex/PDF/media viewers, extra image formats,
external application choice, recursive search, remote downloads and archive
writeback are not included. W15 may later supply downloaded SFTP content.

## Dependencies and Impact

Reuse W06 guarded extraction and W09 activation/preparation foundations. Keep the
ordinary Enter/double-click behavior unchanged. W11 stable tab identity prevents
preparation from targeting a different tab; no hard W12/W13 dependency exists.
Use existing pinned codecs where possible; verify any added features/dependencies
and notices before choosing a decoder.

## Verification

Recover original View/InternalView and text/image viewer boundaries. Freeze
format/encoding, limits, fixture generator and cancellation/result protocol before
implementation. Verify exact decoded content, image dimensions/pixels and both
native interfaces on Linux/macOS, distinguishing visual checks from runtime parity.
This proposal implements no code and does not close earlier pending host gates.
