# Findings removed by the Dashu 0.5 upgrade

The following Dashu 0.4 findings no longer reproduce after this repository upgraded its locked baseline to Dashu 0.5.0:

- `DASHU-001`: signed-zero loss across directed operations;
- `DASHU-003`: nearest rational-to-f32 conversion discrepancies;
- `DASHU-009`: nearest rational-to-f64 conversion discrepancy.

Their former reproducers are retained under `fuzz/resolved_inputs/dashu-0.5.0/` for historical comparison. They are not included in the active findings index and are not candidates for new upstream reports against Dashu 0.5.

The former `DASHU-017` directory represented a second manifestation of the still-active f32 `exp` underflow defect; it has been merged into `DASHU-004` rather than marked fixed.
