#!/usr/bin/env bash

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NO_COLOR='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0

print_test() {
    echo -e "${YELLOW}[TEST]${NO_COLOR} $1"
}

print_pass() {
    echo -e "${GREEN}[PASS]${NO_COLOR} $1"
    ((TESTS_PASSED++))
}

print_fail() {
    echo -e "${RED}[FAIL]${NO_COLOR} $1"
    ((TESTS_FAILED++))
}

print_section() {
    echo ""
    echo "=========================================="
    echo "$1"
    echo "=========================================="
}

cleanup() {
    echo ""
    echo "Cleaning up..."
    rm -rf test/tmp
}

trap cleanup EXIT

print_section "Setting up test environment"
rm -rf test/tmp
mkdir -p test/tmp/images
mkdir -p test/tmp/output_single
mkdir -p test/tmp/output_list
mkdir -p test/tmp/output_dir
mkdir -p test/tmp/messages

echo "Creating test images..."
convert -size 200x200 xc:red test/tmp/images/01.png 2>/dev/null || magick -size 200x200 xc:red test/tmp/images/01.png
convert -size 200x200 xc:blue test/tmp/images/02.png 2>/dev/null || magick -size 200x200 xc:blue test/tmp/images/02.png
convert -size 200x200 xc:green test/tmp/images/03.png 2>/dev/null || magick -size 200x200 xc:green test/tmp/images/03.png
convert -size 150x150 xc:yellow test/tmp/images/04.jpg 2>/dev/null || magick -size 150x150 xc:yellow test/tmp/images/04.jpg

echo "Creating test messages..."
echo "Hello, World!" >test/tmp/messages/short.txt
# Create a message large enough to span multiple images (each 200x200 image can hold ~10KB)
# We need >10KB to use multiple images
dd if=/dev/urandom of=test/tmp/messages/long.txt bs=1024 count=15 2>/dev/null
echo "" >test/tmp/messages/empty.txt

print_section "Building project"
if cargo build --quiet 2>&1; then
    print_pass "Project built successfully"
else
    print_fail "Project build failed"
    exit 1
fi

print_section "Test 1: Unit tests"
print_test "Running cargo test"
if cargo test --quiet 2>&1 | grep -q "test result: ok"; then
    print_pass "All unit tests passed"
else
    print_fail "Unit tests failed"
fi

print_section "Test 2: Single image encode/decode"
print_test "Encoding message into single image"
if cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/encoded.png 2>&1 >/dev/null; then

    if [ -f test/tmp/output_single/encoded.png ]; then
        print_pass "Single image encoded successfully"
    else
        print_fail "Single image encoding failed - output file not created"
    fi
else
    print_fail "Single image encoding command failed"
fi

print_test "Decoding message from single image"
if cargo run --quiet -- decode \
    --image test/tmp/output_single/encoded.png \
    --output test/tmp/output_single/decoded.txt 2>&1 >/dev/null; then

    if [ -f test/tmp/output_single/decoded.txt ]; then
        if diff -q test/tmp/messages/short.txt test/tmp/output_single/decoded.txt >/dev/null 2>&1; then
            print_pass "Single image decoded correctly"
        else
            print_fail "Decoded message does not match original"
        fi
    else
        print_fail "Single image decoding failed - output file not created"
    fi
else
    print_fail "Single image decoding command failed"
fi

print_section "Test 3: Multiple images with --image-list"
print_test "Encoding message into multiple images using --image-list"
if cargo run --quiet -- encode \
    --image-list test/tmp/images/01.png test/tmp/images/02.png test/tmp/images/03.png \
    --message test/tmp/messages/long.txt \
    --output-dir test/tmp/output_list 2>&1 >/dev/null; then

    if [ -f test/tmp/output_list/01.png ]; then
        print_pass "Multiple images encoded successfully with --image-list"
    else
        print_fail "Multiple images encoding with --image-list failed - output files not created"
    fi
else
    print_fail "Multiple images encoding with --image-list command failed"
fi

