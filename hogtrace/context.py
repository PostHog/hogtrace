import uuid
from contextlib import contextmanager
import contextvars
from typing import Final, Optional

from hogtrace.request_store import RequestLocalStore


class ContextScope:
    context_id: Final[str]
    store: Final[RequestLocalStore]

    def __init__(self):
        self.context_id = uuid.uuid4().hex
        self.store = RequestLocalStore()


_context_stack: contextvars.ContextVar[Optional[ContextScope]] = contextvars.ContextVar(
    "__posthog_libdebugger_context_stack", default=None
)


def get_scope() -> Optional[ContextScope]:
    return _context_stack.get()


def get_store() -> Optional[RequestLocalStore]:
    scope = get_scope()

    if scope:
        return scope.store
    else:
        return None


@contextmanager
def new_context():
    tok = _context_stack.set(ContextScope())

    try:
        yield
    finally:
        _context_stack.reset(tok)
