#!/bin/sh

archive_inspection_receipt_marker() {
  if [ -n "${CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR:-}" ] \
    && [ -n "${CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_ID:-}" ] \
    && [ -d "$CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR" ]; then
    printf '%s\n' "$1" \
      >>"$CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR/$CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_ID.marker" \
      2>/dev/null || :
  fi
}

archive_inspection_receipt_marker "fixture=${1##*/}"

python3() {
  if [ "$1" = "$script_dir/check-release-archive-content" ]; then
    archive_inspection_receipt_marker "content-comparator-ran=1"
  fi
  command python3 "$@"
}
