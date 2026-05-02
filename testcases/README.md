# Test Cases

This directory contains focused assembly regression tests for the interpreter.

Run everything from the repository root with:

```sh
./test.sh
```

To also collect per-test interpreter logs under `logs/`, run:

```sh
./test.sh --logs
```

## Call Tests

| File | Role | Expected stdout | Expected cost |
| --- | --- | --- | --- |
| `call_basic_return.s` | Basic argument passing and return-value assignment. | `8` | `72` |
| `call_cost_before_callee.s` | Verifies that call cost is charged in the caller before the callee starts running. | `0` | `8445` |
| `call_deep_chain_flattened.s` | Verifies a deeper nested call chain runs through the interpreter's explicit runtime call stack instead of recursive host-language calls. | `123` | `1023` |
| `call_recursive_stress_5000.s` | Stress test for real recursion depth; performs 5000 self-recursive calls and should still run safely with the flattened interpreter call stack. | `5000` | `545200` |
| `call_restore_args_nested.s` | Verifies nested calls do not permanently overwrite the caller's `arg*` registers. | `7` | `93` |
| `call_restore_registers.s` | Verifies `r*` and `sp` are restored after the callee returns. | `5` | `92` |

## Branch / Control-Cost Tests

| File | Role | Expected stdout | Expected cost |
| --- | --- | --- | --- |
| `branch_after_call_uses_caller_block.s` | Verifies a branch after returning from a call still computes forward-jump cost using the caller block, not the callee entry block. | *(empty)* | `77` |
| `branch_backward_unconditional.s` | Verifies a backward unconditional jump keeps base cost `30` after an earlier forward jump. | *(empty)* | `76` |
| `branch_forward_cond_false.s` | Verifies a forward false branch uses `30 * 1.5 = 45`. | *(empty)* | `56` |
| `branch_forward_cond_true.s` | Verifies a forward true branch uses `90 * 1.5 = 135`. | *(empty)* | `146` |
| `branch_forward_unconditional.s` | Verifies a forward unconditional branch uses `30 * 1.5 = 45`. | *(empty)* | `46` |
| `switch_base_cost.s` | Verifies `switch` stays at base cost `60` and is not forward-jump scaled. | *(empty)* | `61` |

## Async Load / Debt Tests

| File | Role | Expected stdout | Expected cost |
| --- | --- | --- | --- |
| `aload_overwrite_cancels_debt.s` | Overwriting an unresolved `aload` destination should cancel the pending debt. | `42` | `8384` |
| `aload_wait_on_operand_use.s` | Using an unresolved `aload` result as an operand should wait at use time. | `0` | `8444` |
| `aload_wait_paid_by_intervening_load.s` | An intervening instruction should partially resolve `aload` debt before the value is used. | `0` | `8464` |
| `aload_parallel_resolve_multiple.s` | Multiple outstanding `aload` debts should all be reduced by each intervening instruction's elapsed cost. | `0` | `69` |
| `aload_parallel_resolve_fma_cancel.s` | Fused mul/add should cancel only the deferred mul cost; the add cost still reduces all outstanding `aload` debts. | `0` | `69` |
| `aload_parallel_resolve_fma_flush.s` | A non-fused pending mul flush and the following instruction should both reduce all outstanding `aload` debts. | `0` | `69` |
| `aload_parallel_resolve_to_zero.s` | Fully resolved `aload` debts should be removed without disrupting parallel debt resolution. | `0` | `93` |

## Heat Memory Tests

| File | Role | Expected stdout | Expected cost |
| --- | --- | --- | --- |
| `heat_applied_on_aload_issue.s` | `aload` should heat memory when issued, even before the loaded value is consumed. | `0` | `8545` |
| `heat_compare_aload_then_load.s` | Contrast case combining `aload`, later loads, and debt waiting. | `0` | `8594` |
| `heat_compare_neighbor_chain.s` | Contrast case for how a wider access heats neighboring sectors over repeated reuse. | `0` | `16845` |
| `heat_compare_same_sector_chain.s` | Verifies repeated stack accesses now stay at base cost because stack heat is disabled. | `0` | `131` |
| `heat_cooldown_between_accesses.s` | Verifies inserted arithmetic no longer matters for stack-memory cost because stack heat is disabled. | `0` | `131` |
| `heat_heap_allocation_boundary.s` | Verifies heat does not spread from one heap allocation into an adjacent separately allocated block. | `0` | `16815` |
| `heat_heap_baseline.s` | Minimal heap heat sanity check without output. | *(empty)* | `8613` |
| `heat_neighbor_spread.s` | Verifies a wider access heats neighboring 8-byte sectors. | `0` | `16715` |
| `heat_reported_offset32.s` | Regression for an 8-byte heap load heating the sector at `r1+32`. | *(empty)* | `41261` |
| `heat_reverse_neighbor_no_penalty.s` | Verifies heat spread and direct-sector penalty are not treated symmetrically. | *(empty)* | `16645` |
| `heat_repeat_heap_loads.s` | Repeated heap loads from the same byte should become more expensive. | `0` | `8643` |
| `heat_repeat_stack_loads.s` | Verifies repeated stack loads stay flat at base cost because only heap memory is heated. | `0` | `131` |

## Other Semantic Tests

| File | Role | Expected stdout | Expected cost |
| --- | --- | --- | --- |
| `fma_cancels_division_cost.s` | Verifies `div` followed immediately by add/sub also uses the FMA optimization path and cancels the division base cost. | `6` | `41` |
| `fma_cancels_multiply_cost.s` | Verifies FMA now cancels the multiplication cost rather than the later add cost, and logging should show a separate `Mul-FMA` entry on the fused path. | `8` | `41` |
| `fma_cancels_remainder_cost.s` | Verifies `rem` followed immediately by add/sub also uses the FMA optimization path and cancels the remainder base cost. | `0` | `41` |
| `fma_preserves_mul_four_phobia.s` | Verifies FMA cancels only the multiplication base cost; a 4-phobia surcharge paid by the `mul` instruction must remain. | `14` | `51` |
| `four_phobia_basic.s` | Basic 4-phobia cost check on ordinary `<val>` operands. | `5` | `51` |
| `four_phobia_once_with_size.s` | Verifies `<sz>=4` also triggers 4-phobia, but only one surcharge is paid even when another `4` appears in the same instruction. | `0` | `71` |
| `four_phobia_once_with_two_values.s` | Verifies two separate `<val>` operands equal to `4` still trigger only one 4-phobia charge for the instruction. | `8` | `51` |
| `signed_ashr_sign_extend_bw8.s` | Verifies `ashr` sign-extends narrow signed inputs before shifting. | `255` | `41` |
| `stdin_sum_to_n.s` | End-to-end stdin-driven loop test using `stdin_sum_to_n.in`; its total cost includes forward-jump penalties, and `eadd 4 4 64` now pays 4-phobia only once. | `55` | `9933` |

## Expected Failure Test

| File | Role | Expected result |
| --- | --- | --- |
| `invalid_assign_arg_register.s` | Verifies assigning directly to `arg*` registers is rejected. | Non-zero exit with `ArgRegAssign`-style error |
