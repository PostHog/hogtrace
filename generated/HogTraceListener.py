# Generated from HogTrace.g4 by ANTLR 4.13.2
from antlr4 import *
if "." in __name__:
    from .HogTraceParser import HogTraceParser
else:
    from HogTraceParser import HogTraceParser

# This class defines a complete listener for a parse tree produced by HogTraceParser.
class HogTraceListener(ParseTreeListener):

    # Enter a parse tree produced by HogTraceParser#program.
    def enterProgram(self, ctx:HogTraceParser.ProgramContext):
        pass

    # Exit a parse tree produced by HogTraceParser#program.
    def exitProgram(self, ctx:HogTraceParser.ProgramContext):
        pass


    # Enter a parse tree produced by HogTraceParser#probe.
    def enterProbe(self, ctx:HogTraceParser.ProbeContext):
        pass

    # Exit a parse tree produced by HogTraceParser#probe.
    def exitProbe(self, ctx:HogTraceParser.ProbeContext):
        pass


    # Enter a parse tree produced by HogTraceParser#probeSpec.
    def enterProbeSpec(self, ctx:HogTraceParser.ProbeSpecContext):
        pass

    # Exit a parse tree produced by HogTraceParser#probeSpec.
    def exitProbeSpec(self, ctx:HogTraceParser.ProbeSpecContext):
        pass


    # Enter a parse tree produced by HogTraceParser#moduleFunction.
    def enterModuleFunction(self, ctx:HogTraceParser.ModuleFunctionContext):
        pass

    # Exit a parse tree produced by HogTraceParser#moduleFunction.
    def exitModuleFunction(self, ctx:HogTraceParser.ModuleFunctionContext):
        pass


    # Enter a parse tree produced by HogTraceParser#EntryProbe.
    def enterEntryProbe(self, ctx:HogTraceParser.EntryProbeContext):
        pass

    # Exit a parse tree produced by HogTraceParser#EntryProbe.
    def exitEntryProbe(self, ctx:HogTraceParser.EntryProbeContext):
        pass


    # Enter a parse tree produced by HogTraceParser#ExitProbe.
    def enterExitProbe(self, ctx:HogTraceParser.ExitProbeContext):
        pass

    # Exit a parse tree produced by HogTraceParser#ExitProbe.
    def exitExitProbe(self, ctx:HogTraceParser.ExitProbeContext):
        pass


    # Enter a parse tree produced by HogTraceParser#EntryOffsetProbe.
    def enterEntryOffsetProbe(self, ctx:HogTraceParser.EntryOffsetProbeContext):
        pass

    # Exit a parse tree produced by HogTraceParser#EntryOffsetProbe.
    def exitEntryOffsetProbe(self, ctx:HogTraceParser.EntryOffsetProbeContext):
        pass


    # Enter a parse tree produced by HogTraceParser#ExitOffsetProbe.
    def enterExitOffsetProbe(self, ctx:HogTraceParser.ExitOffsetProbeContext):
        pass

    # Exit a parse tree produced by HogTraceParser#ExitOffsetProbe.
    def exitExitOffsetProbe(self, ctx:HogTraceParser.ExitOffsetProbeContext):
        pass


    # Enter a parse tree produced by HogTraceParser#predicate.
    def enterPredicate(self, ctx:HogTraceParser.PredicateContext):
        pass

    # Exit a parse tree produced by HogTraceParser#predicate.
    def exitPredicate(self, ctx:HogTraceParser.PredicateContext):
        pass


    # Enter a parse tree produced by HogTraceParser#action.
    def enterAction(self, ctx:HogTraceParser.ActionContext):
        pass

    # Exit a parse tree produced by HogTraceParser#action.
    def exitAction(self, ctx:HogTraceParser.ActionContext):
        pass


    # Enter a parse tree produced by HogTraceParser#statement.
    def enterStatement(self, ctx:HogTraceParser.StatementContext):
        pass

    # Exit a parse tree produced by HogTraceParser#statement.
    def exitStatement(self, ctx:HogTraceParser.StatementContext):
        pass


    # Enter a parse tree produced by HogTraceParser#assignment.
    def enterAssignment(self, ctx:HogTraceParser.AssignmentContext):
        pass

    # Exit a parse tree produced by HogTraceParser#assignment.
    def exitAssignment(self, ctx:HogTraceParser.AssignmentContext):
        pass


    # Enter a parse tree produced by HogTraceParser#requestVar.
    def enterRequestVar(self, ctx:HogTraceParser.RequestVarContext):
        pass

    # Exit a parse tree produced by HogTraceParser#requestVar.
    def exitRequestVar(self, ctx:HogTraceParser.RequestVarContext):
        pass


    # Enter a parse tree produced by HogTraceParser#sampleDirective.
    def enterSampleDirective(self, ctx:HogTraceParser.SampleDirectiveContext):
        pass

    # Exit a parse tree produced by HogTraceParser#sampleDirective.
    def exitSampleDirective(self, ctx:HogTraceParser.SampleDirectiveContext):
        pass


    # Enter a parse tree produced by HogTraceParser#PercentageSample.
    def enterPercentageSample(self, ctx:HogTraceParser.PercentageSampleContext):
        pass

    # Exit a parse tree produced by HogTraceParser#PercentageSample.
    def exitPercentageSample(self, ctx:HogTraceParser.PercentageSampleContext):
        pass


    # Enter a parse tree produced by HogTraceParser#RatioSample.
    def enterRatioSample(self, ctx:HogTraceParser.RatioSampleContext):
        pass

    # Exit a parse tree produced by HogTraceParser#RatioSample.
    def exitRatioSample(self, ctx:HogTraceParser.RatioSampleContext):
        pass


    # Enter a parse tree produced by HogTraceParser#captureStatement.
    def enterCaptureStatement(self, ctx:HogTraceParser.CaptureStatementContext):
        pass

    # Exit a parse tree produced by HogTraceParser#captureStatement.
    def exitCaptureStatement(self, ctx:HogTraceParser.CaptureStatementContext):
        pass


    # Enter a parse tree produced by HogTraceParser#NamedCaptureArgs.
    def enterNamedCaptureArgs(self, ctx:HogTraceParser.NamedCaptureArgsContext):
        pass

    # Exit a parse tree produced by HogTraceParser#NamedCaptureArgs.
    def exitNamedCaptureArgs(self, ctx:HogTraceParser.NamedCaptureArgsContext):
        pass


    # Enter a parse tree produced by HogTraceParser#PositionalCaptureArgs.
    def enterPositionalCaptureArgs(self, ctx:HogTraceParser.PositionalCaptureArgsContext):
        pass

    # Exit a parse tree produced by HogTraceParser#PositionalCaptureArgs.
    def exitPositionalCaptureArgs(self, ctx:HogTraceParser.PositionalCaptureArgsContext):
        pass


    # Enter a parse tree produced by HogTraceParser#namedArg.
    def enterNamedArg(self, ctx:HogTraceParser.NamedArgContext):
        pass

    # Exit a parse tree produced by HogTraceParser#namedArg.
    def exitNamedArg(self, ctx:HogTraceParser.NamedArgContext):
        pass


    # Enter a parse tree produced by HogTraceParser#AndExpr.
    def enterAndExpr(self, ctx:HogTraceParser.AndExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#AndExpr.
    def exitAndExpr(self, ctx:HogTraceParser.AndExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#ComparisonExpr.
    def enterComparisonExpr(self, ctx:HogTraceParser.ComparisonExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#ComparisonExpr.
    def exitComparisonExpr(self, ctx:HogTraceParser.ComparisonExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#IndexAccess.
    def enterIndexAccess(self, ctx:HogTraceParser.IndexAccessContext):
        pass

    # Exit a parse tree produced by HogTraceParser#IndexAccess.
    def exitIndexAccess(self, ctx:HogTraceParser.IndexAccessContext):
        pass


    # Enter a parse tree produced by HogTraceParser#OrExpr.
    def enterOrExpr(self, ctx:HogTraceParser.OrExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#OrExpr.
    def exitOrExpr(self, ctx:HogTraceParser.OrExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#EqualityExpr.
    def enterEqualityExpr(self, ctx:HogTraceParser.EqualityExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#EqualityExpr.
    def exitEqualityExpr(self, ctx:HogTraceParser.EqualityExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#MulDivModExpr.
    def enterMulDivModExpr(self, ctx:HogTraceParser.MulDivModExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#MulDivModExpr.
    def exitMulDivModExpr(self, ctx:HogTraceParser.MulDivModExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#IdentifierExpr.
    def enterIdentifierExpr(self, ctx:HogTraceParser.IdentifierExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#IdentifierExpr.
    def exitIdentifierExpr(self, ctx:HogTraceParser.IdentifierExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#LiteralExpr.
    def enterLiteralExpr(self, ctx:HogTraceParser.LiteralExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#LiteralExpr.
    def exitLiteralExpr(self, ctx:HogTraceParser.LiteralExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#FunctionCall.
    def enterFunctionCall(self, ctx:HogTraceParser.FunctionCallContext):
        pass

    # Exit a parse tree produced by HogTraceParser#FunctionCall.
    def exitFunctionCall(self, ctx:HogTraceParser.FunctionCallContext):
        pass


    # Enter a parse tree produced by HogTraceParser#NotExpr.
    def enterNotExpr(self, ctx:HogTraceParser.NotExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#NotExpr.
    def exitNotExpr(self, ctx:HogTraceParser.NotExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#RequestVarExpr.
    def enterRequestVarExpr(self, ctx:HogTraceParser.RequestVarExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#RequestVarExpr.
    def exitRequestVarExpr(self, ctx:HogTraceParser.RequestVarExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#ParenExpr.
    def enterParenExpr(self, ctx:HogTraceParser.ParenExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#ParenExpr.
    def exitParenExpr(self, ctx:HogTraceParser.ParenExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#FieldAccess.
    def enterFieldAccess(self, ctx:HogTraceParser.FieldAccessContext):
        pass

    # Exit a parse tree produced by HogTraceParser#FieldAccess.
    def exitFieldAccess(self, ctx:HogTraceParser.FieldAccessContext):
        pass


    # Enter a parse tree produced by HogTraceParser#AddSubExpr.
    def enterAddSubExpr(self, ctx:HogTraceParser.AddSubExprContext):
        pass

    # Exit a parse tree produced by HogTraceParser#AddSubExpr.
    def exitAddSubExpr(self, ctx:HogTraceParser.AddSubExprContext):
        pass


    # Enter a parse tree produced by HogTraceParser#expressionList.
    def enterExpressionList(self, ctx:HogTraceParser.ExpressionListContext):
        pass

    # Exit a parse tree produced by HogTraceParser#expressionList.
    def exitExpressionList(self, ctx:HogTraceParser.ExpressionListContext):
        pass


    # Enter a parse tree produced by HogTraceParser#IntLiteral.
    def enterIntLiteral(self, ctx:HogTraceParser.IntLiteralContext):
        pass

    # Exit a parse tree produced by HogTraceParser#IntLiteral.
    def exitIntLiteral(self, ctx:HogTraceParser.IntLiteralContext):
        pass


    # Enter a parse tree produced by HogTraceParser#FloatLiteral.
    def enterFloatLiteral(self, ctx:HogTraceParser.FloatLiteralContext):
        pass

    # Exit a parse tree produced by HogTraceParser#FloatLiteral.
    def exitFloatLiteral(self, ctx:HogTraceParser.FloatLiteralContext):
        pass


    # Enter a parse tree produced by HogTraceParser#StringLiteral.
    def enterStringLiteral(self, ctx:HogTraceParser.StringLiteralContext):
        pass

    # Exit a parse tree produced by HogTraceParser#StringLiteral.
    def exitStringLiteral(self, ctx:HogTraceParser.StringLiteralContext):
        pass


    # Enter a parse tree produced by HogTraceParser#BoolLiteral.
    def enterBoolLiteral(self, ctx:HogTraceParser.BoolLiteralContext):
        pass

    # Exit a parse tree produced by HogTraceParser#BoolLiteral.
    def exitBoolLiteral(self, ctx:HogTraceParser.BoolLiteralContext):
        pass


    # Enter a parse tree produced by HogTraceParser#NoneLiteral.
    def enterNoneLiteral(self, ctx:HogTraceParser.NoneLiteralContext):
        pass

    # Exit a parse tree produced by HogTraceParser#NoneLiteral.
    def exitNoneLiteral(self, ctx:HogTraceParser.NoneLiteralContext):
        pass



del HogTraceParser