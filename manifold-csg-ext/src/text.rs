use manifold_csg::CrossSection;

use crate::svg_to_cross_section;

// TODO: all the other parameters ever
pub fn text_to_cross_section(text: &str) -> CrossSection {
    let text = xml_escape(text);

    let svg = format!(r#"
<svg>
    <text x="0" y="0" font-family="Liberation Sans">{text}</text>
</svg>
    "#);

    svg_to_cross_section(&svg, 0.25)
        .expect("text rendering failed")
}

fn xml_escape(input: &str) -> String {
    // https://stackoverflow.com/a/1091953/2626000
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
