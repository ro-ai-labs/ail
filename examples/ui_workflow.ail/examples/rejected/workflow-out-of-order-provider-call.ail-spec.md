# UI Workflow Rejected Fixture

The application Support UI manages provider handoff workflows.

Action: Provider call.

When a provider call starts:

- the system requires Manager approval
- the system records a trace event named ProviderCallStarted

Workflow: Refund approval.

The workflow steps are:

- Request
- Provider call
- Manager approval

The workflow blocks:

- Provider call before Manager approval

The workflow records trace:

- RefundApprovalWorkflowViewed
