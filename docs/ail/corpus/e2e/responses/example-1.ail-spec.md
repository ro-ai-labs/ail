# AIL Standard Collections Package

Package: ail.std.collections.

Type: Option<T>.

Option has variants:

- Some(value: T)
- None

Type: Result<T,E>.

Result has variants:

- Success(value: T)
- Failure(error: E)

Type: List<T>.

List has variants:

- Empty
- Items(values: T)

Type: Map<K,V>.

Map has variants:

- Empty
- Entries(key: K, value: V)

Type: Set<T>.

Set has variants:

- Empty
- Members(value: T)

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
- the function records a trace event named OptionMapEvaluatedScenario001