print_test "Decoding message from multiple images using --image-dir"
if cargo run --quiet -- decode \
    --image-dir test/tmp/output_list \
    --output test/tmp/output_list/decoded.txt 2>&1 >/dev/null; then

    if [ -f test/tmp/output_list/decoded.txt ]; then
        if diff -q test/tmp/messages/long.txt test/tmp/output_list/decoded.txt >/dev/null 2>&1; then
            print_pass "Multiple images decoded correctly with --image-dir"
        else
            print_fail "Decoded message does not match original (--image-dir from --image-list)"
        fi
    else
        print_fail "Multiple images decoding with --image-dir failed - output file not created"
    fi
else
    print_fail "Multiple images decoding with --image-dir command failed"
fi

print_section "Test 4: Multiple images with --image-dir"
print_test "Encoding message into multiple images using --image-dir"
if cargo run --quiet -- encode \
    --image-dir test/tmp/images \
    --message test/tmp/messages/long.txt \
    --output-dir test/tmp/output_dir 2>&1 >/dev/null; then

    if [ -f test/tmp/output_dir/01.png ]; then
        print_pass "Multiple images encoded successfully with --image-dir"
    else
        print_fail "Multiple images encoding with --image-dir failed - output files not created"
    fi
else
    print_fail "Multiple images encoding with --image-dir command failed"
fi

print_test "Decoding message from multiple images using --image-dir"
if cargo run --quiet -- decode \
    --image-dir test/tmp/output_dir \
    --output test/tmp/output_dir/decoded.txt 2>&1 >/dev/null; then

    if [ -f test/tmp/output_dir/decoded.txt ]; then
        if diff -q test/tmp/messages/long.txt test/tmp/output_dir/decoded.txt >/dev/null 2>&1; then
            print_pass "Multiple images decoded correctly with --image-dir"
        else
            print_fail "Decoded message does not match original (--image-dir)"
        fi
    else
        print_fail "Multiple images decoding with --image-dir failed - output file not created"
    fi
else
    print_fail "Multiple images decoding with --image-dir command failed"
fi

print_section "Test 5: Empty message"
print_test "Encoding empty message"
if cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/empty.txt \
    --output test/tmp/output_single/empty.png 2>&1 >/dev/null; then

    print_test "Decoding empty message"
    if cargo run --quiet -- decode \
        --image test/tmp/output_single/empty.png \
        --output test/tmp/output_single/empty_decoded.txt 2>&1 >/dev/null; then

        if diff -q test/tmp/messages/empty.txt test/tmp/output_single/empty_decoded.txt >/dev/null 2>&1; then
            print_pass "Empty message handled correctly"
        else
            print_fail "Empty message handling failed - content mismatch"
        fi
    else
        print_fail "Empty message decoding command failed"
    fi
else
    print_fail "Empty message encoding command failed"
fi

print_section "Test 6: JPEG to PNG conversion"
print_test "Encoding with JPEG input (should output PNG)"
if cargo run --quiet -- encode \
    --image-list test/tmp/images/04.jpg test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output-dir test/tmp/output_list 2>&1 >/dev/null; then

    if [ -f test/tmp/output_list/04.png ]; then
        print_pass "JPEG input converted to PNG output"
    else
        print_fail "JPEG to PNG conversion failed"
    fi
else
    print_fail "JPEG encoding command failed"
fi

print_section "Test 7: Parameter validation"
print_test "Testing mutually exclusive --image and --image-list"
OUTPUT=$(cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --image-list test/tmp/images/02.png \
    --message test/tmp/messages/short.txt \
    --output-dir test/tmp/output_single 2>&1)

if echo "$OUTPUT" | grep -q "Only one of"; then
    print_pass "Mutually exclusive parameters rejected correctly"
else
    print_fail "Mutually exclusive parameters not rejected"
fi

print_test "Testing --output-dir required with --image-list"
OUTPUT=$(cargo run --quiet -- encode \
    --image-list test/tmp/images/01.png test/tmp/images/02.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/test.png 2>&1)

if echo "$OUTPUT" | grep -q "output-dir is required"; then
    print_pass "Missing --output-dir detected correctly"
else
    print_fail "Missing --output-dir not detected"
fi

print_test "Testing --output required with --image"
OUTPUT=$(cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output-dir test/tmp/output_single 2>&1)

if echo "$OUTPUT" | grep -q "output is required"; then
    print_pass "Missing --output detected correctly"
else
    print_fail "Missing --output not detected"
