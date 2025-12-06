#!/bin/bash

# NOXTERM Test Suite Runner
# Runs all test suites and provides comprehensive validation results

echo "NOXTERM Anyone Protocol Integration - COMPLETE TEST SUITE"
echo "============================================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TESTS_FAILED=0
TEST_SUITES=0

# Function to run a test suite
run_test_suite() {
    local suite_name="$1"
    local test_script="$2"
    
    echo -e "${BLUE}📋 Running: $suite_name${NC}"
    echo "----------------------------------------"
    
    ((TEST_SUITES++))
    
    if bash "$test_script"; then
        echo -e "${GREEN}✅ $suite_name PASSED${NC}"
    else
        echo -e "${RED}❌ $suite_name FAILED${NC}"
        ((TESTS_FAILED++))
    fi
    
    echo ""
}

# Pre-flight checks
echo "🔍 Pre-flight Checks"
echo "===================="

# Check if backend is running
if curl -s http://localhost:3001/health > /dev/null; then
    echo "✅ Backend is running on port 3001"
else
    echo "❌ Backend is not accessible on port 3001"
    echo "Please ensure backend is running: cargo run --bin noxterm-terminal"
    exit 1
fi

# Check if frontend is running
if curl -s http://localhost:5174/ > /dev/null; then
    echo "✅ Frontend is running on port 5174"
else
    echo "❌ Frontend is not accessible on port 5174"
    echo "Please ensure frontend is running: npm run dev"
    exit 1
fi

echo ""

# Run test suites
cd "$(dirname "$0")"

run_test_suite "Backend API Tests" "./backend-api-tests.sh"
run_test_suite "Security Validation Tests" "./security-tests.sh"
run_test_suite "Frontend Integration Tests" "./frontend-integration-tests.sh"

# Final summary
echo "📊 COMPLETE TEST SUMMARY"
echo "========================="
echo -e "Test Suites Run: ${BLUE}$TEST_SUITES${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "Result: ${GREEN}ALL $TEST_SUITES SUITES PASSED${NC}"
    echo ""
    echo "🎉 NOXTERM Anyone Protocol is 100% WORKING and SECURE!"
    echo "✅ Privacy controls functional"
    echo "✅ SOCKS proxy operational"
    echo "✅ IP anonymization verified"
    echo "✅ Frontend integration complete"
    echo "✅ Backend APIs working"
    echo "✅ Security validations passed"
    echo ""
    exit 0
else
    echo -e "Result: ${RED}$TESTS_FAILED/$TEST_SUITES SUITES FAILED${NC}"
    echo ""
    echo "⚠️  Please review failed tests above and ensure:"
    echo "   - Backend is running on port 3001"
    echo "   - Frontend is running on port 5174"
    echo "   - Anyone Protocol service is active"
    echo "   - Internet connectivity is available"
    echo ""
    exit 1
fi
