app DailyCleanup

things:
  thing CleanupJob
    field id: Text
    field last_run: Time
    field runs: Int

triggers:
  trigger nightly.cleanup -> RunCleanup
    schedule: PT24H
    payload:
      job_id: Text
      run_at: Time
      run_index: Int
    requires:
      event.name is nightly.cleanup
      event.schedule is PT24H
    bind:
      job.id = event.job_id
      job.last_run = event.run_at
      job.runs = event.run_index

intent RunCleanup

subject:
  job: CleanupJob

steps:
  1. Record run
     set: job.runs = job.runs + 1
     changes: job.runs

returns:
  job_id: job.id
  job_runs: job.runs
