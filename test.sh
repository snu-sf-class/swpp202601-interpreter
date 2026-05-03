#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_PATH="$ROOT_DIR/target/release/main"
WITH_LOGS=0
BUILD_ARGS=(--release)
LOG_ROOT="$ROOT_DIR/logs"

if [[ "${1:-}" == "--logs" ]]; then
  WITH_LOGS=1
  BUILD_ARGS+=(--features log)
fi

prepare_log_dir() {
  local asm="$1"
  local test_name="${asm%.s}"
  local test_log_dir="$LOG_ROOT/$test_name"
  mkdir -p "$test_log_dir"
  find "$test_log_dir" -mindepth 1 -maxdepth 1 -type f ! -name '.gitkeep' -delete
  printf '%s\n' "$test_log_dir"
}

collect_program_logs() {
  local test_log_dir="$1"
  local base_log="$ROOT_DIR/swpp-interpreter-basic.log"
  local mem_log="$ROOT_DIR/swpp-interpreter-mem.log"
  local op_log="$ROOT_DIR/swpp-interpreter-op.log"

  if [[ -f "$base_log" ]]; then
    mv "$base_log" "$test_log_dir/base.log"
  fi
  if [[ -f "$mem_log" ]]; then
    mv "$mem_log" "$test_log_dir/mem.log"
  fi
  if [[ -f "$op_log" ]]; then
    mv "$op_log" "$test_log_dir/op.log"
  fi
}

run_positive() {
  local asm="$1"
  local expected_stdout="$2"
  local expected_cost="$3"
  local input_file="${4:-}"
  local test_log_dir=""
  local cost_file
  cost_file="$(mktemp)"

  if [[ $WITH_LOGS -eq 1 ]]; then
    test_log_dir="$(prepare_log_dir "$asm")"
  fi

  local stdout
  if [[ -n "$input_file" ]]; then
    stdout="$("$BIN_PATH" "$ROOT_DIR/testcases/$asm" "$cost_file" < "$ROOT_DIR/testcases/$input_file")"
  else
    stdout="$("$BIN_PATH" "$ROOT_DIR/testcases/$asm" "$cost_file")"
  fi

  local cost
  cost="$(cat "$cost_file")"

  if [[ $WITH_LOGS -eq 1 ]]; then
    printf '%s' "$stdout" > "$test_log_dir/stdout.txt"
    printf '%s\n' "$cost" > "$test_log_dir/cost.txt"
    collect_program_logs "$test_log_dir"
  fi

  rm -f "$cost_file"

  if [[ "$stdout" != "$expected_stdout" ]]; then
    echo "FAIL $asm"
    echo "  expected stdout: [$expected_stdout]"
    echo "  actual stdout:   [$stdout]"
    exit 1
  fi

  if [[ "$cost" != "Final Cost : $expected_cost" ]]; then
    echo "FAIL $asm"
    echo "  expected cost: [Final Cost : $expected_cost]"
    echo "  actual cost:   [$cost]"
    exit 1
  fi

  echo "PASS $asm"
}

run_negative() {
  local asm="$1"
  local error_fragment="$2"
  local test_log_dir=""
  local cost_file
  cost_file="$(mktemp)"

  if [[ $WITH_LOGS -eq 1 ]]; then
    test_log_dir="$(prepare_log_dir "$asm")"
  fi

  set +e
  local output
  output="$("$BIN_PATH" "$ROOT_DIR/testcases/$asm" "$cost_file" 2>&1)"
  local rc=$?
  set -e

  if [[ $WITH_LOGS -eq 1 ]]; then
    printf '%s' "$output" > "$test_log_dir/output.txt"
    if [[ -f "$cost_file" ]]; then
      cat "$cost_file" > "$test_log_dir/cost.txt"
    fi
    collect_program_logs "$test_log_dir"
  fi

  rm -f "$cost_file"

  if [[ $rc -eq 0 ]]; then
    echo "FAIL $asm"
    echo "  expected failure, but the program succeeded"
    exit 1
  fi

  if [[ "$output" != *"$error_fragment"* ]]; then
    echo "FAIL $asm"
    echo "  expected error to contain: [$error_fragment]"
    echo "  actual output: [$output]"
    exit 1
  fi

  echo "PASS $asm"
}

echo "== cargo test =="
cargo test

echo
echo "== cargo build --release =="
cargo build "${BUILD_ARGS[@]}"

echo
echo "== assembly suite =="
run_positive "branch_backward_unconditional.s" "" "76"
run_positive "branch_after_call_uses_caller_block.s" "" "77"
run_positive "branch_forward_cond_false.s" "" "56"
run_positive "branch_forward_cond_true.s" "" "146"
run_positive "branch_forward_unconditional.s" "" "46"
run_positive "call_basic_return.s" "8" "72"
run_positive "call_cost_before_callee.s" "0" "8445"
run_positive "call_deep_chain_flattened.s" "123" "1023"
run_positive "call_recursive_stress_5000.s" "5000" "545200"
run_positive "call_restore_args_nested.s" "7" "93"
run_positive "call_restore_registers.s" "5" "92"
run_positive "fma_cancels_division_cost.s" "6" "41"
run_positive "fma_cancels_multiply_cost.s" "8" "41"
run_positive "fma_cancels_remainder_cost.s" "0" "41"
run_positive "fma_preserves_mul_four_phobia.s" "14" "51"
run_positive "four_phobia_basic.s" "5" "51"
run_positive "four_phobia_once_with_size.s" "0" "71"
run_positive "four_phobia_once_with_two_values.s" "8" "51"
run_positive "aload_overwrite_cancels_debt.s" "42" "8384"
run_positive "aload_wait_on_operand_use.s" "0" "8444"
run_positive "aload_wait_paid_by_intervening_load.s" "0" "8464"
run_positive "aload_parallel_resolve_multiple.s" "0" "69"
run_positive "aload_parallel_resolve_fma_cancel.s" "0" "69"
run_positive "aload_parallel_resolve_fma_flush.s" "0" "69"
run_positive "aload_parallel_resolve_to_zero.s" "0" "93"
run_positive "aload_two_operand_use.s" "0" "79"
run_positive "heat_applied_on_aload_issue.s" "0" "8545"
run_positive "heat_compare_aload_then_load.s" "0" "8594"
run_positive "heat_compare_neighbor_chain.s" "0" "16845"
run_positive "heat_compare_same_sector_chain.s" "0" "131"
run_positive "heat_cooldown_between_accesses.s" "0" "131"
run_positive "heat_heap_allocation_boundary.s" "0" "16815"
run_positive "heat_heap_baseline.s" "" "8613"
run_positive "heat_neighbor_spread.s" "0" "16715"
run_positive "heat_reported_offset32.s" "" "41261"
run_positive "heat_reverse_neighbor_no_penalty.s" "" "16645"
run_positive "heat_repeat_heap_loads.s" "0" "8643"
run_positive "heat_repeat_stack_loads.s" "0" "131"
run_positive "signed_ashr_sign_extend_bw8.s" "255" "41"
run_positive "stdin_sum_to_n.s" "55" "9933" "stdin_sum_to_n.in"
run_positive "switch_base_cost.s" "" "61"
run_negative "invalid_assign_arg_register.s" "Your assembly fails with following Error
You cannot assign the value directly to the argument register arg1"

echo
echo "All tests passed."
