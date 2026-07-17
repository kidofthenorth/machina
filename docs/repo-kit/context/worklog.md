# Worklog
<!-- Append-only ledger. Nothing is in flight without a row. Reconcile at session start + end (T4).
     States: QUEUED → IN-PROGRESS → DONE-AUTO-VERIFIED → NEEDS-HUMAN → CLOSED, plus BLOCKED.
     Rows close on pasted gate evidence, never prose. NEEDS-HUMAN is a label, never a stop. -->

| ID | Task | State | Evidence |
|----|------|-------|----------|
