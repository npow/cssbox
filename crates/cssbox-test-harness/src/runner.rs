//! Test execution and reporting.

use crate::reftest;
use crate::testharness;
use crate::wpt_parser::{ReftestRelation, WptTestFile, WptTestType};

/// Result of running a single WPT test.
#[derive(Debug)]
pub struct TestResult {
    pub path: String,
    pub title: String,
    pub outcome: TestOutcome,
    pub details: String,
}

/// Possible outcomes of a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutcome {
    Pass,
    Fail,
    Error,
    Skip,
}

/// Summary of a test run.
#[derive(Debug)]
pub struct TestRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub skipped: usize,
}

impl TestRunSummary {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64 * 100.0
    }
}

/// Run a single WPT test.
pub fn run_test(test: &WptTestFile, reference_html: Option<&str>) -> TestResult {
    match &test.test_type {
        WptTestType::Reftest {
            relation,
            reference,
        } => match reference_html {
            Some(ref_html) => run_reftest(test, ref_html, relation),
            None => TestResult {
                path: test.path.clone(),
                title: test.title.clone(),
                outcome: TestOutcome::Skip,
                details: format!("Reference file not found: {}", reference),
            },
        },
        WptTestType::TestHarness => run_testharness(test),
        WptTestType::Unknown => TestResult {
            path: test.path.clone(),
            title: test.title.clone(),
            outcome: TestOutcome::Skip,
            details: "Unknown test type".to_string(),
        },
    }
}

fn run_reftest(test: &WptTestFile, reference_html: &str, relation: &ReftestRelation) -> TestResult {
    let result = reftest::compare_layouts(&test.html, reference_html);

    let passed = match relation {
        ReftestRelation::Match => result.passed,
        ReftestRelation::Mismatch => !result.passed,
    };

    let details = if passed {
        "Layout trees match".to_string()
    } else {
        let diffs: Vec<String> = result
            .differences
            .iter()
            .take(5) // Limit output
            .map(|d| {
                format!(
                    "  {}: test={}, ref={}",
                    d.description, d.test_value, d.reference_value
                )
            })
            .collect();
        format!("Differences:\n{}", diffs.join("\n"))
    };

    TestResult {
        path: test.path.clone(),
        title: test.title.clone(),
        outcome: if passed {
            TestOutcome::Pass
        } else {
            TestOutcome::Fail
        },
        details,
    }
}

fn run_testharness(test: &WptTestFile) -> TestResult {
    let assertions = testharness::extract_assertions(&test.html);

    if assertions.is_empty() {
        return TestResult {
            path: test.path.clone(),
            title: test.title.clone(),
            outcome: TestOutcome::Skip,
            details: "No extractable assertions found".to_string(),
        };
    }

    let results = testharness::run_assertions(&test.html, &assertions);

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();

    let details = results
        .iter()
        .filter(|r| !r.passed)
        .take(5)
        .map(|r| {
            format!(
                "  FAIL: {} {:?} expected={}, actual={:?}",
                r.assertion.element_selector,
                r.assertion.property,
                r.assertion.expected_value,
                r.actual_value,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    TestResult {
        path: test.path.clone(),
        title: test.title.clone(),
        outcome: if passed == total {
            TestOutcome::Pass
        } else {
            TestOutcome::Fail
        },
        details: if details.is_empty() {
            format!("{}/{} assertions passed", passed, total)
        } else {
            format!("{}/{} passed:\n{}", passed, total, details)
        },
    }
}

/// Run a batch of tests and produce a summary.
pub fn run_tests(tests: &[(WptTestFile, Option<String>)]) -> (Vec<TestResult>, TestRunSummary) {
    let mut results = Vec::new();
    let mut summary = TestRunSummary {
        total: tests.len(),
        passed: 0,
        failed: 0,
        errors: 0,
        skipped: 0,
    };

    for (test, ref_html) in tests {
        let result = run_test(test, ref_html.as_deref());
        match result.outcome {
            TestOutcome::Pass => summary.passed += 1,
            TestOutcome::Fail => summary.failed += 1,
            TestOutcome::Error => summary.errors += 1,
            TestOutcome::Skip => summary.skipped += 1,
        }
        results.push(result);
    }

    (results, summary)
}

/// Format a test run summary as a string report.
pub fn format_report(results: &[TestResult], summary: &TestRunSummary) -> String {
    let mut report = String::new();
    report.push_str(&format!(
        "WPT Test Results: {} total, {} passed, {} failed, {} errors, {} skipped\n",
        summary.total, summary.passed, summary.failed, summary.errors, summary.skipped
    ));
    report.push_str(&format!("Pass rate: {:.1}%\n\n", summary.pass_rate()));

    // Show failures
    let failures: Vec<&TestResult> = results
        .iter()
        .filter(|r| r.outcome == TestOutcome::Fail)
        .collect();

    if !failures.is_empty() {
        report.push_str("Failures:\n");
        for fail in &failures {
            report.push_str(&format!("  FAIL: {} ({})\n", fail.path, fail.title));
            if !fail.details.is_empty() {
                report.push_str(&format!("    {}\n", fail.details.replace('\n', "\n    ")));
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_reftest_matching() {
        let html = r#"<div style="width: 100px; height: 100px"></div>"#;
        let test = WptTestFile {
            path: "test.html".to_string(),
            html: html.to_string(),
            test_type: WptTestType::Reftest {
                relation: ReftestRelation::Match,
                reference: "ref.html".to_string(),
            },
            reference_path: Some("ref.html".to_string()),
            title: "Test".to_string(),
        };

        let result = run_test(&test, Some(html));
        assert_eq!(result.outcome, TestOutcome::Pass);
    }

    #[test]
    fn test_run_summary() {
        let summary = TestRunSummary {
            total: 100,
            passed: 75,
            failed: 20,
            errors: 2,
            skipped: 3,
        };
        assert!((summary.pass_rate() - 75.0).abs() < 0.1);
    }
}
