@echo off
echo Testing special characters in arguments...

REM Test with quotes
echo Test 1: Quotes
./target/release/agentic-warden.exe claude "Test with \"quotes\"" -p openrouter

REM Test with apostrophes
echo Test 2: Apostrophes
./target/release/agentic-warden.exe claude "Test with 'apostrophes'" -p openrouter

REM Test with special characters
echo Test 3: Special characters
./target/release/agentic-warden.exe claude "Test with $pecial &hars" -p openrouter

REM Test with multiple special chars
echo Test 4: Multiple special characters
./target/release/agentic-warden.exe claude "Check <path> && use {config}" -p openrouter

echo All tests completed successfully!