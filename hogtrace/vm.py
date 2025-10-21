"""
HogTrace Virtual Machine.

Executes probes against Python stack frames.
"""

from types import FrameType
from typing import Any, Optional
import random
import sys

from hogtrace.ast import (
    Probe, Predicate, Action, CaptureAction, AssignmentAction, SampleAction,
    ActionType
)
from hogtrace.frame import FrameContext
from hogtrace.request_store import RequestLocalStore
from hogtrace.evaluator import ExpressionEvaluator, evaluate_expression
from hogtrace.errors import (
    EvaluationError, TimeoutError, CaptureSizeError, RecursionError
)
from hogtrace.limits import HogTraceLimits, DEFAULT_LIMITS
from hogtrace import logger


class ProbeExecutor:
    """
    Executes a probe code against the stack.

    Example:
        probe = program.probes[0]
        store = RequestLocalStore()
        executor = ProbeExecutor(probe, store)

        frame = inspect.currentframe()
        result = executor.execute(frame)

        if result:
            print(f"Captured: {result}")
    """

    def __init__(
        self,
        probe: Probe,
        request_store: Optional[RequestLocalStore] = None,
        limits: Optional[HogTraceLimits] = None
    ):
        """
        Initialize probe executor.

        Args:
            probe: Probe to execute
            request_store: Request-scoped storage
            limits: Resource limits (uses DEFAULT_LIMITS if not provided)
        """
        self.probe = probe
        self.request_store = request_store or RequestLocalStore()
        self.limits = limits or DEFAULT_LIMITS

    def execute(
        self,
        frame: FrameType,
        retval: Any = None,
        exception: Optional[BaseException] = None
    ) -> Optional[dict]:
        """
        Execute the probe against a stack frame.

        Args:
            frame: Python stack frame
            retval: Return value (for exit probes)
            exception: Exception (for exit probes)

        Returns:
            Dict of captured data if probe fires, None otherwise
        """
        try:
            # Build frame context
            frame_context = FrameContext(frame, retval, exception)

            # Check predicate (if exists)
            if self.probe.predicate:
                if not self._evaluate_predicate(self.probe.predicate, frame_context):
                    return None  # Predicate failed, don't fire probe

            # Execute actions
            captured_data = {}
            should_capture = True

            for action in self.probe.actions:
                if action.type == ActionType.SAMPLE:
                    # Check sampling
                    if not self._check_sample(action):
                        should_capture = False
                        break

                elif action.type == ActionType.ASSIGNMENT:
                    # Execute assignment
                    self._execute_assignment(action, frame_context)

                elif action.type == ActionType.CAPTURE:
                    # Collect capture data
                    data = self._execute_capture(action, frame_context)
                    captured_data.update(data)

            # Return captured data if probe fired
            return captured_data if (should_capture and captured_data) else None

        except TimeoutError as e:
            # Probe timed out
            logger.log_timeout(str(self.probe.spec), self.limits.max_predicate_time_ms)
            return None
        except Exception as e:
            # Probes should never crash the application
            # Log error but return None
            logger.log_probe_failed(str(self.probe.spec), e)
            return None

    def _evaluate_predicate(
        self,
        predicate: Predicate,
        frame_context: FrameContext
    ) -> bool:
        """
        Evaluate a predicate expression.

        Args:
            predicate: Predicate to evaluate
            frame_context: Frame context

        Returns:
            True if predicate passes, False otherwise
        """
        try:
            # Use evaluate_expression with timeout support
            result = evaluate_expression(
                predicate.expression,
                frame_context,
                self.request_store,
                self.limits
            )
            passed = bool(result)

            if not passed and self.limits.log_predicate_failures:
                logger.log_predicate_failed(str(self.probe.spec), str(predicate.expression))

            return passed
        except TimeoutError as e:
            logger.log_predicate_error(str(self.probe.spec), e)
            return False
        except Exception as e:
            # If predicate evaluation fails, don't fire probe
            logger.log_predicate_error(str(self.probe.spec), e)
            return False

    def _check_sample(self, action: SampleAction) -> bool:
        """
        Check if probe should fire based on sampling.

        Args:
            action: Sample action

        Returns:
            True if probe should fire
        """
        if action.value is None:
            return True

        # Use random sampling
        return random.random() < action.value

    def _execute_assignment(
        self,
        action: AssignmentAction,
        frame_context: FrameContext
    ) -> None:
        """
        Execute an assignment action.

        Args:
            action: Assignment action
            frame_context: Frame context
        """
        try:
            evaluator = ExpressionEvaluator(frame_context, self.request_store, self.limits)
            value = evaluator.evaluate(action.value)
            self.request_store.set(action.variable.name, value)
        except Exception as e:
            # Silently fail on assignment errors, but log if enabled
            if self.limits.log_probe_execution:
                logger.log_capture_error(str(self.probe.spec), action.variable.name, e)
            pass

    def _execute_capture(
        self,
        action: CaptureAction,
        frame_context: FrameContext
    ) -> dict:
        """
        Execute a capture action.

        Args:
            action: Capture action
            frame_context: Frame context

        Returns:
            Dict of captured data

        Raises:
            CaptureSizeError: If captured data exceeds size limit
        """
        captured = {}
        evaluator = ExpressionEvaluator(frame_context, self.request_store, self.limits)

        try:
            # Handle positional arguments
            for i, expr in enumerate(action.arguments):
                field_name = f'arg{i}'
                try:
                    # Check if it's a special identifier
                    from hogtrace.ast import Identifier
                    if isinstance(expr, Identifier):
                        value = self._resolve_special_capture(expr.name, frame_context)
                        if value is not None:
                            field_name = expr.name
                            # Limit the value size
                            value = self._limit_capture_value(value, field_name)
                            captured[field_name] = value
                            continue

                    # Regular expression evaluation
                    value = evaluator.evaluate(expr)
                    value = self._limit_capture_value(value, field_name)
                    captured[field_name] = value
                except Exception as e:
                    # Skip failed captures but log if enabled
                    if self.limits.log_probe_execution:
                        logger.log_capture_error(str(self.probe.spec), field_name, e)
                    pass

            # Handle named arguments
            for name, expr in action.named_arguments.items():
                try:
                    value = evaluator.evaluate(expr)
                    value = self._limit_capture_value(value, name)
                    captured[name] = value
                except Exception as e:
                    # Skip failed captures but log if enabled
                    if self.limits.log_probe_execution:
                        logger.log_capture_error(str(self.probe.spec), name, e)
                    pass

            # Check total capture size
            self._check_capture_size(captured)

        except CaptureSizeError:
            # Re-raise size errors
            raise
        except Exception:
            # Return whatever we captured so far
            pass

        return captured

    def _resolve_special_capture(
        self,
        name: str,
        frame_context: FrameContext
    ) -> Optional[Any]:
        """
        Resolve special capture names like 'args', 'locals', 'globals'.

        Args:
            name: Special name
            frame_context: Frame context

        Returns:
            Value or None if not a special name
        """
        if name == 'args':
            return frame_context.get('args')
        elif name == 'kwargs':
            return frame_context.get('kwargs')
        elif name == 'locals':
            return frame_context.get('locals')
        elif name == 'globals':
            # Return a copy to avoid capturing too much
            globals_dict = frame_context.get('globals', {})
            # Limit size of globals to prevent huge captures
            max_items = min(self.limits.max_capture_items, 100)
            return dict(list(globals_dict.items())[:max_items])
        elif name == 'retval':
            return frame_context.get('retval')
        elif name == 'exception':
            return frame_context.get('exception')
        elif name == 'self':
            return frame_context.get('self')
        else:
            return None

    def _limit_capture_value(self, value: Any, field_name: str, depth: int = 0) -> Any:
        """
        Limit the size and depth of captured values.

        Args:
            value: Value to limit
            field_name: Name of the field being captured (for logging)
            depth: Current recursion depth

        Returns:
            Limited value

        Raises:
            RecursionError: If depth limit exceeded
        """
        # Check depth limit
        if depth > self.limits.max_capture_depth:
            return f"<max depth {self.limits.max_capture_depth} exceeded>"

        # Handle None, bool, numbers, strings
        if value is None or isinstance(value, (bool, int, float)):
            return value

        if isinstance(value, str):
            # Limit string length
            max_len = 1000
            if len(value) > max_len:
                return value[:max_len] + f"... ({len(value)} chars total)"
            return value

        # Handle lists/tuples
        if isinstance(value, (list, tuple)):
            limited_items = []
            for i, item in enumerate(value):
                if i >= self.limits.max_capture_items:
                    limited_items.append(f"... ({len(value)} items total)")
                    break
                limited_items.append(self._limit_capture_value(item, f"{field_name}[{i}]", depth + 1))
            return limited_items

        # Handle dicts
        if isinstance(value, dict):
            limited_dict = {}
            for i, (k, v) in enumerate(value.items()):
                if i >= self.limits.max_capture_items:
                    limited_dict["..."] = f"({len(value)} keys total)"
                    break
                try:
                    limited_dict[k] = self._limit_capture_value(v, f"{field_name}.{k}", depth + 1)
                except Exception:
                    limited_dict[k] = f"<error capturing {k}>"
            return limited_dict

        # For other objects, try to get a safe representation
        try:
            # Try __dict__ for custom objects
            if hasattr(value, '__dict__') and not isinstance(value, type):
                obj_dict = {}
                for i, (k, v) in enumerate(value.__dict__.items()):
                    if i >= self.limits.max_capture_items:
                        obj_dict["..."] = f"({len(value.__dict__)} attrs total)"
                        break
                    if not k.startswith('_'):  # Skip private attributes
                        obj_dict[k] = self._limit_capture_value(v, f"{field_name}.{k}", depth + 1)
                return obj_dict
            else:
                # Fall back to repr
                repr_str = repr(value)
                if len(repr_str) > 200:
                    repr_str = repr_str[:200] + "..."
                return repr_str
        except Exception:
            return f"<{type(value).__name__}>"

    def _check_capture_size(self, captured: dict) -> None:
        """
        Check if captured data exceeds size limit.

        Args:
            captured: Captured data dictionary

        Raises:
            CaptureSizeError: If size limit exceeded
        """
        try:
            # Estimate size using str representation
            # This is not perfect but gives a rough estimate
            size_estimate = len(str(captured))

            if size_estimate > self.limits.max_capture_size_bytes:
                logger.log_capture_size_exceeded(
                    str(self.probe.spec),
                    size_estimate,
                    self.limits.max_capture_size_bytes
                )
                raise CaptureSizeError(
                    f"Captured data size ({size_estimate} bytes) "
                    f"exceeds limit ({self.limits.max_capture_size_bytes} bytes)"
                )
        except CaptureSizeError:
            raise
        except Exception:
            # If we can't estimate size, allow it
            pass

