"""
JSON serialization/deserialization for HogTrace AST.

This module allows converting parsed HogTrace programs to/from JSON format
for storage and transmission between services.
"""

import json
from typing import Any, Dict, List, Optional, Union

from hogtrace.ast import (
    Program, Probe, ProbeSpec, Predicate, Action,
    CaptureAction, AssignmentAction, SampleAction, ActionType,
    Expression, Literal, Identifier, FieldAccess, IndexAccess,
    FunctionCall, RequestVar, BinaryOp, UnaryOp, ExpressionType
)


# ===== Expression Serialization =====

def serialize_expression(expr: Expression) -> Dict[str, Any]:
    """
    Convert an Expression to a JSON-serializable dict.

    Args:
        expr: Expression AST node

    Returns:
        Dict representation of the expression
    """
    if isinstance(expr, Literal):
        return {
            "type": "literal",
            "value": expr.value,
            "raw": expr.raw
        }

    elif isinstance(expr, Identifier):
        return {
            "type": "identifier",
            "name": expr.name
        }

    elif isinstance(expr, FieldAccess):
        return {
            "type": "field_access",
            "object": serialize_expression(expr.object),
            "field": expr.field,
            "raw": expr.raw
        }

    elif isinstance(expr, IndexAccess):
        return {
            "type": "index_access",
            "object": serialize_expression(expr.object),
            "index": serialize_expression(expr.index),
            "raw": expr.raw
        }

    elif isinstance(expr, FunctionCall):
        return {
            "type": "function_call",
            "function": expr.function,
            "arguments": [serialize_expression(arg) for arg in expr.arguments],
            "raw": expr.raw
        }

    elif isinstance(expr, RequestVar):
        return {
            "type": "request_var",
            "name": expr.name,
            "prefix": expr.prefix,
            "raw": expr.raw
        }

    elif isinstance(expr, BinaryOp):
        return {
            "type": "binary_op",
            "operator": expr.operator,
            "left": serialize_expression(expr.left),
            "right": serialize_expression(expr.right),
            "raw": expr.raw
        }

    elif isinstance(expr, UnaryOp):
        return {
            "type": "unary_op",
            "operator": expr.operator,
            "operand": serialize_expression(expr.operand),
            "raw": expr.raw
        }

    else:
        raise ValueError(f"Unknown expression type: {type(expr)}")


def deserialize_expression(data: Dict[str, Any]) -> Expression:
    """
    Convert a dict back to an Expression AST node.

    Args:
        data: Dict representation from serialize_expression

    Returns:
        Expression AST node
    """
    expr_type = data["type"]

    if expr_type == "literal":
        return Literal(data["value"], data["raw"])

    elif expr_type == "identifier":
        return Identifier(data["name"])

    elif expr_type == "field_access":
        return FieldAccess(
            deserialize_expression(data["object"]),
            data["field"],
            data["raw"]
        )

    elif expr_type == "index_access":
        return IndexAccess(
            deserialize_expression(data["object"]),
            deserialize_expression(data["index"]),
            data["raw"]
        )

    elif expr_type == "function_call":
        return FunctionCall(
            data["function"],
            [deserialize_expression(arg) for arg in data["arguments"]],
            data["raw"]
        )

    elif expr_type == "request_var":
        return RequestVar(
            data["name"],
            data["prefix"],
            data["raw"]
        )

    elif expr_type == "binary_op":
        return BinaryOp(
            data["operator"],
            deserialize_expression(data["left"]),
            deserialize_expression(data["right"]),
            data["raw"]
        )

    elif expr_type == "unary_op":
        return UnaryOp(
            data["operator"],
            deserialize_expression(data["operand"]),
            data["raw"]
        )

    else:
        raise ValueError(f"Unknown expression type: {expr_type}")


# ===== Action Serialization =====

def serialize_action(action: Action) -> Dict[str, Any]:
    """
    Convert an Action to a JSON-serializable dict.

    Args:
        action: Action AST node

    Returns:
        Dict representation of the action
    """
    if isinstance(action, CaptureAction):
        return {
            "type": "capture",
            "function": action.function,
            "arguments": [serialize_expression(arg) for arg in action.arguments],
            "named_arguments": {
                name: serialize_expression(expr)
                for name, expr in action.named_arguments.items()
            }
        }

    elif isinstance(action, AssignmentAction):
        return {
            "type": "assignment",
            "variable": serialize_expression(action.variable),
            "value": serialize_expression(action.value)
        }

    elif isinstance(action, SampleAction):
        result = {
            "type": "sample",
            "spec": action.spec,
            "is_percentage": action.is_percentage
        }
        if action.value is not None:
            result["value"] = action.value
        if action.numerator is not None:
            result["numerator"] = action.numerator
        if action.denominator is not None:
            result["denominator"] = action.denominator
        return result

    else:
        raise ValueError(f"Unknown action type: {type(action)}")


