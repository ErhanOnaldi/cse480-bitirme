#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESULTS_DIR="${ROOT_DIR}/results_10s"

mkdir -p "${RESULTS_DIR}"

rows_from_report_file() {
  local report_path="$1"
  awk '
    NR <= 2 { next }
    NF == 0 { next }
    {
      inst=$1; exact=$2; mean=$3; best=$4; std=$5; mt=$6; bt=$7; gap=$8;
      printf "\\texttt{\\detokenize{%s}} & %s & %s & %s & %s & %s & %s & \\texttt{\\detokenize{%s}} \\\\\n", inst, exact, mean, best, std, mt, bt, gap
    }
  ' "${report_path}"
}

rows_from_synthetic_runbatch() {
  local report_path="$1"
  awk '
    NR <= 2 { next }
    NF == 0 { next }
    $1 ~ /^synthetic-/ {
      inst=$1; mean=$2; best=$3; std=$4; mt=$5; bt=$6;
      printf "\\texttt{\\detokenize{%s}} & - & %s & %s & %s & %s & %s & - \\\\\n", inst, mean, best, std, mt, bt
    }
  ' "${report_path}"
}

write_tp2_table() {
  local out_path="${RESULTS_DIR}/tp2_table.tex"
  {
    echo "{%"
    echo "\\setlength{\\tabcolsep}{4pt}"
    echo "\\renewcommand{\\arraystretch}{0.95}"
    echo "\\begin{table}[H]"
    echo "\\centering"
    echo "\\caption{Results on TP2 example instance, 5 runs}"
    echo "\\begin{tabular}{lrrrrrrp{4.5cm}}"
    echo "\\toprule"
    echo "Instance & Exact & Mean Obj & Best Obj & Std. Dev. & Mean Time [s] & Best Time [s] & Gap\\% runs\\\\"
    echo "\\midrule"
    rows_from_report_file "${RESULTS_DIR}/tp2_report.txt"
    echo "\\bottomrule"
    echo "\\end{tabular}"
    echo "\\end{table}"
    echo "}%"
  } > "${out_path}"
}

write_synthetic_table() {
  local out_path="${RESULTS_DIR}/synthetic_table.tex"
  {
    echo "{%"
    echo "\\setlength{\\tabcolsep}{4pt}"
    echo "\\renewcommand{\\arraystretch}{0.95}"
    echo "\\begin{table}[H]"
    echo "\\centering"
    echo "\\caption{Results on synthetic instances, 5 runs each}"
    echo "\\begin{tabular}{lrrrrrrp{4.5cm}}"
    echo "\\toprule"
    echo "Instance & Exact & Mean Obj & Best Obj & Std. Dev. & Mean Time [s] & Best Time [s] & Gap\\% runs\\\\"
    echo "\\midrule"
    rows_from_synthetic_runbatch "${RESULTS_DIR}/synthetic_runbatch.txt"
    echo "\\bottomrule"
    echo "\\end{tabular}"
    echo "\\end{table}"
    echo "}%"
  } > "${out_path}"
}

write_binpack_longtable() {
  local report_path="$1"
  local label="$2"
  local out_path="$3"
  {
    echo "{%"
    echo "\\setlength{\\tabcolsep}{2pt}"
    echo "\\renewcommand{\\arraystretch}{0.90}"
    echo "\\begin{longtable}{@{}>{\\raggedright\\arraybackslash}p{2.7cm}rrrrrr>{\\raggedright\\arraybackslash}p{3.9cm}@{}}"
    echo "\\caption{Results on \\texttt{${label}}, 20 instances, 5 runs each}\\\\"
    echo "\\toprule"
    echo "Instance & Exact & Mean Obj & Best Obj & Std. Dev. & Mean Time [s] & Best Time [s] & Gap\\% runs\\\\"
    echo "\\midrule"
    echo "\\endfirsthead"
    echo "\\toprule"
    echo "Instance & Exact & Mean Obj & Best Obj & Std. Dev. & Mean Time [s] & Best Time [s] & Gap\\% runs\\\\"
    echo "\\midrule"
    echo "\\endhead"
    echo "\\midrule"
    echo "\\multicolumn{8}{r}{\\textit{Continued on next page}}\\\\"
    echo "\\endfoot"
    echo "\\bottomrule"
    echo "\\endlastfoot"
    rows_from_report_file "${report_path}"
    echo "\\end{longtable}"
    echo "}%"
  } > "${out_path}"
}

write_tp2_table
write_synthetic_table
write_binpack_longtable "${RESULTS_DIR}/binpack2_report.txt" "binpack2.txt" "${RESULTS_DIR}/binpack2_longtable.tex"
write_binpack_longtable "${RESULTS_DIR}/binpack4_report.txt" "binpack4.txt" "${RESULTS_DIR}/binpack4_longtable.tex"
write_binpack_longtable "${RESULTS_DIR}/binpack7_report.txt" "binpack7.txt" "${RESULTS_DIR}/binpack7_longtable.tex"
write_binpack_longtable "${RESULTS_DIR}/binpack8_report.txt" "binpack8.txt" "${RESULTS_DIR}/binpack8_longtable.tex"

echo "Wrote:"
ls -1 "${RESULTS_DIR}"/*_table.tex "${RESULTS_DIR}"/*_longtable.tex
