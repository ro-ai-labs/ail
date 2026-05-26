# Darwin Linux Effect AIL-Spec Example

The application Darwin Linux Effect App manages target-specific effects.

System component: Linux syscall bridge.

The component requires capability:

- call linux syscall exit

The component performs:

- linux syscall exit

Action: Linux exit.

When linux exit happens:

- the system records a trace event named LinuxExit
