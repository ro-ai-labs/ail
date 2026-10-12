# Expression calculator written in AIL

## Intent and scope

The active objective remains Specs → ASD-STE100-based executable intermediate →
binary, followed by a language useful for Python replacement and AI agents. The
previous directory milestone provides real I/O. This milestone tests generality
by implementing the capability ladder's expression-language interpreter in AIL,
with reusable library functions and an agent-friendly JSON-lines interface.
No evaluator opcode, host eval, external calculator, or compiler special case is
permitted. AIL source must implement tokenization, parsing and arithmetic.

Alternatives considered: another host effect would expand access but would not
prove language composition; embedding a host evaluator would bypass the language.
A pure AIL library exercises the current language and can reveal actual compiler
defects. The standalone result must run with only the existing AIL runtime.

## Library and grammar

Create `examples/ail_std_expression.ail` with Application profile and reusable
`evaluate(expression: Text, variables: Json) -> Json` and
`evaluateRequest(text: Text) -> Json` functions. Local imports use alias `Calc`.

```text
expression = sum
sum        = product (("+" | "-") product)*
product    = unary (("*" | "/" | "%") unary)*
unary      = ("+" | "-") unary | primary
primary    = digits | identifier | "(" expression ")"
digits     = ASCII digit+
identifier = (ASCII letter | "_") (ASCII letter | ASCII digit | "_")*
```

Whitespace is exactly ASCII space, tab, LF, CR, vertical tab and form feed.
Leading zeros in decimal literals are permitted. Other characters are lexical
errors, including Unicode digits/letters, NUL, decimal points and quote marks.
Binary operators associate left; unary operators associate right and have higher
precedence. Parentheses may nest up to the token bound. No implicit multiplication,
calls, assignment, comparison, exponentiation, comments or attribute access.

Implement lexer → shunting-yard postfix parser → stack evaluator as separate
AIL functions. This iterative architecture avoids making parentheses consume the
runtime's 64 function frames. Use typed Text/Int lists and existing operations;
the implementation may choose its private helper names and result transports.

## Values, errors and order

Every public result is exactly `{"ok":true,"value":14,"error":""}` or
`{"ok":false,"value":0,"error":"CODE"}`, in that key order. Results contain
no echoed input. Variables must be a JSON object. Looked-up identifiers must be
present and have an integral JSON number token representable as signed i64;
Boolean, null, string, decimal and exponent spellings are not Int values. Unused
members may have any JSON value. Duplicate selected names follow AIL JSON lookup
rejection and report NAME. Decimal literals must be in 0..9223372036854775807;
the minimum i64 is expressible via a variable or `-9223372036854775807 - 1`.

Arithmetic is checked signed i64. Division truncates toward zero; remainder is
`a - trunc(a/b)*b`. Division and remainder of i64 minimum by -1 overflow under
the existing AIL semantics. Unary negation of minimum overflows. Unary plus
preserves its operand. Intermediate overflow is an error, even when subsequent
operations could bring a mathematical result back into range.

`evaluateRequest` accepts one JSON object with required `expression: Text` and
`variables: object`; extra members are ignored. Malformed JSON, wrong root,
missing/wrong required fields are REQUEST. `evaluate` also checks object kind.
Priority is request shape → expression scalar length (at most 4096) → complete
lexing (at most 512 tokens) → complete syntax parsing → postfix evaluation.
Scanning stops at the first invalid character or token-count overflow; completed
earlier tokens count. Literal range and variable lookup occur during evaluation,
left-to-right in postfix order. All syntax must pass before any arithmetic.

| Code | Meaning |
| --- | --- |
| REQUEST | Invalid request/variables shape or malformed JSON |
| LIMIT | More than 4096 expression characters or more than 512 tokens |
| LEX | Character outside the lexical grammar |
| SYNTAX | Empty expression or invalid token arrangement |
| NAME | Selected variable absent, duplicated or not an i64 integer token |
| OVERFLOW | Decimal literal or arithmetic outside signed i64 |
| DIVZERO | Division or remainder by zero |

Existing fatal runtime input, allocation and instruction bounds still apply;
this library does not catch fatal capability/resource errors or promise unbounded
stream processing. Request parsing follows the existing JSON limits. Empty text
is a valid expression input that returns SYNTAX. Whitespace-only stream lines
are skipped using existing AIL trim behavior, separate from expression lexing.

## Executables and independent evidence

`examples/expression-calculator` imports the library. Action `Evaluate` reads
`data.expression: Text`, `data.variables: Json`, returns `out.result: Json`.
Action `Stream` reads bounded stdin once, splits LF records, skips trim-empty
lines, invokes evaluateRequest on each other line, writes one compact JSON result
and LF per request, and continues after every structured error. `--stdio` grants
streams and suppresses diagnostic state output. No stderr on normal operation.
Trim only a copy for blank detection; parse the original nonblank record so
non-JSON outer whitespace is not silently accepted.
Check in a mixed sample and exact generated STE export.

Add a new R026-expression-calculator benchmark with independently authored Python
reference and cases before the AIL implementation. Preserve every old rung input
and lock fingerprint. Freeze the new inputs before implementation; record a
genuine unimplemented RED, then add an importing AIL program. Recording an added
passing rung via --update-lock is allowed; --bless-ladder is not.

The independent oracle uses its own lexer and recursive-descent AST evaluator
(different parsing structure from AIL). Never Python eval. Test deterministic
generated expressions and independently calculated outcomes, malformed lexical
and syntax classes, error precedence, i64 boundaries, unary/associativity rules,
512-token/4096-character exact bounds and above, deep parentheses, duplicate JSON
variables and recovery across a mixed stream. Compare actual stdout and status
across source, saved Core, STE and bytecode VM/native routes; inspect no Python
or external-process dependency in the program. Tests must fail on a deliberately
altered evaluator operation. Existing implementation hashes determine whether
broad compiler regression suites need rerunning; pure new AIL programs do not
justify repeating unchanged Rust semantics suites.

## Workflow constraints

Continue in the current workspace, preserving all previous uncommitted work.
`.git` is read-only; no commits, worktrees, resets or cleanup. The user's active
autonomous continuation governs reversible design/implementation choices. Keep
the full goal active after this milestone; this is neither all ladder tiers nor
general Python compatibility. Record actual outcomes, not inferred completion.
