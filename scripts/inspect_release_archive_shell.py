"""Shell lexical helpers for release archive wrapper inspection."""

import re


HEREDOC_PATTERN = re.compile(
    r"<<(?P<strip>-)?[ \t]*(?P<quote>['\"]?)(?P<delimiter>[^ \t;|&<>()]+)(?P=quote)(?=$|[ \t;|&<>()])"
)
SHELL_LITERAL_PATTERN = re.compile(r"'(?:[^']*)'|\"(?:\\.|[^\"])*\"|\\.")


def shell_scan(line: str) -> tuple[bool, list[tuple[str, bool]]]:
    masked = SHELL_LITERAL_PATTERN.sub(lambda match: "_" * len(match[0]), line)
    if comment := re.search(r"(?<!\S)#", masked):
        masked = masked[: comment.start()]
    delimiters = []
    for match in re.finditer(r"<<", masked):
        heredoc = HEREDOC_PATTERN.match(line, match.start())
        if not heredoc:
            raise ValueError("invalid heredoc")
        delimiters.append((heredoc["delimiter"], bool(heredoc["strip"])))
    return bool(re.search(r"(?<!\\)(?:\\\\)*\\$", masked)), delimiters
