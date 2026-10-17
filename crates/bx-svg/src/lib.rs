//! `icon!("path/to/icon.svg")` -> `(&'static [[f32; 2]], &'static [u32])`: vertices and triangle
//! indices of the icon's stroke, tessellated at compile time. Coordinates are normalised to the
//! SVG's size: x and y both in 0..1, y pointing down. The path is relative to the calling crate.
// stroke only (what lucide uses): no fills, no transforms/groups/opacity, one color.
use lyon_tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers, math::point,
    path::Path,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};
use usvg::tiny_skia_path::PathSegment::*;

#[proc_macro]
pub fn icon(input: TokenStream) -> TokenStream {
    let rel = parse_macro_input!(input as LitStr).value();
    // a bare name ("heart") is a lucide icon from `npm install lucide-static`
    let rel = if rel.contains('/') {
        rel
    } else {
        format!("node_modules/lucide-static/icons/{rel}.svg")
    };
    // relative to the calling crate or any directory above it (node_modules sits at the workspace
    // root, shared by every app)
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let path = manifest
        .ancestors()
        .map(|d| d.join(&rel))
        .find(|p| p.exists())
        .unwrap_or_else(|| manifest.join(&rel));
    let svg = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (w, h) = (tree.size().width(), tree.size().height());

    let mut mesh = VertexBuffers::<[f32; 2], u32>::new();
    for node in tree.root().children() {
        let usvg::Node::Path(p) = node else { continue };
        let Some(s) = p.stroke() else { continue };
        let mut b = Path::builder();
        let mut open = false;
        for seg in p.data().segments() {
            let pt = |p: usvg::tiny_skia_path::Point| point(p.x, p.y);
            match seg {
                MoveTo(a) => {
                    if open {
                        b.end(false);
                    }
                    b.begin(pt(a));
                    open = true;
                }
                LineTo(a) => _ = b.line_to(pt(a)),
                QuadTo(c, a) => _ = b.quadratic_bezier_to(pt(c), pt(a)),
                CubicTo(c1, c2, a) => _ = b.cubic_bezier_to(pt(c1), pt(c2), pt(a)),
                Close => {
                    b.end(true);
                    open = false;
                }
            }
        }
        if open {
            b.end(false);
        }
        let cap = match s.linecap() {
            usvg::LineCap::Butt => lyon_tessellation::LineCap::Butt,
            usvg::LineCap::Round => lyon_tessellation::LineCap::Round,
            usvg::LineCap::Square => lyon_tessellation::LineCap::Square,
        };
        let join = match s.linejoin() {
            usvg::LineJoin::Round => lyon_tessellation::LineJoin::Round,
            usvg::LineJoin::Bevel => lyon_tessellation::LineJoin::Bevel,
            _ => lyon_tessellation::LineJoin::Miter,
        };
        StrokeTessellator::new()
            .tessellate_path(
                &b.build(),
                &StrokeOptions::tolerance(0.01)
                    .with_line_width(s.width().get())
                    .with_line_cap(cap)
                    .with_line_join(join)
                    .with_miter_limit(s.miterlimit().get()),
                &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| {
                    let p = v.position();
                    [p.x / w, p.y / h]
                }),
            )
            .unwrap();
    }

    let path = path.to_str().unwrap();
    let v = mesh.vertices.iter().map(|[x, y]| quote!([#x, #y]));
    let i = &mesh.indices;

    quote! {{
        const _: &[u8] = ::std::include_bytes!(#path);
        (&[#(#v),*] as &[[f32; 2]], &[#(#i),*] as &[u32])
    }}
    .into()
}
