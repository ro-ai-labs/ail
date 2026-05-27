# Accepted Notify Responder Fixture

Tool: Notify incident responder.

The AI Agent may request Notify incident responder when:

- the incident exists
- a responder is assigned
- the notification channel is approved

The tool needs:

- incident id: Text
- responder pager: Text
- message: Text
- pager token: Secret<Text>

The tool produces:

- notification id: Text

The tool can:

- read the incident
- call PagerProvider.notify
- write NotificationAudit entry

The tool must not:

- reveal the pager token

The tool requires permission:

- requester may notify responders

The tool requires approval:

- incident commander approval when severity is Sev1

The tool records:

- IncidentResponderNotified

The tool guarantees:

- pager token is redacted from all agent-visible output
- every notification request is represented in the audit trace
