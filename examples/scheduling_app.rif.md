app Scheduling

things:
  thing Job
    field id: Id<Job>
    field starts_at: Time
    field timeout: Duration

operations:
  operation Scheduler.schedule(at: Time, timeout: Duration) -> Unit

intent ScheduleJob

subject:
  job: Job

steps:
  1. Set start time
     set: job.starts_at = 2026-05-20T09:30:00Z
     changes: job.starts_at

  2. Set timeout
     set: job.timeout = PT30M
     changes: job.timeout

  3. Schedule job
     call: Scheduler.schedule(2026-05-20T09:30:00Z, PT30M)

guarantees:
  if this intent succeeds:
    job.starts_at == 2026-05-20T09:30:00Z
    job.timeout == PT30M