def deserialize_action(data: Dict[str, Any]) -> Action:
    """
    Convert a dict back to an Action AST node.

    Args:
        data: Dict representation from serialize_action

    Returns:
        Action AST node
    """
    action_type = data["type"]

    if action_type == "capture":
        return CaptureAction(
            data["function"],
            [deserialize_expression(arg) for arg in data["arguments"]],
            {
                name: deserialize_expression(expr)
                for name, expr in data["named_arguments"].items()
            }
        )

    elif action_type == "assignment":
        return AssignmentAction(
            deserialize_expression(data["variable"]),
            deserialize_expression(data["value"])
        )

    elif action_type == "sample":
        return SampleAction(
            data["spec"],
            data["is_percentage"],
            data.get("value"),
            data.get("numerator"),
            data.get("denominator")
        )

    else:
        raise ValueError(f"Unknown action type: {action_type}")


# ===== Probe Serialization =====

def serialize_probe_spec(spec: ProbeSpec) -> Dict[str, Any]:
    """
    Convert a ProbeSpec to a JSON-serializable dict.

    Args:
        spec: ProbeSpec AST node

    Returns:
        Dict representation of the probe spec
    """
    return {
        "provider": spec.provider,
        "module_function": spec.module_function,
        "probe_point": spec.probe_point,
        "full_spec": spec.full_spec
    }


def deserialize_probe_spec(data: Dict[str, Any]) -> ProbeSpec:
    """
    Convert a dict back to a ProbeSpec AST node.

    Args:
        data: Dict representation from serialize_probe_spec

    Returns:
        ProbeSpec AST node
    """
    return ProbeSpec(
        data["provider"],
        data["module_function"],
        data["probe_point"],
        data["full_spec"]
    )


def serialize_predicate(predicate: Optional[Predicate]) -> Optional[Dict[str, Any]]:
    """
    Convert a Predicate to a JSON-serializable dict.

    Args:
        predicate: Predicate AST node or None

    Returns:
        Dict representation or None
    """
    if predicate is None:
        return None

    return {
        "expression": serialize_expression(predicate.expression)
    }


def deserialize_predicate(data: Optional[Dict[str, Any]]) -> Optional[Predicate]:
    """
    Convert a dict back to a Predicate AST node.

    Args:
        data: Dict representation from serialize_predicate or None

    Returns:
        Predicate AST node or None
    """
    if data is None:
        return None

    return Predicate(deserialize_expression(data["expression"]))


def serialize_probe(probe: Probe) -> Dict[str, Any]:
    """
    Convert a Probe to a JSON-serializable dict.

    Args:
        probe: Probe AST node

    Returns:
        Dict representation of the probe
    """
    return {
        "spec": serialize_probe_spec(probe.spec),
        "predicate": serialize_predicate(probe.predicate),
        "actions": [serialize_action(action) for action in probe.actions]
    }


def deserialize_probe(data: Dict[str, Any]) -> Probe:
    """
    Convert a dict back to a Probe AST node.

    Args:
        data: Dict representation from serialize_probe

    Returns:
        Probe AST node
    """
    return Probe(
        deserialize_probe_spec(data["spec"]),
        deserialize_predicate(data["predicate"]),
        [deserialize_action(action) for action in data["actions"]]
    )


# ===== Program Serialization =====

def serialize_program(program: Program) -> Dict[str, Any]:
    """
    Convert a Program to a JSON-serializable dict.

    Args:
        program: Program AST node

    Returns:
        Dict representation of the program

    Example:
        >>> program = hogtrace.parse(code)
        >>> data = serialize_program(program)
        >>> json_str = json.dumps(data)
    """
    return {
        "version": "0.1.0",
        "probes": [serialize_probe(probe) for probe in program.probes]
    }


def deserialize_program(data: Dict[str, Any]) -> Program:
    """
    Convert a dict back to a Program AST node.

    Args:
        data: Dict representation from serialize_program

    Returns:
        Program AST node

    Example:
        >>> data = json.loads(json_str)
        >>> program = deserialize_program(data)
        >>> executor = ProgramExecutor(program, store)
    """
    # Check version for future compatibility
    version = data.get("version", "0.1.0")
    if version != "0.1.0":
        raise ValueError(f"Unsupported program version: {version}")

    return Program([deserialize_probe(probe) for probe in data["probes"]])


# ===== Convenience Functions =====

def program_to_json(program: Program, indent: Optional[int] = 2) -> str:
    """
    Convert a Program to a JSON string.

    Args:
        program: Program AST node
        indent: JSON indentation (None for compact, 2 for pretty)

    Returns:
        JSON string representation

    Example:
        >>> program = hogtrace.parse(code)
        >>> json_str = program_to_json(program)
        >>> # Store in database
        >>> db.save_probe_definition(session_id, json_str)
    """
    return json.dumps(serialize_program(program), indent=indent)


def program_from_json(json_str: str) -> Program:
    """
    Convert a JSON string back to a Program.

    Args:
        json_str: JSON string from program_to_json

    Returns:
        Program AST node

    Example:
        >>> json_str = db.fetch_probe_definition(session_id)
        >>> program = program_from_json(json_str)
        >>> executor = ProgramExecutor(program, store)
    """
    data = json.loads(json_str)
    return deserialize_program(data)
