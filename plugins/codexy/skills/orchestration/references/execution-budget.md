Non-trivial lanes MUST cap implementation/repair/review; continuation MUST meet
criterion/blocker; churn/waits MUST NOT renew caps. Fanout<=3 non-Sentinel.
Terminal PASS/BLOCK/UNOBSERVABLE<=3 issue-wide; PENDING/RUNNING nonterminal.
Third BLOCK requires all root repairs+exact-head proof; fourth review MUST NOT
occur. Third UNOBSERVABLE requires maintainer disposition+equivalent proof.
Test/CI/thread/safety/ownership/LOC/merge gates remain. External-only: one idle-
wait handoff then finish; waiting/exhaustion MUST NOT block absent an unanswered
decision without safe default.
