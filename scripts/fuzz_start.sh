#!/bin/bash
# Start a new AFL fuzzing campaign for HogTrace parser
# This will use persistent mode for maximum performance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting AFL fuzzing campaign for HogTrace parser${NC}"

# Check if AFL is installed
if ! command -v afl-fuzz &> /dev/null; then
    echo -e "${RED}Error: afl-fuzz not found. Please install AFL:${NC}"
    echo "  macOS: brew install afl-fuzz"
    echo "  Linux: sudo apt-get install afl"
    exit 1
fi

# Check if python-afl is installed
if [ ! -f ".venv/bin/py-afl-fuzz" ]; then
    echo -e "${RED}Error: python-afl not installed. Run: uv add --dev python-afl${NC}"
    exit 1
fi

if [ ! -f ".venv/bin/python" ]; then
    echo -e "${RED}Error: Virtual environment not found. Run: uv sync${NC}"
    exit 1
fi

# Check if python-afl module is available
if ! .venv/bin/python -c "import afl" 2>/dev/null; then
    echo -e "${RED}Error: python-afl not installed. Run: uv add --dev python-afl${NC}"
    exit 1
fi

# macOS-specific: Check crash reporter
if [[ "$OSTYPE" == "darwin"* ]]; then
    if launchctl list | grep -q com.apple.ReportCrash; then
        echo -e "${RED}Error: macOS crash reporter is enabled and will interfere with AFL${NC}"
        echo -e "${YELLOW}To disable it, run these commands:${NC}"
        echo ""
        echo "  SL=/System/Library; PL=com.apple.ReportCrash"
        echo "  launchctl unload -w \${SL}/LaunchAgents/\${PL}.plist"
        echo "  sudo launchctl unload -w \${SL}/LaunchDaemons/\${PL}.Root.plist"
        echo ""
        echo -e "${YELLOW}To re-enable after fuzzing:${NC}"
        echo "  launchctl load -w \${SL}/LaunchAgents/\${PL}.plist"
        echo "  sudo launchctl load -w \${SL}/LaunchDaemons/\${PL}.Root.plist"
        echo ""
        exit 1
    fi
fi

# Create findings directory
mkdir -p fuzz/findings

# Check if corpus exists
if [ ! -d "fuzz/corpus" ] || [ -z "$(ls -A fuzz/corpus)" ]; then
    echo -e "${RED}Error: No seed corpus found in fuzz/corpus/${NC}"
    exit 1
fi

echo -e "${YELLOW}Corpus files:${NC}"
ls -lh fuzz/corpus/

# Check for existing findings
if [ -d "fuzz/findings/queue" ] && [ -n "$(ls -A fuzz/findings/queue)" ]; then
    echo -e "${YELLOW}Warning: Existing findings detected in fuzz/findings/${NC}"
    echo "Use scripts/fuzz_resume.sh to continue, or remove fuzz/findings/ to start fresh"
    exit 1
fi

# Start fuzzing with persistent mode target
echo -e "${GREEN}Starting AFL with persistent mode (1000 iterations per spawn)${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop fuzzing${NC}"
echo ""

# Skip CPU frequency check on macOS (not applicable)
export AFL_SKIP_CPUFREQ=1

# Use py-afl-fuzz wrapper which sets up instrumentation
# Important: Use venv binaries directly, not through 'uv run'
.venv/bin/py-afl-fuzz \
    -i fuzz/corpus \
    -o fuzz/findings \
    -m none \
    -- .venv/bin/python fuzz/fuzz_parser_persistent.py

echo -e "${GREEN}Fuzzing session ended${NC}"
