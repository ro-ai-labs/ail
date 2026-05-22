app IssueTracker

things:
  thing Issue
    field id: Id<Issue>
    field status: State<Open, Resolved, Closed>
    field assignee: Ref<User>

  thing User
    field id: Id<User>
    field email: Text

  thing Comment
    field id: Id<Comment>
    field body: Text

operations:
  operation IssuePolicy.require_assignee(issue: Issue, actor: User) -> Bool
    reads: issue.assignee, actor.id
    may fail with: NotAssignee

  operation IssueComments.add(issue_id: Id<Issue>, body: Text) -> Comment
    changes: IssueComments

  operation Notification.send_issue_resolved(issue_id: Id<Issue>) -> Unit
    external call: Notification
    may fail with: NotificationFailed

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

  2. Add resolution note
     call: IssueComments.add(issue.id, resolution_note)
     output: comment: Comment

  3. Mark issue resolved
     set: issue.status = Resolved
     changes: issue.status

  4. Notify watchers
     call: Notification.send_issue_resolved(issue.id)

failure behavior:
  if assignee check fails:
    stop with NotAssignee

  if notification fails:
    ignore NotificationFailed

guarantees:
  if this intent succeeds:
    issue.status is Resolved
    comment exists

intent ReopenIssue

subject:
  issue: Issue

requires:
  issue.status is Resolved

state transition:
  issue.status: Resolved -> Open

steps:
  1. Mark issue open
     set: issue.status = Open
     changes: issue.status

guarantees:
  if this intent succeeds:
    issue.status is Open
