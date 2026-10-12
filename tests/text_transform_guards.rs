use ail::ail::{AilPackage, load_ail_package_dir, parse_ail_package_spec_text};

fn descriptive_sources(compiler: &AilPackage, statement: &str) -> [String; 2] {
    [
        format!(
            "The application Texts manages records.\n\nFunction: legacy.\nThe function needs:\n- raw: Text\nThe function produces:\n- result: Text\nWhen legacy runs:\n- {statement}\n- the function returns raw unchanged\n"
        ),
        compiler.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        ),
    ]
}

#[test]
fn punctuated_malformed_transformations_are_rejected_in_descriptive_bodies() {
    let application = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for subject in ["the system", "the function"] {
        for operation in ["splits", "replaces", "trims", "joins list"] {
            for punctuation in ["", "."] {
                let statement = format!("{subject} {operation}{punctuation}");
                for (package, text) in [&application, &compiler]
                    .into_iter()
                    .zip(descriptive_sources(&compiler, &statement))
                {
                    let error = parse_ail_package_spec_text(package, &text)
                        .expect_err(&format!("accepted {statement}"));
                    assert!(error.contains("AIL-TEXT-001"), "{error}");
                }
            }
        }
    }
}

#[test]
fn transformation_guards_preserve_unrelated_descriptive_identifiers() {
    let application = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for subject in ["the system", "the function"] {
        for operation in [
            "splits_record fields",
            "replaces_map entries",
            "trims_record names",
            "joins listwise records",
        ] {
            let statement = format!("{subject} {operation}.");
            for (package, text) in [&application, &compiler]
                .into_iter()
                .zip(descriptive_sources(&compiler, &statement))
            {
                assert!(
                    parse_ail_package_spec_text(package, &text).is_ok(),
                    "rejected unrelated statement {statement}"
                );
            }
        }
    }
}
