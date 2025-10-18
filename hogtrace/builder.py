"""
AST Builder for HogTrace.

Converts ANTLR parse trees into clean AST objects.
"""

import sys
from pathlib import Path
from antlr4 import ParseTreeWalker

# Add generated/ to path so we can import the ANTLR-generated files
_generated_dir = Path(__file__).parent.parent / "generated"
if str(_generated_dir) not in sys.path:
    sys.path.insert(0, str(_generated_dir))

from HogTraceParser import HogTraceParser
from HogTraceListener import HogTraceListener
from hogtrace.ast import (
    Program, Probe, ProbeSpec, Predicate, Action,
    CaptureAction, AssignmentAction, SampleAction,
    Expression, Literal, Identifier, FieldAccess, IndexAccess,
    FunctionCall, RequestVar, BinaryOp, UnaryOp
)


class ASTBuilder(HogTraceListener):
    """
    Builds a clean AST from the ANTLR parse tree.

    This visitor walks the parse tree and constructs Program/Probe/Action objects.
    """

    def __init__(self):
        self.program = Program()
        self.current_probe = None
        self.current_predicate = None

    def build(self, parse_tree) -> Program:
        """Walk the parse tree and build the AST"""
        walker = ParseTreeWalker()
        walker.walk(self, parse_tree)
        return self.program

    # ===== Probe Definition =====

    def enterProbe(self, ctx: HogTraceParser.ProbeContext):
        """Start a new probe"""
        self.current_probe = Probe(
            spec=None,  # Will be filled in enterProbeSpec
            predicate=None,
            actions=[]
        )

    def exitProbe(self, ctx: HogTraceParser.ProbeContext):
        """Finish the probe and add to program"""
        if self.current_probe:
            self.program.probes.append(self.current_probe)
            self.current_probe = None

    def enterProbeSpec(self, ctx: HogTraceParser.ProbeSpecContext):
        """Parse probe specification"""
        provider = ctx.provider.text
        module_func = ctx.moduleFunction().getText()
        probe_point = ctx.probePoint().getText()
        full_spec = f"{provider}:{module_func}:{probe_point}"

        self.current_probe.spec = ProbeSpec(
            provider=provider,
            module_function=module_func,
            probe_point=probe_point,
            full_spec=full_spec
        )

    # ===== Predicate =====

    def enterPredicate(self, ctx: HogTraceParser.PredicateContext):
        """Parse predicate expression"""
        expr = self._build_expression(ctx.expression())
        self.current_probe.predicate = Predicate(expression=expr)

    # ===== Actions =====

    def enterCaptureStatement(self, ctx: HogTraceParser.CaptureStatementContext):
        """Parse capture/send statement"""
        function = ctx.getChild(0).getText()  # "capture" or "send"

        arguments = []
        named_arguments = {}

        if ctx.captureArgs():
            args_ctx = ctx.captureArgs()

            # Check if it's named or positional args
            if isinstance(args_ctx, HogTraceParser.NamedCaptureArgsContext):
                # Named arguments
                for named_arg_ctx in args_ctx.namedArg():
                    name = named_arg_ctx.IDENTIFIER().getText()
                    expr = self._build_expression(named_arg_ctx.expression())
                    named_arguments[name] = expr
            else:
                # Positional arguments
                for expr_ctx in args_ctx.expression():
                    expr = self._build_expression(expr_ctx)
                    arguments.append(expr)

        action = CaptureAction(
            function=function,
            arguments=arguments,
            named_arguments=named_arguments
        )
        self.current_probe.actions.append(action)

    def enterAssignment(self, ctx: HogTraceParser.AssignmentContext):
        """Parse assignment to request variable"""
        var = self._build_request_var(ctx.requestVar())
        value = self._build_expression(ctx.expression())

        action = AssignmentAction(variable=var, value=value)
        self.current_probe.actions.append(action)

    def enterSampleDirective(self, ctx: HogTraceParser.SampleDirectiveContext):
        """Parse sample directive"""
        spec_ctx = ctx.sampleSpec()
        spec_text = spec_ctx.getText()

        if isinstance(spec_ctx, HogTraceParser.PercentageSampleContext):
            # Percentage: "10%"
            percentage = int(spec_ctx.INT().getText())
            action = SampleAction(
                spec=spec_text,
                is_percentage=True,
                value=percentage / 100.0
            )
        else:
            # Ratio: "1/100"
            ints = spec_ctx.INT()
            numerator = int(ints[0].getText())
            denominator = int(ints[1].getText())
            action = SampleAction(
                spec=spec_text,
                is_percentage=False,
                numerator=numerator,
                denominator=denominator,
                value=numerator / denominator if denominator != 0 else 0
            )

        self.current_probe.actions.append(action)

    # ===== Expression Building =====

    def _build_expression(self, ctx) -> Expression:
        """Build an expression from a parse tree context"""
        if ctx is None:
            return None

        raw = ctx.getText()

        # Literal
        if isinstance(ctx, HogTraceParser.LiteralExprContext):
            return self._build_literal(ctx.literal())

        # Identifier
        if isinstance(ctx, HogTraceParser.IdentifierExprContext):
            return Identifier(ctx.IDENTIFIER().getText())

        # Request variable
        if isinstance(ctx, HogTraceParser.RequestVarExprContext):
            return self._build_request_var(ctx.requestVar())

        # Field access (obj.field)
        if isinstance(ctx, HogTraceParser.FieldAccessContext):
            obj = self._build_expression(ctx.expression())
            field = ctx.IDENTIFIER().getText()
            return FieldAccess(obj, field, raw)

        # Index access (obj[index])
        if isinstance(ctx, HogTraceParser.IndexAccessContext):
            obj = self._build_expression(ctx.expression(0))
            index = self._build_expression(ctx.expression(1))
            return IndexAccess(obj, index, raw)

        # Function call
        if isinstance(ctx, HogTraceParser.FunctionCallContext):
            func_name = ctx.IDENTIFIER().getText()
            args = []
            if ctx.expressionList():
                for expr_ctx in ctx.expressionList().expression():
                    args.append(self._build_expression(expr_ctx))
            return FunctionCall(func_name, args, raw)

        # Parenthesized expression
        if isinstance(ctx, HogTraceParser.ParenExprContext):
            return self._build_expression(ctx.expression())

        # Unary operator
        if isinstance(ctx, HogTraceParser.NotExprContext):
            operand = self._build_expression(ctx.expression())
            return UnaryOp("!", operand, raw)

        # Binary operators
        if isinstance(ctx, (
            HogTraceParser.MulDivModExprContext,
            HogTraceParser.AddSubExprContext,
            HogTraceParser.ComparisonExprContext,
            HogTraceParser.EqualityExprContext,
            HogTraceParser.AndExprContext,
            HogTraceParser.OrExprContext
        )):
            left = self._build_expression(ctx.expression(0))
            right = self._build_expression(ctx.expression(1))
            operator = ctx.op.text
            return BinaryOp(operator, left, right, raw)

        # Fallback: return raw text as identifier
        return Identifier(raw)

    def _build_literal(self, ctx) -> Literal:
        """Build a literal value"""
        raw = ctx.getText()

        if isinstance(ctx, HogTraceParser.IntLiteralContext):
            return Literal(int(ctx.INT().getText()), raw)

        if isinstance(ctx, HogTraceParser.FloatLiteralContext):
            return Literal(float(ctx.FLOAT().getText()), raw)

        if isinstance(ctx, HogTraceParser.StringLiteralContext):
            # Remove quotes
            text = ctx.STRING().getText()
            value = text[1:-1]  # Strip quotes
            # Handle escape sequences
            value = value.replace('\\n', '\n').replace('\\t', '\t').replace('\\"', '"').replace("\\'", "'")
            return Literal(value, raw)

        if isinstance(ctx, HogTraceParser.BoolLiteralContext):
            value = ctx.BOOL().getText() == "True"
            return Literal(value, raw)

        if isinstance(ctx, HogTraceParser.NoneLiteralContext):
            return Literal(None, raw)

        # Fallback
        return Literal(raw, raw)

    def _build_request_var(self, ctx) -> RequestVar:
        """Build a request-scoped variable reference"""
        raw = ctx.getText()
        # Extract prefix and name
        # Format: $req.name or $request.name
        parts = raw.split('.', 1)
        prefix = parts[0][1:]  # Remove $
        name = parts[1] if len(parts) > 1 else ""

        return RequestVar(name, prefix, raw)
