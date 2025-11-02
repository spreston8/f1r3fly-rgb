#!/bin/bash
set -e

: '
===============================================================================
RGB Transfer Balance Test Runner
===============================================================================
Runs the rgb_transfer_balance_test in a loop with fresh regtest environment

This script:
1. Starts regtest environment (Bitcoin Core + Electrs + Esplora)
2. Runs the RGB transfer balance test
3. Stops and cleans regtest environment
4. Repeats for specified number of iterations
5. Logs all output and provides summary

Usage:
  ./run-rgb-transfer-test.sh [iterations]

Examples:
  ./run-rgb-transfer-test.sh        # Run 5 times (default)
  ./run-rgb-transfer-test.sh 10     # Run 10 times
===============================================================================
'

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WALLET_DIR="$PROJECT_ROOT/wallet"
LOG_DIR="$PROJECT_ROOT/test-logs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
TEST_LOG="$LOG_DIR/rgb_transfer_test_$TIMESTAMP.log"
SUMMARY_LOG="$LOG_DIR/rgb_transfer_test_summary_$TIMESTAMP.log"

# Test configuration
ITERATIONS=${1:-5}  # Default to 5 iterations if not specified
START_REGTEST="$SCRIPT_DIR/start-regtest.sh"
STOP_REGTEST="$SCRIPT_DIR/stop-regtest.sh"

# Delays (in seconds) - tuned for reliability
DELAY_AFTER_START=5    # Let services fully initialize
DELAY_AFTER_STOP=3     # Let services fully stop and release locks

# Tracking
declare -a RESULTS
PASSED=0
FAILED=0

# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

