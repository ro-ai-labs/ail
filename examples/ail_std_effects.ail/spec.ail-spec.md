# AIL Standard Effects Package

Package: ail.std.effects.

The application AIL Standard Effects manages declared read and write effects.

A ResourceEffect has:

- id: Text
- resource: Text
- effect: State<Read, Write>

Action: Read resource.

When the runtime reads a resource:

- the system requires the resource to be declared
- the system reads resource data
- the system records a trace event named ResourceRead

Action: Write resource.

When the runtime writes a resource:

- the system requires the resource to be declared
- the system changes resource data
- the system records a trace event named ResourceWritten
