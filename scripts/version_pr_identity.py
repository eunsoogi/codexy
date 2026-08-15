"""Typed governing identity for version pull-request reconciliation."""

from __future__ import annotations

from dataclasses import dataclass
import re

VERSION_PATTERN = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
OWNER_PATTERN = re.compile(r"(?=[A-Za-z0-9-]{1,39}$)[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*")
REPOSITORY_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,99}")
ISSUE_URL_PATTERN = re.compile(
    r"https://github\.com/(?P<owner>[A-Za-z0-9-]+)/"
    r"(?P<repository>[A-Za-z0-9._-]+)/issues/(?P<number>[1-9][0-9]*)"
)
CLOSING_KEYWORD = r"(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)"
CLOSING_CANDIDATE_PATTERN = re.compile(
    rf"(?i)\b(?P<keyword>{CLOSING_KEYWORD})\b"
    rf"(?P<separator>\s*:?\s*)"
    rf"(?P<reference>#[^\s,;]+|[^\s,;]+#[^\s,;]+)"
)


def require_object(value: object, context: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be an object")
    return value


def parse_repository(value: str) -> tuple[str, str]:
    parts = value.split("/")
    if (
        len(parts) != 2
        or OWNER_PATTERN.fullmatch(parts[0]) is None
        or REPOSITORY_PATTERN.fullmatch(parts[1]) is None
    ):
        raise ValueError("repository must use canonical OWNER/NAME form")
    return parts[0], parts[1]


@dataclass(frozen=True)
class CanonicalIssueIdentity:
    owner: str
    repository: str
    number: int
    url: str

    @classmethod
    def parse(
        cls,
        url: object,
        expected_number: int,
        context: str,
    ) -> "CanonicalIssueIdentity":
        if not isinstance(url, str):
            raise ValueError(f"{context} requires a canonical issue URL")
        match = ISSUE_URL_PATTERN.fullmatch(url)
        if (
            match is None
            or OWNER_PATTERN.fullmatch(match["owner"]) is None
            or REPOSITORY_PATTERN.fullmatch(match["repository"]) is None
            or int(match["number"]) != expected_number
        ):
            raise ValueError(f"{context} requires a canonical issue URL")
        return cls(match["owner"], match["repository"], expected_number, url)

    @classmethod
    def from_issue(cls, value: object) -> "CanonicalIssueIdentity":
        issue = require_object(value, "requested issue")
        number = issue.get("number")
        if not isinstance(number, int) or isinstance(number, bool) or number < 1:
            raise ValueError("requested issue number must be a positive integer")
        return cls.parse(issue.get("url"), number, f"requested issue #{number}")

    def require_repository(self, repository: str, context: str) -> None:
        if f"{self.owner}/{self.repository}" != repository:
            raise ValueError(f"{context} must belong to {repository}")


def canonicalize_requested_issue(
    requested: str, issue: object, repository: str
) -> CanonicalIssueIdentity:
    if not requested or not requested.isascii() or not requested.isdigit():
        raise ValueError("requested issue must be a positive integer")
    requested_number = int(requested)
    if requested_number < 1:
        raise ValueError("requested issue must be a positive integer")
    identity = CanonicalIssueIdentity.from_issue(issue)
    identity.require_repository(repository, "requested issue")
    if identity.number != requested_number:
        raise ValueError("requested issue does not match the fetched issue")
    return identity


def parse_body_closing_references(
    body: str, repository: str
) -> tuple[CanonicalIssueIdentity, ...]:
    default_owner, default_repository = parse_repository(repository)
    references: list[CanonicalIssueIdentity] = []
    for match in CLOSING_CANDIDATE_PATTERN.finditer(body):
        separator = match["separator"]
        if re.fullmatch(r"(?:\s+|:\s+)", separator) is None:
            raise ValueError("observed PR body contains a malformed closing reference")
        token = match["reference"].rstrip(".,")
        if token.startswith("#"):
            owner, name, number_text = default_owner, default_repository, token[1:]
        else:
            owner_repository, marker, number_text = token.rpartition("#")
            if not marker:
                raise ValueError(
                    "observed PR body contains a malformed closing reference"
                )
            try:
                owner, name = parse_repository(owner_repository)
            except ValueError as error:
                raise ValueError(
                    "observed PR body contains a malformed closing reference"
                ) from error
        if (
            not number_text
            or not number_text.isascii()
            or not number_text.isdigit()
            or number_text.startswith("0")
        ):
            raise ValueError("observed PR body contains a malformed closing reference")
        number = int(number_text)
        url = f"https://github.com/{owner}/{name}/issues/{number}"
        references.append(CanonicalIssueIdentity(owner, name, number, url))
    return tuple(references)


from version_pr_observed import ObservedVersionPrIdentity


def authorize_governing_identity(
    action: str,
    version: str,
    repository: str,
    requested_issue: object,
    observed_pr: object | None,
    issue_link_mode: str = "closing",
) -> None:
    if VERSION_PATTERN.fullmatch(version) is None:
        raise ValueError("version must use MAJOR.MINOR.PATCH form")
    parse_repository(repository)
    requested = CanonicalIssueIdentity.from_issue(requested_issue)
    requested.require_repository(repository, "requested issue")
    if action in ("first-run", "pushed-no-pr"):
        if observed_pr is not None:
            raise ValueError("new PR transition must not include observed PR identity")
        return
    if action != "existing-pr-update":
        raise ValueError(f"unsupported governing-identity transition: {action}")
    if observed_pr is None:
        raise ValueError("existing PR update requires observed governing identity")
    observed = ObservedVersionPrIdentity.from_pr(
        observed_pr, repository, issue_link_mode
    )
    if observed.branch != f"codexy/version-{version}":
        raise ValueError("observed PR version branch does not match requested version")
    if observed.issue != requested:
        raise ValueError("observed governing issue does not match requested issue")
