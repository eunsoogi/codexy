"""Thread-delivery routing metadata admission."""

from .envelope import Request

FIELDS = ("model", "thinking")


def forbidden(request: Request) -> bool:
    data = request.tool_input
    return not isinstance(data, dict) or any(
        not isinstance(data.get(field), str) or not data[field].strip()
        for field in FIELDS
    )
