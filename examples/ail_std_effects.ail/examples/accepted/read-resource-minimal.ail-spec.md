# Accepted Effects Fixture

Package: ail.std.effects.

The application AIL Standard Effects manages declared read and write effects.

Action: Read resource.

When the runtime reads a resource:

- the system requires the resource to be declared
- the system reads resource data
- the system records a trace event named ResourceRead
