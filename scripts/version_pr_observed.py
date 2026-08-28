"""Observed version pull-request identity parsing."""

from __future__ import annotations

from dataclasses import dataclass

from version_pr_identity import (
    CanonicalIssueIdentity,
    parse_body_closing_references,
    parse_repository,
    require_object,
)
from version_pr_tracks import parse_tracks_issue_number


@dataclass(frozen=True)
class ObservedVersionPrIdentity:
    branch: str
    issue: CanonicalIssueIdentity
    labels: tuple[str, ...]
    body: str

    @classmethod
    def from_pr(
        cls, value: object, repository: str, issue_link_mode: str = "closing"
    ) -> "ObservedVersionPrIdentity":
        pr = require_object(value, "observed PR")
        branch = pr.get("headRefName")
        if not isinstance(branch, str) or not branch or branch != branch.strip():
            raise ValueError("observed PR requires a canonical version branch")
        references = pr.get("closingIssuesReferences")
        if issue_link_mode == "nonclosing":
            return cls._from_nonclosing(pr, branch, references, repository)
        if issue_link_mode != "closing":
            raise ValueError("unsupported governing issue link mode")
        return cls._from_closing(pr, branch, references, repository)

    @classmethod
    def _from_nonclosing(
        cls,
        pr: dict[str, object],
        branch: str,
        references: object,
        repository: str,
    ) -> "ObservedVersionPrIdentity":
        if not isinstance(references, list) or references:
            raise ValueError("existing provisional release PR must not close an issue")
        body = cls._body(pr)
        owner, name = parse_repository(repository)
        number = parse_tracks_issue_number(body)
        issue = CanonicalIssueIdentity(
            owner, name, number, f"https://github.com/{repository}/issues/{number}"
        )
        return cls(branch, issue, cls._labels(pr.get("labels")), body)

    @classmethod
    def _from_closing(
        cls,
        pr: dict[str, object],
        branch: str,
        references: object,
        repository: str,
    ) -> "ObservedVersionPrIdentity":
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
        body = cls._body(pr)
        if parse_body_closing_references(body, repository) != (issue,):
            raise ValueError("observed PR body must end with the governing issue link")
        expected_line = f"Fixes #{number}"
        if not (lines := [line for line in body.splitlines() if line]) or (
            lines[-1] != expected_line
        ):
            raise ValueError(
                "observed PR body must end with exactly one canonical closing issue reference"
            )
        return cls(branch, issue, cls._labels(pr.get("labels")), body)

    @staticmethod
    def _body(pr: dict[str, object]) -> str:
        body = pr.get("body")
        if not isinstance(body, str):
            raise ValueError("observed PR requires a body")
        return body

    @staticmethod
    def _validate_reference_repository(
        reference: dict[str, object], repository: str
    ) -> None:
        expected_owner, expected_name = parse_repository(repository)
        observed = require_object(
            reference.get("repository"), "closing issue repository"
        )
        owner = require_object(observed.get("owner"), "closing issue repository owner")
        if (
            observed.get("name") != expected_name
            or owner.get("login") != expected_owner
        ):
            raise ValueError(
                f"observed closing issue reference must belong to {repository}"
            )

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
