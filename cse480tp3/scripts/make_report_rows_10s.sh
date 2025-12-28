#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESULTS_DIR="${ROOT_DIR}/results_10s"

mkdir -p "${RESULTS_DIR}"

make_rows_from_report_file() {
  local report_path="$1"
  local out_path="$2"

  awk '
    NR <= 2 { next }
    NF == 0 { next }
    {
      inst=$1; exact=$2; mean=$3; best=$4; std=$5; mt=$6; bt=$7; gap=$8;
      printf "\\texttt{\\detokenize{%s}} & %s & %s & %s & %s & %s & %s & \\texttt{\\detokenize{%s}} \\\\tabularnewline\n", inst, exact, mean, best, std, mt, bt, gap
    }
  ' "${report_path}" > "${out_path}"
}

make_rows_from_synthetic_runbatch() {
  local report_path="$1"
  local out_path="$2"

  awk '
    NR <= 2 { next }
    NF == 0 { next }
    $1 ~ /^synthetic-/ {
      inst=$1; mean=$2; best=$3; std=$4; mt=$5; bt=$6;
      printf "\\texttt{\\detokenize{%s}} & - & %s & %s & %s & %s & %s & - \\\\tabularnewline\n", inst, mean, best, std, mt, bt
    }
  ' "${report_path}" > "${out_path}"
}

make_rows_from_report_file "${RESULTS_DIR}/tp2_report.txt" "${RESULTS_DIR}/tp2_report_rows.tex"
make_rows_from_synthetic_runbatch "${RESULTS_DIR}/synthetic_runbatch.txt" "${RESULTS_DIR}/synthetic_report_rows.tex"

make_rows_from_report_file "${RESULTS_DIR}/binpack2_report.txt" "${RESULTS_DIR}/binpack2_report_rows.tex"
make_rows_from_report_file "${RESULTS_DIR}/binpack4_report.txt" "${RESULTS_DIR}/binpack4_report_rows.tex"
make_rows_from_report_file "${RESULTS_DIR}/binpack7_report.txt" "${RESULTS_DIR}/binpack7_report_rows.tex"
make_rows_from_report_file "${RESULTS_DIR}/binpack8_report.txt" "${RESULTS_DIR}/binpack8_report_rows.tex"

echo "Wrote:"
ls -1 "${RESULTS_DIR}"/*_report_rows.tex
