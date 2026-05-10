#!/usr/bin/env bash
set -euo pipefail

# Comprehensive test runner for Pylos PR validation
# This script runs all test suites to validate changes

echo "🧪 Starting Pylos Test Suite..."
echo "=================================="

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to report test result
report_result() {
  local test_name=$1
  local result=$2
  
  if [ "$result" -eq 0 ]; then
    echo -e "${GREEN}✅ $test_name passed${NC}"
    ((TESTS_PASSED++))
  else
    echo -e "${RED}❌ $test_name failed${NC}"
    ((TESTS_FAILED++))
  fi
}

# 1. Formatting Check
echo ""
echo "📝 1/5 - Running Formatting Check..."
echo "-----------------------------------"
if cargo fmt --all -- --check; then
  report_result "Formatting Check" 0
else
  report_result "Formatting Check" 1
fi

# 2. Workspace Build Validation
echo ""
echo "📦 2/5 - Validating Workspace Build..."
echo "-----------------------------------"
if cargo build --workspace; then
  report_result "Workspace Build" 0
else
  report_result "Workspace Build" 1
fi

# 3. Linting (Clippy)
echo ""
echo "🔍 3/5 - Running Linting (Clippy)..."
echo "-----------------------------------"
if cargo clippy --workspace -- -D warnings; then
  report_result "Clippy Lints" 0
else
  report_result "Clippy Lints" 1
fi

# 4. Unit Tests
echo ""
echo "🧪 4/5 - Running Unit Tests..."
echo "-----------------------------------"
if cargo test --workspace; then
  report_result "Unit Tests" 0
else
  report_result "Unit Tests" 1
fi

# 5. Security Audit
echo ""
echo "🛡️  5/5 - Running Security Audit..."
echo "-----------------------------------"
# We check if cargo-audit is installed, otherwise we skip but warn
if command -v cargo-audit &> /dev/null; then
  if cargo audit; then
    report_result "Security Audit" 0
  else
    report_result "Security Audit" 1
  fi
else
  echo -e "${YELLOW}⚠️  cargo-audit not found, skipping security check...${NC}"
  report_result "Security Audit (Skipped)" 0
fi

# Final Summary
echo ""
echo "=================================="
echo "🏁 Test Suite Complete!"
echo "=================================="
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ "$TESTS_FAILED" -gt 0 ]; then
  echo -e "${RED}❌ Some tests failed. Please review the output above.${NC}"
  exit 1
else
  echo -e "${GREEN}✅ All tests passed successfully!${NC}"
  exit 0
fi
