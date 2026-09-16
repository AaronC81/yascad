use std::sync::Arc;

use kurbo::{BezPath, PathEl, flatten};
use manifold_csg::CrossSection;
use usvg::{Group, Node, Options, Path, Tree, tiny_skia_path::PathSegment};

use crate::fonts::INCLUDED_FONTS;

pub fn svg_to_cross_section(svg_content: &str, tolerance: f64) -> Result<CrossSection, usvg::Error> {
    // TODO: cache
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    for font in INCLUDED_FONTS {
        fontdb.load_font_data(font.to_vec());
    }

    let options = Options {
        fontdb: Arc::new(fontdb),
        ..Default::default()
    };

    let tree = Tree::from_str(svg_content, &options)?;
    let paths = walk_tree(&tree);
    let cross_sections = paths.into_iter()
        .map(|cx| bezier_path_to_cross_section(cx, tolerance))
        .collect::<Vec<_>>();

    Ok(CrossSection::batch_union(&cross_sections))
}

fn bezier_path_to_cross_section(bezier_path: BezPath, tolerance: f64) -> CrossSection {
    let mut polygons = vec![vec![]];
    flatten(
        bezier_path, tolerance,
        |el| match el {
            PathEl::MoveTo(p) => {
                // Start a new polygon at the given point
                polygons.push(vec![[p.x, p.y]]);
            }

            PathEl::LineTo(p) => {
                // Draw a line on current (last) polygon
                let polygon = polygons.last_mut().unwrap();
                polygon.push([p.x, p.y]);
            },

            PathEl::ClosePath => {},

            _ => unreachable!("unexpected flattened curve element"),
        }
    );

    // Font polygons seem to intersect each other in clever ways to create holes in letters
    // Don't really get it, but from trial-and-error, we need this and not just a batch union of simple polygons!
    CrossSection::from_polygons(&polygons)
}
    

fn walk_tree(tree: &Tree) -> Vec<BezPath> {
    walk_group(tree.root())
}

fn walk_group(group: &Group)-> Vec<BezPath> {
    group.children()
        .into_iter()
        .flat_map(|node| walk_node(node))
        .collect()
}

fn walk_node(node: &Node) -> Vec<BezPath> {
    match node {
        Node::Group(group) => walk_group(group),
        Node::Path(path) => walk_path(path).into_iter().collect(),
        Node::Text(text) => walk_group(text.flattened()),

        // TODO: never supported, warning mechanism?
        Node::Image(_) => todo!("images in SVGs not supported"),
    }
}

fn walk_path(path: &Path) -> Option<BezPath> {
    let mut bezier_path = BezPath::new();

    for segment in path.data().segments() {
        match segment {
            PathSegment::MoveTo(p) => bezier_path.move_to((p.x, p.y)),
            PathSegment::LineTo(p) => bezier_path.line_to((p.x, p.y)),
            PathSegment::QuadTo(p1, p2) => bezier_path.quad_to((p1.x, p1.y), (p2.x, p2.y)),
            PathSegment::CubicTo(p1, p2, p3) => bezier_path.curve_to((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)),
            PathSegment::Close => bezier_path.close_path(),
        }
    }

    // TODO: support strokes?
    if path.fill().is_none() {
        return None;
    }

    // TODO: deal with fill rule?

    Some(bezier_path)
}