fi

print_section "Test 8: Error handling"
print_test "Testing message too long for image capacity"
dd if=/dev/zero of=test/tmp/messages/huge.txt bs=1M count=1 2>/dev/null

OUTPUT=$(cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/huge.txt \
    --output test/tmp/output_single/huge.png 2>&1)

if echo "$OUTPUT" | grep -q "too long"; then
    print_pass "Message too long error handled correctly"
else
    print_fail "Message too long error not handled"
fi

print_test "Testing invalid image directory"
OUTPUT=$(cargo run --quiet -- encode \
    --image-dir test/tmp/nonexistent \
    --message test/tmp/messages/short.txt \
    --output-dir test/tmp/output_dir 2>&1)

if echo "$OUTPUT" | grep -q "not a directory\|No such file"; then
    print_pass "Invalid directory handled correctly"
else
    print_fail "Invalid directory not handled"
fi

print_section "Test 9: Sequence metadata - renamed files"
print_test "Creating large message requiring multiple images"
# Create 50KB message to ensure it spans multiple 200x200 images (each can hold ~19.5KB)
dd if=/dev/urandom of=test/tmp/messages/multi_image.txt bs=1024 count=50 2>/dev/null

print_test "Encoding message into multiple images"
if cargo run --quiet -- encode \
    --image-list test/tmp/images/01.png test/tmp/images/02.png test/tmp/images/03.png \
    --message test/tmp/messages/multi_image.txt \
    --output-dir test/tmp/output_list/seq_test 2>&1 >/dev/null; then

    if [ -f test/tmp/output_list/seq_test/01.png ] && [ -f test/tmp/output_list/seq_test/02.png ] && [ -f test/tmp/output_list/seq_test/03.png ]; then
        print_pass "Multiple images encoded with sequence metadata"

        print_test "Renaming files to random names"
        mkdir -p test/tmp/output_list/renamed
        cp test/tmp/output_list/seq_test/01.png test/tmp/output_list/renamed/zebra.png
        cp test/tmp/output_list/seq_test/02.png test/tmp/output_list/renamed/apple.png
        cp test/tmp/output_list/seq_test/03.png test/tmp/output_list/renamed/mango.png

        print_test "Decoding from renamed files (alphabetically wrong order)"
        if cargo run --quiet -- decode \
            --image-list test/tmp/output_list/renamed/zebra.png test/tmp/output_list/renamed/mango.png test/tmp/output_list/renamed/apple.png \
            --output test/tmp/output_list/renamed/decoded.txt 2>&1 >/dev/null; then

            if [ -f test/tmp/output_list/renamed/decoded.txt ]; then
                if diff -q test/tmp/messages/multi_image.txt test/tmp/output_list/renamed/decoded.txt >/dev/null 2>&1; then
                    print_pass "Sequence metadata allows correct decoding despite file renaming"
                else
                    print_fail "Decoded message does not match original after file renaming"
                fi
            else
                print_fail "Decoding renamed files failed - output file not created"
            fi
        else
            print_fail "Decoding renamed files command failed"
        fi
    else
        print_fail "Multiple images encoding with sequence metadata failed - output files not created"
    fi
else
    print_fail "Multiple images encoding with sequence metadata command failed"
fi

print_section "Test 10: Encryption key parameter"
print_test "Encoding with custom key"
if cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/custom_key.png \
    --key "my-secret-password" 2>&1 >/dev/null; then

    if [ -f test/tmp/output_single/custom_key.png ]; then
        print_pass "Encoding with custom key successful"
    else
        print_fail "Encoding with custom key failed - output file not created"
    fi
else
    print_fail "Encoding with custom key command failed"
fi

print_test "Decoding with correct custom key"
if cargo run --quiet -- decode \
    --image test/tmp/output_single/custom_key.png \
    --output test/tmp/output_single/custom_key_decoded.txt \
    --key "my-secret-password" 2>&1 >/dev/null; then

    if [ -f test/tmp/output_single/custom_key_decoded.txt ]; then
        if diff -q test/tmp/messages/short.txt test/tmp/output_single/custom_key_decoded.txt >/dev/null 2>&1; then
            print_pass "Decoding with correct custom key successful"
        else
            print_fail "Decoded message does not match original"
        fi
    else
        print_fail "Decoding with correct custom key failed - output file not created"
    fi
