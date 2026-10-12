# Executable integer functions and recursive call frames

The full objective remains STE between specifications and binary execution,
then a useful Python replacement for AI agents. The verified baseline is
14/16 capability rungs and 570 tests. This architectural stage supplies genuine
value-returning functions, including recursive factorial and Fibonacci, without
changing frozen cases/oracles or treating legacy function traces as computation.

Extend existing Function declarations. Inputs and exactly one output are Int.
Optional `The function stores:` declares named Int locals. An explicit
`The function executes:` section contains ordered executable bullets:

```
- if local.n <= 1:
- the function returns 1
- end if
- the function calls factorial with n = local.n - 1 into local.partial
- the function returns local.n * local.partial
```

Inputs and locals are referenced as `local.name`. Local storage is isolated per
invocation, initially containing only bound parameters; uninitialized local
reads fail. Function bodies support integer computation, calls, early returns,
the existing structured control markers and explicit trace events. Functions
cannot access global fields or call stateful actions. Named call arguments are
separated by `;`, with `name = expression`. A zero-argument call uses `with none`.
Actions call functions using `the system calls name with ... into result.field`.
Arguments evaluate in sorted name order before entering the callee. The result
is assigned only after successful return. Calls can be forward or mutual.
Return falls through neither later statements nor enclosing blocks. Falling
off the body is a runtime missing-return error. Legacy descriptive function
bodies retain their documented trace semantics and cannot be CALL_INT targets.

Use AilIntegerFunction { locals: BTreeMap<String,String>, body: Vec<IntegerStep> }
inside AilFunction.integer: Option<AilIntegerFunction>. IntegerStep is Compute
{key, expression}, Call(AilIntegerCall), Return(IntExpr), Control(AilControl),
Trace(String). AilIntegerCall has target, arguments: BTreeMap<String,IntExpr>, key.
Actions gain integer_calls and ordered IntegerCall(index) steps.

Core stores executable Function attributes and ordered FunctionStep nodes,
typed IntegerExpression trees and local Field nodes owned by the function.
Calls reference their target Function and argument roots. Action value-call
steps share the action step order. Checker validates ownership, local/input
types, exact arity, single Int output, graph endpoints and step indices. Saved
Core and rendered source must preserve execution, and imported functions keep
local names while qualifying function targets/global action arguments.

Bytecode protocol:

- INTEGER_FUNCTION {parameters}: first instruction, semicolon-delimited names.
- CALL_INT {target, arguments, key}: arguments encode `name=postfix;name=postfix`.
- RETURN_INT {expression}: evaluated checked i64 return, exits only this frame.
- Missing return is detected at the end of an INTEGER_FUNCTION procedure.

Function bodies otherwise use EVAL_INT, BRANCH_INT, LABEL, JUMP, EMIT_TRACE.
INTEGER_FUNCTION is not executable as a root action; use an action wrapper.
Verifier rejects cross-kind calls, malformed signatures/arguments, duplicate or
missing arguments, invalid return placement, and instructions outside this
function subset. STE adds exact templates for these three operations.

VM uses isolated local maps and a maximum of 64 function frames, separate from
the existing stateful-action depth. Each function invocation receives the same
instruction budget formula as an action. Native uses real machine calls and
stack frames, never recursive inlining. Parent calls restore state storage and
fuel after return. Both backends agree on checked arithmetic, missing locals,
depth/fuel exhaustion and missing returns. A native function failure produces
nonzero exit with no result stdout. Pure function traces are opt-in so recursive
computation does not require storing a trace per instruction or invocation.

Independent implementation domains: function source/Core module, native code
generation, and root VM/STE/toolchain integration. Current worktree remains the
authorized workspace. Verification includes multiple calls per frame, caller
local preservation, mutual recursion, named-argument reordering, malformed
Core/STE, imports, early return, no return, errors and actual ELF execution.
