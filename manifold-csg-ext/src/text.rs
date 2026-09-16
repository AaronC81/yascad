use manifold_csg::CrossSection;

use crate::svg_to_cross_section;

// TODO: all the other parameters ever
pub fn text_to_cross_section(text: &str) -> CrossSection {
    // TODO: SVG-escape text
    let svg = format!(r#"
<svg>
    <text x="0" y="0" font-family="sans-serif">{text}</text>
</svg>
    "#);

    svg_to_cross_section(&svg, 0.25)
        .expect("text rendering failed")
}
