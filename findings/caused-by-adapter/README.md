# Caused by adapter (not dashu bugs)

These inputs were opendp-num **adapter** defects — double rounding, native-float
prechecks, and an unsound directed-conversion helper in the previous
`src/backend/dashu.rs`. They were never dashu bugs, and they are fixed. The raw
inputs are retained here only as internal regression provenance; they are not
upstream-relevant and are intentionally not written up.
