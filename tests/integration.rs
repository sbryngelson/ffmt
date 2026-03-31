use std::fs;
use std::path::Path;

fn run_fixture(name: &str) {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = fs::read_to_string(fixture_dir.join(format!("{}.input.fpp", name)))
        .unwrap_or_else(|e| panic!("Failed to read {}.input.fpp: {}", name, e));
    let expected = fs::read_to_string(fixture_dir.join(format!("{}.expected.fpp", name)))
        .unwrap_or_else(|e| panic!("Failed to read {}.expected.fpp: {}", name, e));
    let result = ffmt::format_string(&input);
    if result != expected {
        // Print a useful diff
        let result_lines: Vec<&str> = result.lines().collect();
        let expected_lines: Vec<&str> = expected.lines().collect();
        for (i, (r, e)) in result_lines.iter().zip(expected_lines.iter()).enumerate() {
            if r != e {
                eprintln!("Line {} differs:", i + 1);
                eprintln!("  got:      {:?}", r);
                eprintln!("  expected: {:?}", e);
            }
        }
        if result_lines.len() != expected_lines.len() {
            eprintln!(
                "Line count: got {}, expected {}",
                result_lines.len(),
                expected_lines.len()
            );
        }
        panic!("Fixture '{}' mismatch", name);
    }
}

#[test]
fn test_simple() {
    run_fixture("simple");
}

#[test]
fn test_fypp() {
    run_fixture("fypp");
}

#[test]
fn test_directives() {
    run_fixture("directives");
}

#[test]
fn test_call_block() {
    run_fixture("call_block");
}

#[test]
fn test_blank_lines() {
    run_fixture("blank_lines");
}

#[test]
fn test_doxygen_spacing() {
    run_fixture("doxygen_spacing");
}

#[test]
fn test_idempotent_simple() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = fs::read_to_string(fixture_dir.join("simple.input.fpp")).unwrap();
    let first = ffmt::format_string(&input);
    let second = ffmt::format_string(&first);
    assert_eq!(
        first, second,
        "Formatter is not idempotent on simple fixture"
    );
}

#[test]
fn test_idempotent_fypp() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = fs::read_to_string(fixture_dir.join("fypp.input.fpp")).unwrap();
    let first = ffmt::format_string(&input);
    let second = ffmt::format_string(&first);
    assert_eq!(first, second, "Formatter is not idempotent on fypp fixture");
}

#[test]
fn test_trailing_comment_wrap() {
    run_fixture("trailing_comment_wrap");
}

#[test]
fn test_idempotent_trailing_comment_wrap() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = fs::read_to_string(fixture_dir.join("trailing_comment_wrap.input.fpp")).unwrap();
    let first = ffmt::format_string(&input);
    let second = ffmt::format_string(&first);
    assert_eq!(
        first, second,
        "Formatter is not idempotent on trailing_comment_wrap fixture"
    );
}

#[test]
fn test_error_recovery() {
    let input =
        "subroutine s_foo()\n    @@@ weird line @@@\n    integer :: x\nend subroutine s_foo\n";
    let result = ffmt::format_string(input);
    assert!(result.contains("@@@ weird line @@@"));
    assert!(result.contains("end subroutine"));
    let second = ffmt::format_string(&result);
    assert_eq!(
        result, second,
        "Formatter is not idempotent with unrecognized lines"
    );
}
