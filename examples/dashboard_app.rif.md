app Dashboards

things:
  thing Dashboard
    field counts: Map<Text, Int>

operations:
  operation Metrics.publish(counts: Map<Text, Int>) -> Unit

intent PublishDashboard

subject:
  dashboard: Dashboard

steps:
  1. Set counts
     set: dashboard.counts = {"open":1,"closed":2}
     changes: dashboard.counts

  2. Publish counts
     call: Metrics.publish({"open":1,"closed":2})

guarantees:
  if this intent succeeds:
    dashboard.counts == {"open":1,"closed":2}
