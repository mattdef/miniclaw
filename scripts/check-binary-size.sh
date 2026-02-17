#!/bin/bash
# Script to validate miniclaw binary size
# Target: < 15MB (measured with strip) - NFR-P1
# Exit codes: 0 = success, 1 = binary too large

set -e

# Binary size target from NFR-P1
MAX_SIZE_MB=15
BINARY_NAME="miniclaw"

echo "=== Binary Size Validation ==="
echo "Target: < ${MAX_SIZE_MB}MB (stripped)"
echo ""

# Check if binary exists
if [ ! -f "target/release/${BINARY_NAME}" ]; then
    echo "ERROR: Release binary not found at target/release/${BINARY_NAME}"
    echo "Build with: cargo build --release"
    exit 1
fi

# Get binary size in bytes (cross-platform)
BINARY_PATH="target/release/${BINARY_NAME}"
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    BINARY_SIZE_BYTES=$(stat -f%z "$BINARY_PATH")
else
    # Linux
    BINARY_SIZE_BYTES=$(stat -c%s "$BINARY_PATH")
fi
BINARY_SIZE_MB=$(echo "scale=2; $BINARY_SIZE_BYTES / 1024 / 1024" | bc)

echo "Binary path: $BINARY_PATH"
echo "Binary size: ${BINARY_SIZE_MB}MB (${BINARY_SIZE_BYTES} bytes)"
echo ""

# Check if strip is available
if command -v strip &> /dev/null; then
    # Create a temporary stripped copy
    TEMP_BINARY=$(mktemp)
    cp "$BINARY_PATH" "$TEMP_BINARY"
    strip "$TEMP_BINARY"
    
    # Get stripped size (cross-platform)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        STRIPPED_SIZE_BYTES=$(stat -f%z "$TEMP_BINARY")
    else
        # Linux
        STRIPPED_SIZE_BYTES=$(stat -c%s "$TEMP_BINARY")
    fi
    STRIPPED_SIZE_MB=$(echo "scale=2; $STRIPPED_SIZE_BYTES / 1024 / 1024" | bc)
    
    echo "Stripped size: ${STRIPPED_SIZE_MB}MB (${STRIPPED_SIZE_BYTES} bytes)"
    echo ""
    
    # Use stripped size for validation
    ACTUAL_SIZE_MB=$STRIPPED_SIZE_MB
    ACTUAL_SIZE_BYTES=$STRIPPED_SIZE_BYTES
    
    # Cleanup
    rm -f "$TEMP_BINARY"
else
    echo "WARNING: 'strip' command not found, using unstripped size"
    echo ""
    ACTUAL_SIZE_MB=$BINARY_SIZE_MB
    ACTUAL_SIZE_BYTES=$BINARY_SIZE_BYTES
fi

# Validate against target
if (( $(echo "$ACTUAL_SIZE_MB < $MAX_SIZE_MB" | bc -l) )); then
    echo "✅ PASS: Binary size (${ACTUAL_SIZE_MB}MB) is within target (< ${MAX_SIZE_MB}MB)"
    exit 0
else
    echo "❌ FAIL: Binary size (${ACTUAL_SIZE_MB}MB) exceeds target (< ${MAX_SIZE_MB}MB)"
    echo ""
    echo "Suggestions:"
    echo "  - Check Cargo.toml for unnecessary dependencies"
    echo "  - Enable LTO in release profile"
    echo "  - Use 'strip = true' in Cargo.toml"
    echo "  - Consider using 'panic = \"abort\"'"
    exit 1
fi
