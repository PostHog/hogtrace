#!/usr/bin/env python3
"""
Example of using the HogTrace parser programmatically.

This shows how to:
1. Parse HogTrace code
2. Walk the parse tree
3. Extract information from probes
"""

import sys
from pathlib import Path
from antlr4 import *

# Add generated/ to path so we can import the ANTLR-generated files
_generated_dir = Path(__file__).parent.parent / "generated"
if str(_generated_dir) not in sys.path:
    sys.path.insert(0, str(_generated_dir))

from HogTraceLexer import HogTraceLexer
from HogTraceParser import HogTraceParser
from HogTraceListener import HogTraceListener


class HogTraceExtractor(HogTraceListener):
    """
    Example listener that extracts information from HogTrace programs.

    This walks the parse tree and prints information about each probe.
    You would extend this to build an AST or generate bytecode.
    """

    def __init__(self):
        self.probes = []
        self.current_probe = None

    def enterProbe(self, ctx: HogTraceParser.ProbeContext):
        """Called when entering a probe definition"""
        self.current_probe = {
            'spec': None,
            'predicate': None,
            'actions': []
        }

    def exitProbe(self, ctx: HogTraceParser.ProbeContext):
        """Called when exiting a probe definition"""
        if self.current_probe:
            self.probes.append(self.current_probe)
            self.current_probe = None

    def enterProbeSpec(self, ctx: HogTraceParser.ProbeSpecContext):
        """Extract probe specification"""
        provider = ctx.provider.text
        module_func = ctx.moduleFunction().getText()
        probe_point = ctx.probePoint().getText()

        self.current_probe['spec'] = {
            'provider': provider,
            'module_function': module_func,
            'probe_point': probe_point,
            'full_spec': f"{provider}:{module_func}:{probe_point}"
        }

    def enterPredicate(self, ctx: HogTraceParser.PredicateContext):
        """Extract predicate expression"""
        # Get the text between the / / delimiters
        predicate_text = ctx.expression().getText()
        self.current_probe['predicate'] = predicate_text

    def enterCaptureStatement(self, ctx: HogTraceParser.CaptureStatementContext):
        """Extract capture/send statements"""
        action = ctx.getChild(0).getText()  # 'capture' or 'send'

        args = []
        if ctx.captureArgs():
            # Extract argument expressions
            for child in ctx.captureArgs().children:
                if isinstance(child, HogTraceParser.ExpressionContext):
                    args.append(child.getText())
                elif isinstance(child, HogTraceParser.NamedArgContext):
                    name = child.IDENTIFIER().getText()
                    value = child.expression().getText()
                    args.append(f"{name}={value}")

        self.current_probe['actions'].append({
            'type': 'capture',
            'function': action,
            'args': args
        })

    def enterSampleDirective(self, ctx: HogTraceParser.SampleDirectiveContext):
        """Extract sampling directives"""
        sample_spec = ctx.sampleSpec().getText()
        self.current_probe['actions'].append({
            'type': 'sample',
            'spec': sample_spec
        })

    def enterAssignment(self, ctx: HogTraceParser.AssignmentContext):
        """Extract request variable assignments"""
        var_name = ctx.requestVar().getText()
        value = ctx.expression().getText()

        self.current_probe['actions'].append({
            'type': 'assignment',
            'variable': var_name,
            'value': value
        })

    def print_summary(self):
        """Print a summary of extracted probes"""
        print(f"\n{'='*60}")
        print(f"Found {len(self.probes)} probe(s)")
        print(f"{'='*60}\n")

        for i, probe in enumerate(self.probes, 1):
            print(f"Probe #{i}: {probe['spec']['full_spec']}")

            if probe['predicate']:
                print(f"  Predicate: {probe['predicate']}")

            if probe['actions']:
                print(f"  Actions:")
                for action in probe['actions']:
                    if action['type'] == 'capture':
                        args_str = ', '.join(action['args'])
                        print(f"    - {action['function']}({args_str})")
                    elif action['type'] == 'sample':
                        print(f"    - sample {action['spec']}")
                    elif action['type'] == 'assignment':
                        print(f"    - {action['variable']} = {action['value']}")

            print()


def parse_hogtrace_file(filename: str):
    """Parse a HogTrace file and extract probe information"""

    # Read the file
    with open(filename, 'r') as f:
        input_stream = InputStream(f.read())

    # Create lexer and parser
    lexer = HogTraceLexer(input_stream)
    stream = CommonTokenStream(lexer)
    parser = HogTraceParser(stream)

    # Parse the program
    tree = parser.program()

    # Walk the tree with our extractor
    extractor = HogTraceExtractor()
    walker = ParseTreeWalker()
    walker.walk(extractor, tree)

    return extractor


def main():
    """Example usage"""

    # Example 1: Parse a simple HogTrace program
    print("Example 1: Simple probe")
    print("-" * 60)

    simple_code = """
    fn:myapp.users.create_user:entry
    / arg0.role == "admin" /
    {
        capture(args);
    }
    """

    input_stream = InputStream(simple_code)
    lexer = HogTraceLexer(input_stream)
    stream = CommonTokenStream(lexer)
    parser = HogTraceParser(stream)
    tree = parser.program()

    extractor = HogTraceExtractor()
    walker = ParseTreeWalker()
    walker.walk(extractor, tree)
    extractor.print_summary()

    # Example 2: Parse a complex multi-probe program
    print("\nExample 2: Request tracking")
    print("-" * 60)

    complex_code = """
    fn:django.core.handlers.wsgi.WSGIHandler:entry
    {
        $req.request_id = arg0.META["REQUEST_ID"];
        $req.start_time = timestamp();
    }

    fn:myapp.db.execute_query:entry
    / $req.request_id != None /
    {
        sample 10%;
        capture(query=$req.request_id, sql=arg0);
    }

    fn:django.core.handlers.wsgi.WSGIHandler:exit
    {
        send(
            request_id=$req.request_id,
            duration=timestamp()-$req.start_time,
            status=retval.status_code
        );
    }
    """

    input_stream = InputStream(complex_code)
    lexer = HogTraceLexer(input_stream)
    stream = CommonTokenStream(lexer)
    parser = HogTraceParser(stream)
    tree = parser.program()

    extractor = HogTraceExtractor()
    walker = ParseTreeWalker()
    walker.walk(extractor, tree)
    extractor.print_summary()

    # Example 3: Parse from file
    print("\nExample 3: Parsing tests/test_examples.hogtrace")
    print("-" * 60)

    try:
        extractor = parse_hogtrace_file('tests/test_examples.hogtrace')
        extractor.print_summary()
    except FileNotFoundError:
        print("File not found: tests/test_examples.hogtrace")


if __name__ == "__main__":
    main()
