//! Parse WPT test files.
//!
//! WPT tests come as HTML files with metadata in <link> and <meta> tags.

/// Metadata extracted from a WPT test file.
#[derive(Debug, Clone)]
pub struct WptTestFile {
    pub path: String,
    pub html: String,
    pub test_type: WptTestType,
    pub reference_path: Option<String>,
    pub title: String,
}

/// The type of WPT test.
#[derive(Debug, Clone, PartialEq)]
pub enum WptTestType {
    /// A reftest: compare layout against a reference file.
    Reftest {
        relation: ReftestRelation,
        reference: String,
    },
    /// A testharness.js test with JavaScript assertions.
    TestHarness,
    /// Unknown test type.
    Unknown,
}

/// Whether a reftest should match or not match the reference.
#[derive(Debug, Clone, PartialEq)]
pub enum ReftestRelation {
    Match,
    Mismatch,
}

/// Parse a WPT HTML test file and extract its metadata.
pub fn parse_wpt_test(path: &str, html: &str) -> WptTestFile {
    let test_type = detect_test_type(html);
    let title = extract_title(html);

    WptTestFile {
        path: path.to_string(),
        html: html.to_string(),
        test_type,
        reference_path: extract_reference_path(html),
        title,
    }
}

fn detect_test_type(html: &str) -> WptTestType {
    // Check for reftest link
    if let Some(ref_info) = extract_reftest_info(html) {
        return ref_info;
    }

    // Check for testharness.js
    if html.contains("testharness.js") || html.contains("testharnessreport.js") {
        return WptTestType::TestHarness;
    }

    WptTestType::Unknown
}

fn extract_reftest_info(html: &str) -> Option<WptTestType> {
    // Look for <link rel="match" href="..."> or <link rel="mismatch" href="...">
    let _lower = html.to_lowercase();

    for line in html.lines() {
        let lower_line = line.to_lowercase();
        if lower_line.contains("<link")
            && (lower_line.contains("rel=\"match\"") || lower_line.contains("rel='match'"))
        {
            if let Some(href) = extract_href(line) {
                return Some(WptTestType::Reftest {
                    relation: ReftestRelation::Match,
                    reference: href,
                });
            }
        }
        if lower_line.contains("<link")
            && (lower_line.contains("rel=\"mismatch\"") || lower_line.contains("rel='mismatch'"))
        {
            if let Some(href) = extract_href(line) {
                return Some(WptTestType::Reftest {
                    relation: ReftestRelation::Mismatch,
                    reference: href,
                });
            }
        }
    }

    None
}

fn extract_href(tag: &str) -> Option<String> {
    if let Some(pos) = tag.find("href=\"") {
        let start = pos + 6;
        if let Some(end) = tag[start..].find('"') {
            return Some(tag[start..start + end].to_string());
        }
    }
    if let Some(pos) = tag.find("href='") {
        let start = pos + 6;
        if let Some(end) = tag[start..].find('\'') {
            return Some(tag[start..start + end].to_string());
        }
    }
    None
}

fn extract_reference_path(html: &str) -> Option<String> {
    if let Some(WptTestType::Reftest { reference, .. }) = extract_reftest_info(html) {
        Some(reference)
    } else {
        None
    }
}

fn extract_title(html: &str) -> String {
    if let Some(start) = html.find("<title>") {
        let content_start = start + 7;
        if let Some(end) = html[content_start..].find("</title>") {
            return html[content_start..content_start + end].trim().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reftest() {
        let html = r#"
            <title>CSS Test: Block Width</title>
            <link rel="match" href="reference/green-box-ref.html">
            <div style="width: 100px; height: 100px; background: green"></div>
        "#;

        let test = parse_wpt_test("css/CSS2/box/width-001.html", html);
        assert_eq!(test.title, "CSS Test: Block Width");
        assert!(matches!(
            test.test_type,
            WptTestType::Reftest {
                relation: ReftestRelation::Match,
                ..
            }
        ));
        assert_eq!(
            test.reference_path,
            Some("reference/green-box-ref.html".to_string())
        );
    }

    #[test]
    fn test_parse_testharness() {
        let html = r#"
            <script src="/resources/testharness.js"></script>
            <script src="/resources/testharnessreport.js"></script>
            <div id="target" style="width: 100px"></div>
            <script>
                test(function() {
                    assert_equals(document.getElementById("target").getBoundingClientRect().width, 100);
                });
            </script>
        "#;

        let test = parse_wpt_test("css/CSS2/box/width-002.html", html);
        assert_eq!(test.test_type, WptTestType::TestHarness);
    }
}
