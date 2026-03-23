# v13 Pilot Analysis: Condition A (Sonnet + MCP)

## Runs

| Run ID | Binary | Tool calls | Callees returned |
|--------|--------|-----------|-----------------|
| A-pilot-1 | v0.1.6 (broken) | 59 | 0 |
| A-pilot-2 | v0.1.7 (fixed) | 31 | 1470 for AD_CalcOutput |

Both runs on commit `2895884d2be01862173c88d70f86b358d2f1a50a` of `OpenFAST/openfast`.

---

## A-pilot-1 (broken binary, v0.1.6)

### Findings

Task is tractable. The agent correctly identified both entry-point subroutines with exact file
paths and line numbers (AD_CalcOutput: line 2148, AD_UpdateStates: line 1830, both in
`modules/aerodyn/src/AeroDyn.f90`). The integration map was complete across 9 files covering the
full call stack from glue code to blade-element application.

### Critical tool limitation discovered

`analyze_symbol` returned `CALLEES: none` for all Fortran subroutines. The bug: `extract_calls`
was hard-coded to the `"function_item"` node kind (Rust-specific), so the callee half of the call
graph never fired for any non-Rust language.

**Impact on benchmark design (at time of A-pilot-1):**

- MCP conditions could orient efficiently (directory + analyze_file for entry-point discovery +
  locate-by-name) but could not auto-trace callees via analyze_symbol.
- This temporarily flipped the Dimension 2 dynamic: native tools could trace callees by reading
  file content; MCP tools could not via analyze_symbol.

### Tool call efficiency

59 calls breakdown:

- Directory orientation: ~5 calls
- analyze_file on key files: ~15 calls
- analyze_symbol (caller lookup only; callee side empty): ~20 calls
- analyze_module for import surveys: ~10 calls
- Redundant/exploratory calls: ~9 calls

---

## A-pilot-2 (fixed binary, v0.1.7)

### Fixes applied before this run

- **PR #416:** `enclosing_function_name` now covers all 6 language node kinds (was Rust-only).
  `extract_calls` now returns callees for Fortran, Go, Java, Python, and TypeScript.
- **PR #419:** `analyze_symbol` pagination emitted a cursor only for the callers section; when
  callers were exhausted, the callees section was silently dropped. Fixed.

### Callee graph result

`analyze_symbol(path="modules/aerodyn/src", symbol="AD_CalcOutput", follow_depth=2)` returned
1470 callees. Confirmed non-empty output validates both fixes end-to-end.

Depth-1 callees (direct): RotCalcOutput, FVW_CalcOutput, AD_CalcWind, RotCavtCrit, RotWriteOutputs.

Depth-2 callees (via RotCalcOutput): BEMT_CalcOutput, SetErrStat (NWTC), UA_CalcOutput,
AA_CalcOutput, ADTwr_CalcOutput, TFin_CalcOutput.

Pagination was triggered: AD_CalcOutput (1470 callees), RotCalcOutput (468), AD_UpdateStates (405)
all required cursor pagination. The PR #419 fix was exercised.

### Tool call efficiency

31 calls -- 47% fewer than A-pilot-1. The working callee graph eliminated the redundant
analyze_symbol/analyze_file loops the agent used to manually reconstruct the call chain.

Breakdown:

- Directory orientation: ~4 calls
- analyze_file on key files: ~10 calls
- analyze_symbol (callee graph now functional): ~8 calls (with pagination)
- analyze_module for import surveys: ~6 calls
- Exploratory/redundant calls: ~3 calls

### NWTC callee depth note

Most depth-1 callees from AD_CalcOutput are AeroDyn-internal wrappers (RotCalcOutput, AD_CalcWind,
etc.), not NWTC library routines directly. Confirmed NWTC library routines at depth 2: SetErrStat
(NWTC_Base.f90). IfW_FlowField_GetVelAcc appears at depth-2 via AD_CalcWind but belongs to
InflowWind, not NWTC library proper.

---

## Ground-truth anchor values (verified across both pilots)

| Item | Value |
|------|-------|
| AD_CalcOutput location | `modules/aerodyn/src/AeroDyn.f90:2148` |
| AD_UpdateStates location | `modules/aerodyn/src/AeroDyn.f90:1830` |
| Companion types file | `modules/aerodyn/src/AeroDyn_Types.f90` |
| Primary NWTC callee (depth 2) | SetErrStat, `modules/nwtc-library/src/NWTC_Base.f90` |
| AeroDyn internal depth-1 callees | RotCalcOutput, FVW_CalcOutput, AD_CalcWind |
| AeroDyn internal depth-2 callees | BEMT_CalcOutput, UA_CalcOutput |
| Glue code entry | `modules/openfast-library/src/FAST_Subs.f90:2676-3440` |
| Blade-element application | `modules/aerodyn/src/AeroDyn.f90:4068-4147` (SetOutputsFromBEMT) |
| Registry source | `modules/aerodyn/src/AeroDyn_Registry.txt` |

---

## Rubric calibration (final, applied to scored runs)

### Dimension 1: Entry-point location

- Score 3: Names both AD_CalcOutput + AD_UpdateStates with correct file, line within +/-5
- Score 2: Names both with correct file, line within +/-20
- Score 1: Correct file, at least one subroutine name, no line number

### Dimension 2: Call chain tracing

Dimension renamed from "NWTC Library Call Chain Tracing" to "Call Chain Tracing" to credit
correct intermediate-routine identification regardless of library origin.

- Score 3: Names RotCalcOutput + BEMT_CalcOutput + at least 1 NWTC routine (SetErrStat or similar)
  with file paths
- Score 2: Names BEMT_CalcOutput + 1 routine with file path; minor gaps
- Score 1: Mentions NWTC library is used; names fewer than 2 specific routines without file paths

### Dimension 3: Integration map quality

- Score 3: Integration map covers FAST_Subs.f90 + AeroDyn_Types.f90/Registry + AeroDyn.f90
  (SetParameters + SetOutputsFromBEMT/RotCalcOutput); notes registry regeneration pattern
- Score 2: Covers AeroDyn.f90 + FAST_Subs.f90 + mentions types file; 2+ line ranges; does not
  note registry pattern
- Score 1: Only AeroDyn.f90; no glue-code touchpoint; no types discussion

---

## Expected scored-run efficiency

| Condition | Expected calls |
|-----------|---------------|
| A (Sonnet + MCP) | 20-35 |
| B (Sonnet + native) | 30-50 |
| C (Haiku + MCP) | 25-45 |
| D (Haiku + native) | 40-70 |

MCP advantage is now confirmed on Dimensions 1, 2, and 3. The broken-callee limitation from
A-pilot-1 no longer applies. Scored runs should reflect the full MCP capability.

---

## Methodology updates required

1. Remove the note "analyze_symbol callee graphs are not available for Fortran" -- fixed in v0.1.7.
2. Rename Dimension 2 to "Call Chain Tracing" in the rubric.
3. Update Dimension 2 anchors per the calibration table above.
4. Add ground-truth anchor table to the calibration section of methodology.md.
5. Update expected MCP call efficiency from 15-25 to 20-35 (pagination overhead at depth=2 on
   large Fortran files is real; 1470 callees requires multiple cursor pages).
