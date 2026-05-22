app Profiles
module Profiles.Registry

things:
  thing Profile
    field email: Text
    field age: Int

collections:
  collection profiles: Profile
    unique: email

intent ListProfiles

subject:
  report: Report

things:
  thing Report
    field count: Int

steps:
  1. Count profiles
     set: report.count = profiles.count
