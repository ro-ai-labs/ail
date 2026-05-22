intent ResetPassword

subject:
  user: User

inputs:
  reset_token: Text
  new_password: Secret<Text>

requires:
  user.status is Active

steps:
  1. Verify reset token
     call: Auth.verify_reset_token(user, reset_token)
     output: token_ok: Bool
     reads: user.id
     reads: reset_token
     may fail with: InvalidResetToken

  2. Hash password
     call: Hash.password(new_password)
     output: password_hash: PasswordHash
     reads: new_password

  3. Store password hash
     set: user.password_hash = password_hash
     changes: user.password_hash

  4. Send confirmation email
     call: Email.send_password_changed(user.email)
     reads: user.email
     external call: Email
     may fail with: EmailDeliveryFailed

failure behavior:
  if reset token verification fails:
    stop with InvalidResetToken

  if email delivery fails:
    ignore EmailDeliveryFailed

guarantees:
  if this intent succeeds:
    password_hash exists

