"""
HogTrace Abstract Syntax Tree (AST) data structures.

These classes represent the parsed structure of HogTrace programs
in a clean, easy-to-use format.
"""

from dataclasses import dataclass, field
from typing import Optional, Union
from enum import Enum


# ===== Expressions =====

class ExpressionType(Enum):
    """Types of expressions in HogTrace"""
    LITERAL = "literal"
    IDENTIFIER = "identifier"
    FIELD_ACCESS = "field_access"
    INDEX_ACCESS = "index_access"
    FUNCTION_CALL = "function_call"
    REQUEST_VAR = "request_var"
    BINARY_OP = "binary_op"
    UNARY_OP = "unary_op"


@dataclass
class Expression:
    """Base class for all expressions"""
    type: ExpressionType
    raw: str  # Raw text representation

    def __str__(self):
        return self.raw


@dataclass
class Literal(Expression):
    """Literal value (int, float, string, bool, None)"""
    value: Union[int, float, str, bool, None]

    def __init__(self, value: Union[int, float, str, bool, None], raw: str):
        super().__init__(ExpressionType.LITERAL, raw)
        self.value = value


@dataclass
class Identifier(Expression):
    """Variable identifier (args, arg0, retval, etc.)"""
    name: str

    def __init__(self, name: str):
        super().__init__(ExpressionType.IDENTIFIER, name)
        self.name = name


@dataclass
class FieldAccess(Expression):
    """Object field access (obj.field)"""
    object: Expression
    field: str

    def __init__(self, object: Expression, field: str, raw: str):
        super().__init__(ExpressionType.FIELD_ACCESS, raw)
        self.object = object
        self.field = field


@dataclass
class IndexAccess(Expression):
    """Array/dict index access (obj[index])"""
    object: Expression
    index: Expression

    def __init__(self, object: Expression, index: Expression, raw: str):
        super().__init__(ExpressionType.INDEX_ACCESS, raw)
        self.object = object
        self.index = index


@dataclass
class FunctionCall(Expression):
    """Function call (timestamp(), len(args))"""
    function: str
    arguments: list[Expression]

    def __init__(self, function: str, arguments: list[Expression], raw: str):
        super().__init__(ExpressionType.FUNCTION_CALL, raw)
        self.function = function
        self.arguments = arguments


@dataclass
class RequestVar(Expression):
    """Request-scoped variable ($req.var or $request.var)"""
    name: str
    prefix: str  # "req" or "request"

    def __init__(self, name: str, prefix: str, raw: str):
        super().__init__(ExpressionType.REQUEST_VAR, raw)
        self.name = name
        self.prefix = prefix


@dataclass
class BinaryOp(Expression):
    """Binary operation (a + b, a == b, etc.)"""
    operator: str
    left: Expression
    right: Expression

    def __init__(self, operator: str, left: Expression, right: Expression, raw: str):
        super().__init__(ExpressionType.BINARY_OP, raw)
        self.operator = operator
        self.left = left
        self.right = right


@dataclass
class UnaryOp(Expression):
    """Unary operation (!expr)"""
    operator: str
    operand: Expression

    def __init__(self, operator: str, operand: Expression, raw: str):
        super().__init__(ExpressionType.UNARY_OP, raw)
        self.operator = operator
        self.operand = operand


# ===== Actions =====

class ActionType(Enum):
    """Types of actions in HogTrace"""
    CAPTURE = "capture"
    ASSIGNMENT = "assignment"
    SAMPLE = "sample"


@dataclass
class Action:
    """Base class for all actions"""
    type: ActionType


@dataclass
class CaptureAction(Action):
    """Capture/send action"""
    function: str  # "capture" or "send"
    arguments: list[Expression]
    named_arguments: dict[str, Expression]

    def __init__(self, function: str, arguments: list[Expression] = None,
                 named_arguments: dict[str, Expression] = None):
        super().__init__(ActionType.CAPTURE)
        self.function = function
        self.arguments = arguments or []
        self.named_arguments = named_arguments or {}

    def __str__(self):
        args = []
        args.extend(str(arg) for arg in self.arguments)
        args.extend(f"{name}={expr}" for name, expr in self.named_arguments.items())
        return f"{self.function}({', '.join(args)})"


@dataclass
class AssignmentAction(Action):
    """Assignment to request-scoped variable"""
    variable: RequestVar
    value: Expression

    def __init__(self, variable: RequestVar, value: Expression):
        super().__init__(ActionType.ASSIGNMENT)
        self.variable = variable
        self.value = value

    def __str__(self):
        return f"{self.variable} = {self.value}"


@dataclass
class SampleAction(Action):
    """Sampling directive"""
    spec: str  # "10%", "1/100", etc.
    is_percentage: bool
    value: Optional[float] = None  # Parsed percentage (0.0-1.0)
    numerator: Optional[int] = None  # For ratio sampling
    denominator: Optional[int] = None

    def __init__(self, spec: str, is_percentage: bool, value: float = None,
                 numerator: int = None, denominator: int = None):
        super().__init__(ActionType.SAMPLE)
        self.spec = spec
        self.is_percentage = is_percentage
        self.value = value
        self.numerator = numerator
        self.denominator = denominator

    def __str__(self):
        return f"sample {self.spec}"


# ===== Probe Components =====

@dataclass
class ProbeSpec:
    """Probe specification (provider:module.function:probe_point)"""
    provider: str  # "fn", "py", etc.
    module_function: str  # "myapp.users.create_user"
    probe_point: str  # "entry", "exit", "entry+10", etc.
    full_spec: str  # Complete specification string

    def __str__(self):
        return self.full_spec


@dataclass
class Predicate:
    """Predicate (guard condition)"""
    expression: Expression

    def __str__(self):
        return f"/ {self.expression} /"


@dataclass
class Probe:
    """A single probe definition"""
    spec: ProbeSpec
    predicate: Optional[Predicate]
    actions: list[Action]

    def __init__(self, spec: ProbeSpec, predicate: Optional[Predicate] = None,
                 actions: list[Action] = None):
        self.spec = spec
        self.predicate = predicate
        self.actions = actions or []

    def __str__(self):
        result = [str(self.spec)]
        if self.predicate:
            result.append(str(self.predicate))
        result.append("{")
        for action in self.actions:
            result.append(f"    {action};")
        result.append("}")
        return "\n".join(result)


# ===== Program =====

@dataclass
class Program:
    """A complete HogTrace program (collection of probes)"""
    probes: list[Probe]

    def __init__(self, probes: Optional[list[Probe]] = None):
        self.probes = probes or []

    def __str__(self):
        return "\n\n".join(str(probe) for probe in self.probes)

    def __len__(self):
        return len(self.probes)

    def __iter__(self):
        return iter(self.probes)

    def __getitem__(self, index):
        return self.probes[index]
