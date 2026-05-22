app OptionalProfiles

things:
  thing User
    field id: Id<User>
    field nickname: Option<Text>
    field age: Option<Int>

operations:
  operation Profile.update(nickname: Option<Text>, age: Option<Int>) -> Unit

intent UpdateProfile

subject:
  user: User

steps:
  1. Clear nickname
     set: user.nickname = None
     changes: user.nickname

  2. Set age
     set: user.age = Some(42)
     changes: user.age

  3. Update profile
     call: Profile.update(Some("Ada"), None)

guarantees:
  if this intent succeeds:
    user.nickname == None
    user.age == Some(42)
