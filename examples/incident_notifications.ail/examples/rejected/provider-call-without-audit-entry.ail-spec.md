# Rejected Provider Call Without Audit Entry Fixture

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

Failure ProviderDeliveryFailed happens when PagerProvider.notify rejects delivery:

- the system records failure ProviderDeliveryFailed
- the system retries PagerProvider.notify with bounded backoff
- the system escalates to the incident commander if delivery still fails
- the trace records NotificationProviderDeliveryFailed
