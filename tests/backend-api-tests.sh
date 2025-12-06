#!/bin/bash

# NOXTERM Backend API Tests
# Tests all privacy control endpoints and backend functionality

echo "🔧 BACKEND API TESTS"
echo "===================="

# Test privacy status endpoint
echo "Testing Privacy Status Endpoint..."
RESPONSE=$(curl -s http://localhost:3001/api/privacy/status)
if echo "$RESPONSE" | jq -e '.enabled != null' > /dev/null; then
    echo "✅ Privacy status endpoint working"
else
    echo "❌ Privacy status endpoint failed"
    exit 1
fi

# Test privacy disable
echo "Testing Privacy Disable..."
RESPONSE=$(curl -s -X POST http://localhost:3001/api/privacy/disable)
if echo "$RESPONSE" | jq -e '.status == "disabled"' > /dev/null; then
    echo "✅ Privacy disable working"
else
    echo "❌ Privacy disable failed"
    exit 1
fi

# Test privacy enable  
echo "Testing Privacy Enable..."
RESPONSE=$(curl -s -X POST http://localhost:3001/api/privacy/enable)
if echo "$RESPONSE" | jq -e '.status == "enabled"' > /dev/null; then
    echo "✅ Privacy enable working"
else
    echo "❌ Privacy enable failed"
    exit 1
fi

# Test session creation
echo "Testing Session Creation..."
RESPONSE=$(curl -s -X POST http://localhost:3001/api/sessions \
    -H "Content-Type: application/json" \
    -d '{"user_id":"test_user","container_image":"ubuntu:22.04"}')
if echo "$RESPONSE" | jq -e '.session_id != null' > /dev/null; then
    echo "✅ Session creation working"
else
    echo "❌ Session creation failed"
    exit 1
fi

echo "🎉 All backend API tests passed!"
