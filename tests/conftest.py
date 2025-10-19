"""
Pytest configuration for HogTrace tests.
"""

import sys
from pathlib import Path

# Add parent directory to Python path so tests can import hogtrace
sys.path.insert(0, str(Path(__file__).parent.parent))
