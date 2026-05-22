intent ResolveIssue

subject:
  issue: Issue

inputs:
  actor: User
  resolution_note: Text

requires:
  issue.status is Open

state transition:
  issue.status: Open -> Resolved

steps:
  1. Check assignee
     call: IssuePolicy.require_assignee(issue, actor)
     reads: issue.assignee
     reads: actor.id
     may fail with: NotAssignee

  2. Add resolution note
     call: IssueComments.add(issue.id, resolution_note)
     output: comment: Comment
     reads: issue.id
     reads: resolution_note
     changes: IssueComments

  3. Mark issue resolved
     set: issue.status = Resolved
     changes: issue.status

  4. Notify watchers
     call: Notification.send_issue_resolved(issue.id)
     reads: issue.id
     external call: Notification
     may fail with: NotificationFailed

failure behavior:
  if assignee check fails:
    stop with NotAssignee

  if notification fails:
    ignore NotificationFailed

guarantees:
  if this intent succeeds:
    issue.status is Resolved
    comment exists

