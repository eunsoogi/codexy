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
CLOSING_PATTERN = re.compile(r"Fixes #(?P<number>[1-9][0-9]*)")


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


@dataclass(frozen=True)
class ObservedVersionPrIdentity:
    branch: str
    issue: CanonicalIssueIdentity
    labels: tuple[str, ...]
    body: str

    @classmethod
    def from_pr(cls, value: object, repository: str) -> "ObservedVersionPrIdentity":
        pr = require_object(value, "observed PR")
        branch = pr.get("headRefName")
        if not isinstance(branch, str) or not branch or branch != branch.strip():
            raise ValueError("observed PR requires a canonical version branch")
        references = pr.get("closingIssuesReferences")
        if not isinstance(references, list) or len(references) != 1:
            raise ValueError(
                "existing PR must have exactly one canonical closing issue reference"
            )
        reference = require_object(references[0], "observed closing issue reference")
        number = reference.get("number")
        if not isinstance(number, int) or isinstance(number, bool) or number < 1:
            raise ValueError("observed closing issue number must be a positive integer")
        issue = CanonicalIssueIdentity.parse(
            reference.get("url"), number, "observed closing issue reference"
        )
        issue.require_repository(repository, "observed closing issue reference")
        cls._validate_reference_repository(reference, repository)
        body = pr.get("body")
        if not isinstance(body, str):
            raise ValueError("observed PR requires a body")
        nonempty_lines = [line for line in body.splitlines() if line]
        closing_lines = [
            line for line in nonempty_lines if CLOSING_PATTERN.fullmatch(line) is not None
        ]
        expected_line = f"Fixes #{number}"
        if len(closing_lines) != 1 or not nonempty_lines or nonempty_lines[-1] != expected_line:
            raise ValueError(
                "observed PR body must end with exactly one canonical closing issue reference"
            )
        labels = cls._labels(pr.get("labels"))
        return cls(branch, issue, labels, body)

    @staticmethod
    def _validate_reference_repository(reference: dict[str, object], repository: str) -> None:
        expected_owner, expected_name = parse_repository(repository)
        observed = require_object(reference.get("repository"), "closing issue repository")
        owner = require_object(observed.get("owner"), "closing issue repository owner")
        if observed.get("name") != expected_name or owner.get("login") != expected_owner:
            raise ValueError(f"observed closing issue reference must belong to {repository}")

    @staticmethod
    def _labels(value: object) -> tuple[str, ...]:
        if not isinstance(value, list):
            raise ValueError("observed PR labels must be an array")
        names: list[str] = []
        for item in value:
            label = require_object(item, "observed PR label")
            name = label.get("name")
            if not isinstance(name, str) or not name.strip():
                raise ValueError("observed PR labels require non-empty names")
            names.append(name)
        if len(names) != len(set(names)):
            raise ValueError("observed PR labels contain duplicates")
        return tuple(sorted(names))


def authorize_governing_identity(
    action: str,
    version: str,
    repository: str,
    requested_issue: object,
    observed_pr: object | None,
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
    observed = ObservedVersionPrIdentity.from_pr(observed_pr, repository)
    if observed.branch != f"codexy/version-{version}":
        raise ValueError("observed PR version branch does not match requested version")
    if observed.issue != requested:
        raise ValueError("observed governing issue does not match requested issue")
