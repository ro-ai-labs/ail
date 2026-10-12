use ail::ail::*;
use ail::bool_expression::BoolExpr;

fn core(expression: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let text = format!(
        "The application Predicates manages records.\n\nAn Output has:\n- selected: Bool\n\nAction: Run.\nWhen Run happens:\n- the system computes boolean output.selected as {expression}\n- the system records a trace event named Done\n"
    );
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, &text).unwrap(),
    )
}

#[test]
fn saved_core_counts_escaped_text_bytes_across_one_predicate() {
    let saved = render_ail_core(&core("text \"x\" == \"y\""));
    for (left, right, valid) in [(5000, 5000, true), (6000, 6000, false), (11000, 1, false)] {
        let escaped = |n| format!("\"{}\"", "\\u0061".repeat(n));
        let text = saved
            .replace("value=\"x\"", &format!("value={}", escaped(left)))
            .replace("value=\"y\"", &format!("value={}", escaped(right)));
        let checked = parse_ail_core_text(&text).and_then(|core| compile_ail_core_bytecode(&core));
        assert_eq!(checked.is_ok(), valid, "escaped counts {left}/{right}");
    }
}

#[test]
fn bounded_boolean_source_renders_without_extra_nesting_or_bytes() {
    for count in [1, 63, 64, 100, 126] {
        for operand in ["data.x", "(data.x + 1)"] {
            let source = format!("floating {}{operand} == 0", "-".repeat(count));
            if let Ok(expression) = BoolExpr::parse(&source) {
                assert_eq!(
                    BoolExpr::parse(&expression.to_source()).unwrap(),
                    expression
                );
            }
        }
    }
    for operand in ["-(1)", "-(0)", "-(nan)", "--(1)", "-(-1)"] {
        let source = format!("floating {operand} < 0");
        let expression = BoolExpr::parse(&source).unwrap();
        assert_eq!(
            BoolExpr::parse(&expression.to_source()).unwrap(),
            expression
        );
    }
    for source in [
        format!(
            "boolean(boolean data.{}==true)==(boolean true==true)",
            "a".repeat(65480)
        ),
        format!("boolean(not data.{})==(integer 1<2)", "a".repeat(65499)),
    ] {
        let expression = BoolExpr::parse(&source).unwrap();
        assert_eq!(
            BoolExpr::parse(&expression.to_source()).unwrap(),
            expression
        );
    }
    let source = format!("boolean data.{} == true", "a".repeat(65512));
    let expression = BoolExpr::parse(&source).unwrap();
    assert_eq!(
        BoolExpr::parse(&expression.to_source()).unwrap(),
        expression
    );
}

#[test]
fn boolean_named_integer_fields_keep_legacy_compute_syntax() {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let text = "The application Predicates manages records.\n\nA Boolean has:\n- count: Int\n\nAction: Run.\nWhen Run happens:\n- the system computes boolean.count as 1 + 2\n- if boolean.count == 3:\n- the system computes boolean.count as boolean.count + 1\n- end if\n- the system returns boolean.count\n- the system records a trace event named Done\n";
    let document = parse_ail_package_spec_text(&package, text).unwrap();
    let program = compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", Default::default()).unwrap();
    assert_eq!(result.final_state["boolean.count"], "4");
}
