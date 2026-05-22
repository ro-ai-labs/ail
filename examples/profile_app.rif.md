app Profiles

things:
  thing User
    field id: Id<User>
    field profile: Profile

  thing Profile
    field email: Text
    field age: Int

intent CompleteProfile

subject:
  user: User

steps:
  1. Set profile age
     set: user.profile.age = 18
     changes: user.profile.age

guarantees:
  if this intent succeeds:
    user.profile.age > 0
