#!/bin/bash
# Resume an existing AFL fuzzing campaign

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Resuming AFL fuzzing campaign${NC}"

# Check if findings directory exists
if [ ! -d "fuzz/findings/default/queue" ]; then
    echo -e "${RED}Error: No existing campaign found in fuzz/findings/${NC}"
    echo "Use scripts/fuzz_start.sh to start a new campaign"
    exit 1
fi

# Check if venv binaries exist
if [ ! -f ".venv/bin/py-afl-fuzz" ] || [ ! -f ".venv/bin/python" ]; then
    echo -e "${RED}Error: Virtual environment not found. Run: uv sync${NC}"
    exit 1
fi

# Show stats from previous run
if [ -f "fuzz/findings/default/fuzzer_stats" ]; then
    echo -e "${YELLOW}Previous campaign stats:${NC}"
    grep -E "start_time|last_update|execs_done|execs_per_sec|paths_total|uniq_crashes|uniq_hangs" fuzz/findings/default/fuzzer_stats | head -10
    echo ""
fi

echo -e "${GREEN}Resuming fuzzing with persistent mode${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop fuzzing${NC}"
echo ""

# Skip CPU frequency check on macOS
export AFL_SKIP_CPUFREQ=1

# Resume with -i- flag (reuse existing queue)
# Use venv binaries directly, not through 'uv run'
.venv/bin/py-afl-fuzz \
    -i- \
    -o fuzz/findings \
    -m none \
    -- .venv/bin/python fuzz/fuzz_parser_persistent.py

echo -e "${GREEN}Fuzzing session ended${NC}"