log_header() {
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${BLUE}$1${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

log_info() {
    echo -e "${CYAN}ℹ${NC}  $1"
}

log_success() {
    echo -e "${GREEN}✓${NC}  $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC}  $1"
}

log_error() {
    echo -e "${RED}✗${NC}  $1"
}

# ============================================================================
# INITIALIZATION
# ============================================================================

log_header "RGB Transfer Balance Test Runner"

echo -e "${CYAN}Configuration:${NC}"
echo -e "  • Iterations:    ${YELLOW}$ITERATIONS${NC}"
echo -e "  • Project Root:  ${YELLOW}$PROJECT_ROOT${NC}"
echo -e "  • Wallet Dir:    ${YELLOW}$WALLET_DIR${NC}"
echo -e "  • Test Log:      ${YELLOW}$TEST_LOG${NC}"
echo -e "  • Summary Log:   ${YELLOW}$SUMMARY_LOG${NC}"
echo ""

# Create log directory
mkdir -p "$LOG_DIR"

# Verify required scripts exist
if [ ! -f "$START_REGTEST" ]; then
    log_error "Start script not found: $START_REGTEST"
    exit 1
fi

if [ ! -f "$STOP_REGTEST" ]; then
    log_error "Stop script not found: $STOP_REGTEST"
    exit 1
fi

# Verify wallet directory exists
if [ ! -d "$WALLET_DIR" ]; then
    log_error "Wallet directory not found: $WALLET_DIR"
    exit 1
fi

# Initialize logs
echo "RGB Transfer Balance Test - Run started at $(date)" | tee "$TEST_LOG" "$SUMMARY_LOG"
echo "Configuration: $ITERATIONS iterations" | tee -a "$TEST_LOG" "$SUMMARY_LOG"
echo "========================================" | tee -a "$TEST_LOG" "$SUMMARY_LOG"
echo "" | tee -a "$TEST_LOG" "$SUMMARY_LOG"

# ============================================================================
# MAIN TEST LOOP
# ============================================================================

for i in $(seq 1 $ITERATIONS); do
    echo ""
    log_header "Iteration $i of $ITERATIONS"
    
    ITERATION_START=$(date +%s)
    ITERATION_LOG="$LOG_DIR/iteration_${i}_$TIMESTAMP.log"
    
    # ------------------------------------------------------------------------
    # Step 1: Start Regtest Environment
    # ------------------------------------------------------------------------
    
    log_info "Starting regtest environment..."
    if "$START_REGTEST" >> "$ITERATION_LOG" 2>&1; then
        log_success "Regtest started successfully"
    else
        log_error "Failed to start regtest environment"
        RESULTS[$i]="FAILED (startup)"
        FAILED=$((FAILED + 1))
        echo "[Iteration $i] FAILED - Regtest startup failed" >> "$SUMMARY_LOG"
        
        # Try to clean up
        "$STOP_REGTEST" >> "$ITERATION_LOG" 2>&1 || true
        sleep $DELAY_AFTER_STOP
        continue
    fi
    
    # Wait for services to fully initialize
    log_info "Waiting ${DELAY_AFTER_START}s for services to initialize..."
    sleep $DELAY_AFTER_START
    
    # ------------------------------------------------------------------------
    # Step 2: Run Test
    # ------------------------------------------------------------------------
    
    log_info "Running RGB transfer balance test..."
    
    cd "$WALLET_DIR"
    
    TEST_OUTPUT=$(mktemp)
    TEST_RESULT=0
    TEST_FAILED=false
    
    if cargo test --test rgb_transfer_balance_test -- --ignored --nocapture > "$TEST_OUTPUT" 2>&1; then
        log_success "Test PASSED"
        RESULTS[$i]="PASSED"
        PASSED=$((PASSED + 1))
        echo "[Iteration $i] PASSED" >> "$SUMMARY_LOG"
    else
        TEST_RESULT=$?
        log_error "Test FAILED (exit code: $TEST_RESULT)"
        RESULTS[$i]="FAILED (test)"
        FAILED=$((FAILED + 1))
        echo "[Iteration $i] FAILED - Test failed with exit code $TEST_RESULT" >> "$SUMMARY_LOG"
        TEST_FAILED=true
    fi
    
    # Append test output to logs
    cat "$TEST_OUTPUT" >> "$ITERATION_LOG"
    cat "$TEST_OUTPUT" >> "$TEST_LOG"
    rm "$TEST_OUTPUT"
    
    cd "$SCRIPT_DIR"
    
    # ------------------------------------------------------------------------
    # Step 3: Stop Regtest Environment
    # ------------------------------------------------------------------------
    
    log_info "Stopping regtest environment..."
    if "$STOP_REGTEST" >> "$ITERATION_LOG" 2>&1; then
        log_success "Regtest stopped successfully"
    else
        log_warning "Failed to stop regtest cleanly (will continue anyway)"
    fi
    
    # ------------------------------------------------------------------------
    # Iteration Summary
    # ------------------------------------------------------------------------
    
    ITERATION_END=$(date +%s)
    ITERATION_TIME=$((ITERATION_END - ITERATION_START))
    
    echo ""
    echo -e "${BOLD}Iteration $i Result: ${RESULTS[$i]} (${ITERATION_TIME}s)${NC}"
    echo "----------------------------------------"
    echo ""
    
    echo "[Iteration $i] Duration: ${ITERATION_TIME}s" >> "$SUMMARY_LOG"
    echo "" >> "$SUMMARY_LOG"
    
    # ------------------------------------------------------------------------
    # Check if test failed - if so, stop the loop
    # ------------------------------------------------------------------------
    
    if [ "$TEST_FAILED" = true ]; then
        echo ""
        log_error "Test failed on iteration $i - stopping test run"
        echo "[ABORTED] Test run stopped after iteration $i due to test failure" >> "$SUMMARY_LOG"
        echo "" >> "$SUMMARY_LOG"
        break
    fi
    
    # Wait for services to fully stop and release locks before next iteration
    if [ $i -lt $ITERATIONS ]; then
        log_info "Waiting ${DELAY_AFTER_STOP}s for cleanup..."
        sleep $DELAY_AFTER_STOP
    fi
done

# ============================================================================
# FINAL SUMMARY
# ============================================================================

echo ""
log_header "Test Run Complete"

TOTAL_RUN=$((PASSED + FAILED))
SUCCESS_RATE=$(awk "BEGIN {printf \"%.1f\", ($PASSED/$TOTAL_RUN)*100}")

echo -e "\n${BOLD}${CYAN}📊 Summary:${NC}"
echo -e "  • Planned Iterations: ${YELLOW}$ITERATIONS${NC}"
echo -e "  • Completed:          ${YELLOW}$TOTAL_RUN${NC}"
echo -e "  • Passed:             ${GREEN}$PASSED${NC}"
echo -e "  • Failed:             ${RED}$FAILED${NC}"
echo -e "  • Success Rate:       ${YELLOW}${SUCCESS_RATE}%${NC}"

echo -e "\n${BOLD}${CYAN}📋 Detailed Results:${NC}"
for i in $(seq 1 $TOTAL_RUN); do
    if [[ "${RESULTS[$i]}" == "PASSED" ]]; then
        echo -e "  ${GREEN}✓${NC} Iteration $i: ${GREEN}${RESULTS[$i]}${NC}"
    else
        echo -e "  ${RED}✗${NC} Iteration $i: ${RED}${RESULTS[$i]}${NC}"
    fi
done

echo -e "\n${BOLD}${CYAN}📝 Logs:${NC}"
echo -e "  • Full Log:     ${YELLOW}$TEST_LOG${NC}"
echo -e "  • Summary Log:  ${YELLOW}$SUMMARY_LOG${NC}"
echo -e "  • Individual:   ${YELLOW}$LOG_DIR/iteration_*_$TIMESTAMP.log${NC}"

# Write summary to log
echo "" >> "$SUMMARY_LOG"
echo "========================================" >> "$SUMMARY_LOG"
echo "FINAL SUMMARY" >> "$SUMMARY_LOG"
echo "========================================" >> "$SUMMARY_LOG"
echo "Planned Iterations: $ITERATIONS" >> "$SUMMARY_LOG"
echo "Completed: $TOTAL_RUN" >> "$SUMMARY_LOG"
echo "Passed: $PASSED" >> "$SUMMARY_LOG"
echo "Failed: $FAILED" >> "$SUMMARY_LOG"
echo "Success Rate: ${SUCCESS_RATE}%" >> "$SUMMARY_LOG"
echo "" >> "$SUMMARY_LOG"
echo "Run completed at $(date)" >> "$SUMMARY_LOG"

echo ""

# Exit with error code if any tests failed
if [ $FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi

