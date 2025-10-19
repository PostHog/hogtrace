"""
HogTrace Virtual Machine.

Executes probes against Python stack frames.
"""

from types import FrameType
from typing import Any, Optional
import random

from hogtrace.ast import (
    Probe, Predicate, Action, CaptureAction, AssignmentAction, SampleAction,
    ActionType
)
from hogtrace.frame import FrameContext
from hogtrace.request_store import RequestLocalStore
from hogtrace.evaluator import ExpressionEvaluator
from hogtrace.errors import EvaluationError


class ProbeExecutor:
    """
    Executes a single probe against a stack frame.

    Example:
        probe = program.probes[0]
        store = RequestLocalStore()
        executor = ProbeExecutor(probe, store)

        frame = inspect.currentframe()
        result = executor.execute(frame)

        if result:
            print(f"Captured: {result}")
    """

    def __init__(self, probe: Probe, request_store: Optional[RequestLocalStore] = None):
        """
        Initialize probe executor.

        Args:
            probe: Probe to execute
            request_store: Request-scoped storage
        """
        self.probe = probe
        self.request_store = request_store or RequestLocalStore()

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

        except Exception as e:
            # Probes should never crash the application
            # Log error but return None
            # TODO: Add logging
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
            evaluator = ExpressionEvaluator(frame_context, self.request_store)
            result = evaluator.evaluate(predicate.expression)
            return bool(result)
        except Exception:
            # If predicate evaluation fails, don't fire probe
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
            evaluator = ExpressionEvaluator(frame_context, self.request_store)
            value = evaluator.evaluate(action.value)
            self.request_store.set(action.variable.name, value)
        except Exception:
            # Silently fail on assignment errors
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
        """
        captured = {}
        evaluator = ExpressionEvaluator(frame_context, self.request_store)

        try:
            # Handle positional arguments
            for i, expr in enumerate(action.arguments):
                try:
                    # Check if it's a special identifier
                    from hogtrace.ast import Identifier
                    if isinstance(expr, Identifier):
                        value = self._resolve_special_capture(expr.name, frame_context)
                        if value is not None:
                            captured[expr.name] = value
                            continue

                    # Regular expression evaluation
                    value = evaluator.evaluate(expr)
                    captured[f'arg{i}'] = value
                except Exception:
                    # Skip failed captures
                    pass

            # Handle named arguments
            for name, expr in action.named_arguments.items():
                try:
                    value = evaluator.evaluate(expr)
                    captured[name] = value
                except Exception:
                    # Skip failed captures
                    pass

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
            return dict(list(globals_dict.items())[:100])
        elif name == 'retval':
            return frame_context.get('retval')
        elif name == 'exception':
            return frame_context.get('exception')
        elif name == 'self':
            return frame_context.get('self')
        else:
            return None


class ProgramExecutor:
    """
    Executes multiple probes from a program.

    Example:
        program = hogtrace.parse(code)
        store = RequestLocalStore()
        executor = ProgramExecutor(program, store)

        frame = inspect.currentframe()
        results = executor.execute_all(frame)

        for probe_name, data in results:
            print(f"{probe_name}: {data}")
    """

    def __init__(self, program, request_store: Optional[RequestLocalStore] = None):
        """
        Initialize program executor.

        Args:
            program: Parsed HogTrace program
            request_store: Request-scoped storage
        """
        self.program = program
        self.request_store = request_store or RequestLocalStore()
        self.executors = [
            ProbeExecutor(probe, self.request_store)
            for probe in program.probes
        ]

    def execute_all(
        self,
        frame: FrameType,
        retval: Any = None,
        exception: Optional[BaseException] = None
    ) -> list[tuple[str, dict]]:
        """
        Execute all probes against a frame.

        Args:
            frame: Python stack frame
            retval: Return value (for exit probes)
            exception: Exception (for exit probes)

        Returns:
            List of (probe_spec, captured_data) tuples
        """
        results = []

        for probe, executor in zip(self.program.probes, self.executors):
            data = executor.execute(frame, retval, exception)
            if data:
                results.append((str(probe.spec), data))

        return results