else
    print_fail "Decoding with correct custom key command failed"
fi

print_test "Decoding with wrong key (should fail)"
OUTPUT=$(cargo run --quiet -- decode \
    --image test/tmp/output_single/custom_key.png \
    --output test/tmp/output_single/wrong_key_decoded.txt \
    --key "wrong-password" 2>&1)

if echo "$OUTPUT" | grep -q "Decryption failed"; then
    print_pass "Decoding with wrong key correctly rejected"
else
    print_fail "Decoding with wrong key not rejected"
fi

print_test "Encoding with empty key (should fail)"
OUTPUT=$(cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/empty_key.png \
    --key "" 2>&1)

if echo "$OUTPUT" | grep -q "cannot be empty"; then
    print_pass "Empty key correctly rejected for encoding"
else
    print_fail "Empty key not rejected for encoding"
fi

print_test "Decoding with empty key (should fail)"
OUTPUT=$(cargo run --quiet -- decode \
    --image test/tmp/output_single/encoded.png \
    --output test/tmp/output_single/empty_key_decoded.txt \
    --key "" 2>&1)

if echo "$OUTPUT" | grep -q "cannot be empty"; then
    print_pass "Empty key correctly rejected for decoding"
else
    print_fail "Empty key not rejected for decoding"
fi

print_test "Testing default key encode/decode"
if cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/default_key.png 2>&1 >/dev/null; then

    if cargo run --quiet -- decode \
        --image test/tmp/output_single/default_key.png \
        --output test/tmp/output_single/default_key_decoded.txt 2>&1 >/dev/null; then

        if diff -q test/tmp/messages/short.txt test/tmp/output_single/default_key_decoded.txt >/dev/null 2>&1; then
            print_pass "Default key encode/decode works correctly"
        else
            print_fail "Default key decoded message does not match original"
        fi
    else
        print_fail "Default key decoding failed"
    fi
else
    print_fail "Default key encoding failed"
fi

print_test "Testing Unicode key (Chinese + emoji)"
if cargo run --quiet -- encode \
    --image test/tmp/images/01.png \
    --message test/tmp/messages/short.txt \
    --output test/tmp/output_single/unicode_key.png \
    --key "MyKey🔐" 2>&1 >/dev/null; then

    if cargo run --quiet -- decode \
        --image test/tmp/output_single/unicode_key.png \
        --output test/tmp/output_single/unicode_key_decoded.txt \
        --key "MyKey🔐" 2>&1 >/dev/null; then

        if diff -q test/tmp/messages/short.txt test/tmp/output_single/unicode_key_decoded.txt >/dev/null 2>&1; then
            print_pass "Unicode key (Chinese + emoji) works correctly"
        else
            print_fail "Unicode key decoded message does not match original"
        fi
    else
        print_fail "Unicode key decoding failed"
    fi
else
    print_fail "Unicode key encoding failed"
fi

print_test "Testing multi-image with custom key"
if cargo run --quiet -- encode \
    --image-list test/tmp/images/01.png test/tmp/images/02.png \
    --message test/tmp/messages/long.txt \
    --output-dir test/tmp/output_list/custom_key \
    --key "multi-image-secret" 2>&1 >/dev/null; then

    if cargo run --quiet -- decode \
        --image-dir test/tmp/output_list/custom_key \
        --output test/tmp/output_list/custom_key/decoded.txt \
        --key "multi-image-secret" 2>&1 >/dev/null; then

        if diff -q test/tmp/messages/long.txt test/tmp/output_list/custom_key/decoded.txt >/dev/null 2>&1; then
            print_pass "Multi-image with custom key works correctly"
        else
            print_fail "Multi-image custom key decoded message does not match original"
        fi
    else
        print_fail "Multi-image custom key decoding failed"
    fi
else
    print_fail "Multi-image custom key encoding failed"
fi

print_section "Test Summary"
TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NO_COLOR}"
echo -e "Failed: ${RED}$TESTS_FAILED${NO_COLOR}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NO_COLOR}"
    exit 0
else
    echo -e "\n${RED}Some tests failed.${NO_COLOR}"
    exit 1
fi
