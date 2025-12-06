#!/bin/bash

# NOXTERM Anyone Protocol Integration Test Suite
# This script comprehensively tests the security and functionality of Anyone SDK integration

echo "NOXTERM Anyone Protocol Integration Test Suite"
echo "================================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_TOTAL=0

# Test function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_exit_code="${3:-0}"
    
    echo "🔍 TEST $((++TESTS_TOTAL)): $test_name"
    
    if eval "$test_command" > /tmp/test_output 2>&1; then
        if [ $? -eq $expected_exit_code ]; then
            echo -e "${GREEN}✅ PASSED${NC}"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}❌ FAILED${NC} (unexpected exit code)"
            cat /tmp/test_output
        fi
    else
        echo -e "${RED}❌ FAILED${NC} (command failed)"
        cat /tmp/test_output
    fi
    echo ""
}

# Backend API Tests
echo "🔧 BACKEND API TESTS"
echo "===================="

run_test "Privacy Status Endpoint" \
    "curl -s -f http://localhost:3001/api/privacy/status | jq -e '.enabled != null'"

run_test "SOCKS Proxy Accessibility" \
    "timeout 3 nc -z 127.0.0.1 9050"

run_test "Privacy Disable Functionality" \
    "curl -s -X POST http://localhost:3001/api/privacy/disable | jq -e '.status == \"disabled\"'"

run_test "Privacy Enable Functionality" \
    "curl -s -X POST http://localhost:3001/api/privacy/enable | jq -e '.status == \"enabled\"'"

# Security Tests
echo "🔒 SECURITY VALIDATION TESTS"
echo "============================"

run_test "IP Anonymization Verification" \
    'DIRECT_IP=$(curl -s http://httpbin.org/ip | jq -r ".origin"); 
     PROXY_IP=$(curl --socks5-hostname 127.0.0.1:9050 -s http://httpbin.org/ip | jq -r ".origin");
     [ "$DIRECT_IP" != "$PROXY_IP" ]'

run_test "SOCKS5 Proxy Functionality" \
    "timeout 10 curl --socks5-hostname 127.0.0.1:9050 -s -f http://httpbin.org/ip"

run_test "Error Handling - Invalid Endpoint" \
    "curl -s -w '%{http_code}' http://localhost:3001/api/invalid | grep -q '404'"

run_test "Error Handling - Malformed JSON" \
    "curl -s -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d '{invalid}' http://localhost:3001/api/sessions | grep -q '400\\|422'"

# Frontend Integration Tests
echo "🌐 FRONTEND INTEGRATION TESTS"
echo "============================="

run_test "Frontend Proxy to Backend" \
    "curl -s -f http://localhost:5174/api/privacy/status | jq -e '.enabled != null'"

run_test "Frontend Health Check" \
    "curl -s -w '%{http_code}' http://localhost:5174/ | grep -q '200'"

# Container Security Tests
echo "🐳 CONTAINER SECURITY TESTS"
echo "==========================="

run_test "Session Creation" \
    "curl -s -X POST http://localhost:3001/api/sessions -H 'Content-Type: application/json' -d '{\"user_id\":\"security_test\",\"container_image\":\"ubuntu:22.04\"}' | jq -e '.session_id != null'"

# Performance Tests
echo "⚡ PERFORMANCE TESTS"
echo "==================="

run_test "Privacy API Response Time" \
    "timeout 5 curl -s -w '%{time_total}' http://localhost:3001/api/privacy/status | awk '{if(\$1 < 1) exit 0; else exit 1}'"

run_test "SOCKS Proxy Response Time" \
    "timeout 10 curl --socks5-hostname 127.0.0.1:9050 -s -w '%{time_total}' http://httpbin.org/ip | awk '{if(\$1 < 5) exit 0; else exit 1}'"

# Summary
echo "📊 TEST SUMMARY"
echo "==============="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}/$TESTS_TOTAL"

if [ $TESTS_PASSED -eq $TESTS_TOTAL ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED - Integration is working securely!${NC}"
    exit 0
else
    echo -e "${RED}⚠️  Some tests failed - Review the output above${NC}"
    exit 1
fi
