#!/bin/bash

# NOXTERM Frontend Integration Tests
# Tests frontend-to-backend connectivity and proxy functionality

echo "🌐 FRONTEND INTEGRATION TESTS"
echo "============================="

# Test frontend proxy to backend
echo "Testing Frontend Proxy to Backend..."
RESPONSE=$(curl -s http://localhost:5174/api/privacy/status)
if echo "$RESPONSE" | jq -e '.enabled != null' > /dev/null; then
    echo "✅ Frontend proxy working correctly"
else
    echo "❌ Frontend proxy test failed"
    exit 1
fi

# Test frontend accessibility
echo "Testing Frontend Accessibility..."
STATUS_CODE=$(curl -s -w '%{http_code}' http://localhost:5174/ -o /dev/null)
if [ "$STATUS_CODE" = "200" ]; then
    echo "✅ Frontend accessible on port 5174"
else
    echo "❌ Frontend accessibility test failed (status: $STATUS_CODE)"
    exit 1
fi

# Test frontend API proxy for session endpoints
echo "Testing Frontend Session API Proxy..."
RESPONSE=$(curl -s -X POST http://localhost:5174/api/sessions \
    -H "Content-Type: application/json" \
    -d '{"user_id":"frontend_test","container_image":"ubuntu:22.04"}')
if echo "$RESPONSE" | jq -e '.session_id != null' > /dev/null; then
    echo "✅ Frontend session API proxy working"
else
    echo "❌ Frontend session API proxy failed"
    exit 1
fi

# Test privacy toggle through frontend
echo "Testing Privacy Toggle Through Frontend..."
DISABLE_RESPONSE=$(curl -s -X POST http://localhost:5174/api/privacy/disable)
ENABLE_RESPONSE=$(curl -s -X POST http://localhost:5174/api/privacy/enable)

if echo "$DISABLE_RESPONSE" | jq -e '.status == "disabled"' > /dev/null && \
   echo "$ENABLE_RESPONSE" | jq -e '.status == "enabled"' > /dev/null; then
    echo "✅ Privacy toggle through frontend working"
else
    echo "❌ Privacy toggle through frontend failed"
    exit 1
fi

echo "🎉 All frontend integration tests passed!"
