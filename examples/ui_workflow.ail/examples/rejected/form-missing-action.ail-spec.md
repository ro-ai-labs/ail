# UI Form Missing Action Rejected Fixture

The application Support UI manages ticket intake.

Form: Create ticket.

The form calls action:

- MissingAction

The form fields are:

- title: Text

The form validates:

- title is not empty

If form validation fails:

- FormValidationFailed

The form accessibility is:

- title error is announced

