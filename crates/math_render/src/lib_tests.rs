use super::*;

#[test]
fn renders_inline_math() {
    let svg = render_math_to_svg(r"O(n \log n)", false, "#e0e0e0", 16.0).expect("should render");
    assert!(svg.starts_with("<svg"), "output should be an SVG document");
    assert!(svg.contains("<path"), "glyphs should be embedded as paths");
}

#[test]
fn renders_display_math() {
    let svg = render_math_to_svg(r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}", true, "#000000", 16.0)
        .expect("should render");
    assert!(svg.starts_with("<svg"));
}

#[test]
fn parse_error_on_invalid_latex() {
    let result = render_math_to_svg(r"\frac{unclosed", true, "#000000", 16.0);
    assert!(matches!(result, Err(MathRenderError::Parse(_))));
}

#[test]
fn error_on_invalid_color() {
    let result = render_math_to_svg("x", false, "not-a-color", 16.0);
    assert!(matches!(result, Err(MathRenderError::InvalidColor(_))));
}

#[test]
fn reports_plausible_baseline_fraction() {
    // A simple symbol with a descender-free body: baseline near the bottom.
    let r = render_math("x", false, "#000000", 20.0).expect("should render");
    assert!(r.baseline_fraction > 0.5 && r.baseline_fraction <= 1.0,
        "baseline for 'x' should be in the lower half, got {}", r.baseline_fraction);

    // A fraction extends well below the baseline (large depth), pulling the
    // baseline fraction up toward the middle.
    let frac = render_math(r"\frac{a}{b}", false, "#000000", 20.0).expect("should render");
    assert!(frac.baseline_fraction > 0.2 && frac.baseline_fraction < r.baseline_fraction,
        "fraction baseline {} should sit higher than plain 'x' {}", frac.baseline_fraction, r.baseline_fraction);
    assert!(frac.svg.starts_with("<svg"));
}
