#!/bin/bash

# NOXTERM Security Validation Tests
# Validates that Anyone Protocol integration provides proper anonymization and security

echo "🔒 SECURITY VALIDATION TESTS"
echo "============================"

# Test SOCKS proxy accessibility
echo "Testing SOCKS Proxy Accessibility..."
if timeout 3 nc -z 127.0.0.1 9050; then
    echo "✅ SOCKS proxy accessible on port 9050"
else
    echo "❌ SOCKS proxy not accessible"
    exit 1
fi

# Test IP anonymization
echo "Testing IP Anonymization..."
DIRECT_IP=$(curl -s http://httpbin.org/ip | jq -r '.origin')
PROXY_IP=$(curl --socks5-hostname 127.0.0.1:9050 -s http://httpbin.org/ip | jq -r '.origin')

if [ "$DIRECT_IP" != "$PROXY_IP" ]; then
    echo "✅ IP anonymization working (Direct: $DIRECT_IP, Proxy: $PROXY_IP)"
else
    echo "❌ IP anonymization failed - same IP addresses"
    exit 1
fi

# Test geolocation anonymization
echo "Testing Geolocation Anonymization..."
DIRECT_COUNTRY=$(curl -s http://ip-api.com/json | jq -r '.country')
PROXY_COUNTRY=$(curl --socks5-hostname 127.0.0.1:9050 -s http://ip-api.com/json | jq -r '.country')

if [ "$DIRECT_COUNTRY" != "$PROXY_COUNTRY" ]; then
    echo "✅ Geolocation anonymization working (Direct: $DIRECT_COUNTRY, Proxy: $PROXY_COUNTRY)"
else
    echo "⚠️  Geolocation same country (may be expected depending on network topology)"
fi

# Test SOCKS proxy functionality
echo "Testing SOCKS Proxy Functionality..."
if timeout 10 curl --socks5-hostname 127.0.0.1:9050 -s -f http://httpbin.org/ip > /dev/null; then
    echo "✅ SOCKS proxy functioning correctly"
else
    echo "❌ SOCKS proxy functionality test failed"
    exit 1
fi

# Test error handling
echo "Testing Error Handling..."
STATUS_CODE=$(curl -s -w '%{http_code}' http://localhost:3001/api/invalid_endpoint -o /dev/null)
if [ "$STATUS_CODE" = "404" ]; then
    echo "✅ Proper 404 error handling"
else
    echo "❌ Error handling test failed (got status $STATUS_CODE)"
    exit 1
fi

echo "🎉 All security tests passed!"
