rung: R026-expression-calculator
tier: 2
action: Evaluate
targets: vm,native
blocked-on: pure AIL expression lexer, parser and checked evaluator

# R026-expression-calculator — Interpret an expression language

Given `data.expression: Text` and `data.variables: Json`, return `out.result: Json`
exactly as `{"ok":true,"value":14,"error":""}` on success or
`{"ok":false,"value":0,"error":"CODE"}` on failure, in that key order.

Interpret ASCII decimal literals, identifiers, parentheses, unary +/- and binary
+ - * / %. Operators use conventional precedence and left binary associativity;
unary signs associate right. Whitespace is ASCII space/tab/LF/CR/VT/FF. No host
or language eval primitive, external process, evaluator opcode or compiler special
case may implement the interpreter. The AIL program must lex, parse and evaluate.

Variables must be a JSON object. Selected names must be unique integral i64
number tokens; unused values are unrestricted JSON. Arithmetic is checked i64,
division truncates toward zero, and remainder follows the dividend's sign.
MIN/-1 and MIN%-1 overflow. Decimal literals may have leading zeros but must not
exceed MAX; thus spelling MIN as one negated positive literal overflows.

Error priority: request/object shape; 4096-scalar expression limit; complete lexing
with at most 512 tokens; complete syntax; left-to-right postfix-equivalent checked
evaluation. Codes are REQUEST, LIMIT, LEX, SYNTAX, NAME, OVERFLOW, DIVZERO.

The independently authored Python reference uses recursive descent and an AST,
not the AIL implementation's shunting-yard design. Cases were frozen and recorded
as unimplemented before program.ail was added. Full grammar and public request
stream contract: docs/superpowers/specs/2026-10-11-expression-calculator-design.md.
