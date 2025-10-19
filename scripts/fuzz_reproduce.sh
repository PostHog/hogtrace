#!/bin/bash
# Reproduce a crash or hang found by AFL

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if [ $# -eq 0 ]; then
    echo -e "${YELLOW}Usage: $0 <crash_file>${NC}"
    echo ""
    echo "Examples:"
    echo "  $0 fuzz/findings/default/crashes/id:000000,sig:06,src:000000,op:havoc,rep:4"
    echo "  $0 fuzz/findings/default/hangs/id:000000,src:000000,op:havoc,rep:2"
    echo ""

    # List available crashes and hangs
    if [ -d "fuzz/findings/default/crashes" ] && [ -n "$(ls -A fuzz/findings/default/crashes 2>/dev/null | grep -v README)" ]; then
        echo -e "${RED}Available crashes:${NC}"
        ls -lh fuzz/findings/default/crashes/ | grep -v README | grep -v "^total"
        echo ""
    fi

    if [ -d "fuzz/findings/default/hangs" ] && [ -n "$(ls -A fuzz/findings/default/hangs 2>/dev/null | grep -v README)" ]; then
        echo -e "${YELLOW}Available hangs:${NC}"
        ls -lh fuzz/findings/default/hangs/ | grep -v README | grep -v "^total"
        echo ""
    fi

    if [ ! -d "fuzz/findings/default/crashes" ] && [ ! -d "fuzz/findings/default/hangs" ]; then
        echo -e "${GREEN}No crashes or hangs found yet. Keep fuzzing!${NC}"
    fi

    exit 1
fi

CRASH_FILE="$1"

if [ ! -f "$CRASH_FILE" ]; then
    echo -e "${RED}Error: File not found: $CRASH_FILE${NC}"
    exit 1
fi

echo -e "${GREEN}Reproducing crash/hang with file: $CRASH_FILE${NC}"
echo ""
echo -e "${YELLOW}File contents (first 500 bytes):${NC}"
head -c 500 "$CRASH_FILE" | cat -v
echo ""
echo ""

echo -e "${YELLOW}Running parser without AFL instrumentation:${NC}"
echo ""

# Run without AFL to see the actual error
# Use venv python directly
.venv/bin/python fuzz/fuzz_parser.py < "$CRASH_FILE"

echo ""
echo -e "${GREEN}Reproduction complete${NC}"
