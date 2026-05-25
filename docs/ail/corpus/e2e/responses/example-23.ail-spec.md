# Option Map AIL-Spec Example

Package: ail.std.collections.

Type: Option<T>.

Option has variants:

- Some(value: T)
- None

Function: Option.map.

The function needs:

- option: Option<T>
- mapper: Text

The function produces:

- result: Option<U>

When Option.map runs:

- if the option is Some(value), the function calls mapper with value
- the function returns Some(mapped value)
- if the option is None, the function returns None
- the function records a trace event named OptionMapEvaluatedScenario023
