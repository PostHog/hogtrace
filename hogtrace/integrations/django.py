from typing import TYPE_CHECKING, Callable
from hogtrace import context


if TYPE_CHECKING:
    from django.http import HttpRequest, HttpResponse


class HogTraceContextMiddleware:
    """
    Middleware to automatically track Django requests.
    """

    sync_capable = True
    async_capable = False

    def __init__(self, get_response: Callable[[HttpRequest], HttpResponse]):
        self.get_response = get_response

    def __call__(self, request: HttpRequest) -> HttpResponse:
        with context.new_context():
            return self.get_response(request)
