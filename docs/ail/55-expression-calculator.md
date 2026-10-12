# An expression interpreter written in AIL

The [expression calculator](../../examples/expression-calculator/README.md)
tokenizes, parses and evaluates a small expression language using AIL functions.
Its [library source](../../examples/ail_std_expression.ail/spec.ail-spec.md)
uses ordinary text, list, JSON, control-flow and arithmetic operations. The
compiler treats it like any other program; the compiled Linux x86-64 executable
does the computation without Python or an external calculator.

## Run a request

```bash
cargo build
target/debug/ail ail-run examples/expression-calculator --action Evaluate \
  'data.expression=2 + rate * (units - 1)' \
  'data.variables={"rate":7,"units":4}'
```

The selected result is:

```text
out.result={"ok":true,"value":23,"error":""}
```

For request streams:

```bash
target/debug/ail ail-compile examples/expression-calculator --action Stream \
  --target linux-x86_64-elf --out /tmp/ail-calculator
/tmp/ail-calculator --stdio < examples/expression-calculator/input.jsonl
```

Each nonblank input line is an object with `expression` (Text) and `variables`
(object). Extra request members are ignored. Each produces one compact JSON
object and LF on stdout. Structured expression errors do not stop later requests.
Successful stream execution writes no stderr. The program reads bounded stdin
once; it is not a persistent interactive server. `--stdio` explicitly grants
standard streams and suppresses the usual CLI state/trace output.

## Grammar and arithmetic

Expressions accept decimal integer literals, ASCII identifiers, parentheses,
unary `+`/`-`, and binary `+`, `-`, `*`, `/`, `%`. Identifiers start with an ASCII
letter or underscore and continue with letters, digits or underscores. Whitespace
is exactly space, tab, LF, CR, vertical tab and form feed. Leading decimal zeros
are allowed. Implicit multiplication, calls, assignment and exponentiation are
unsupported: `2(3)`, `f(1)` and `2**3` report SYNTAX, while `x=1` and `2^3`
report LEX because their punctuation is outside the grammar.

Unary operators bind most tightly and associate right. Multiplication, division
and remainder bind more tightly than addition and subtraction. Binary operators
associate left. Parentheses override precedence. The parser converts tokens to
postfix using an explicit operator stack, so deep parentheses do not consume
recursive function frames.

Values and intermediate results use checked signed 64-bit integers. Division
truncates toward zero; remainder has the dividend's sign. For example, `-7 / 3`
is `-2`, and `-7 % 3` is `-1`. This differs from Python's `//` and `%` rules.
Both minimum-i64 division and remainder by `-1` report OVERFLOW. Negating minimum
i64 also overflows. Decimal literals themselves must fit 0..9223372036854775807;
use `-9223372036854775807 - 1` or a variable for minimum i64.

Variables come from the provided JSON object. A selected member must be a unique
integer number token within signed i64. Strings, Booleans, null, decimal/exponent
spellings and duplicate selected keys report NAME. Unused members are permitted
to have other types. Names are case-sensitive.

## Structured failures and bounds

Every result has exactly these fields in order:

```json
{"ok":true,"value":14,"error":""}
{"ok":false,"value":0,"error":"SYNTAX"}
```

| Error | Cause |
| --- | --- |
| REQUEST | Malformed JSON or invalid required request/variables shape |
| LIMIT | Expression exceeds 4096 Unicode scalars or 512 tokens |
| LEX | Character outside the lexical grammar |
| SYNTAX | Empty expression, missing operand, mismatched parentheses or invalid arrangement |
| NAME | Selected variable is missing, duplicated or is not an i64 integer token |
| OVERFLOW | Literal or intermediate arithmetic result exceeds signed i64 |
| DIVZERO | Division or remainder by zero |

Request shape is checked first, followed by expression length, complete lexing,
complete syntax parsing, then evaluation in postfix order. The lexer stops at
its first invalid character or token-limit failure. Syntax errors take priority
over arithmetic errors: `1 / 0 +` returns SYNTAX. Invalid characters take priority
over syntax: `1 + @` returns LEX. Results do not echo request contents.

These are library limits, within the existing runtime bounds: 1 MiB Text/Json
values and stdin, shared 64 MiB Text and List/Json result arenas, and existing
instruction limits. Large request batches can exhaust those shared budgets.
Fatal runtime resource or capability failures remain fatal; a library result
does not turn the runtime into an unbounded service. Stream blank-line detection
uses AIL's Unicode trim set; whitespace inside expressions uses the six ASCII
characters above.
For a nonblank stream record, the original line is parsed as JSON; blank-line
detection does not trim invalid outer characters from a request.

## Import the library

Add a local package import to `ail-package.md`, relative to the importing package:

```text
imports: ../ail_std_expression.ail as Calc
```

Then call either public function from an action or another function:

```text
- the system calls Calc.evaluate with expression = data.expression; variables = data.variables into out.result
- the system calls Calc.evaluateRequest with text = data.request into out.result
```

Declare `expression` and `request` as Text, and `variables` and `result` as Json.
Inside a function, use `the function calls` and declared `local.*` operands
instead of the action's global fields.
`evaluate` accepts already-parsed values; `evaluateRequest` handles the JSON
request envelope. Both return the same result schema and perform no I/O.

## Verification

```bash
python3 scripts/ail_calculator_differential.py
python3 scripts/ail_bench.py --rung R026-expression-calculator
```

The oracle implements separate Python recursive descent and AST evaluation,
while AIL uses shunting yard and a value stack. It checks actual results through
source, saved Core, [STE](../../examples/ste/expression-calculator.ail-ste) and
bytecode routes, with native execution, malformed requests, limits and generated
expressions. Python is test tooling only. The new rung preserves the previously
frozen benchmark definitions.

See the [implementation record](../superpowers/plans/2026-10-11-expression-calculator.md)
for verification evidence: the 73-case rung passes on both backends, and the
independent oracle checks 1,044 individual results and 4,212 streamed results
across 12 routes, including deliberate arithmetic mutation detection. The
complete toolchain regression target passes 434 tests.
The full frozen capability ladder passes 17/17, with all prior rung definitions
and fingerprints preserved.
This is the expression-interpreter capability;
it does not establish the remaining ladder tiers or general Python compatibility.
