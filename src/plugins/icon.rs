use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use maplit::{hashmap, hashset};
use xmltree::{Element, XMLNode};

#[derive(Clone, Debug)]
pub struct SVGIcon {
    pub root: Element,
}

impl FromStr for SVGIcon {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut root =
            Element::parse(s.as_bytes()).map_err(|e| format!("XML Parse Error: {}", e))?;

        Self::sanitize_tree(&mut root);

        Ok(SVGIcon { root })
    }
}

impl std::fmt::Display for SVGIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Vec::new();

        self.root.write(&mut buf).unwrap();

        write!(f, "{}", String::from_utf8(buf).unwrap_or_default())
    }
}

impl SVGIcon {
    pub fn with_size(&mut self, w: u8, h: u8) -> &mut Self {
        self.root.attributes.insert("width".into(), w.to_string());
        self.root.attributes.insert("height".into(), h.to_string());

        self
    }

    fn sanitize_tree(root: &mut Element) {
        let tags = hashset![
            "svg",
            "g",
            "defs",
            "path",
            "circle",
            "rect",
            "line",
            "polyline",
            "polygon",
            "ellipse",
            "linearGradient",
            "radialGradient",
            "stop",
            "clipPath",
            "mask",
            "filter",
            "feGaussianBlur",
            "feOffset",
            "feBlend",
            "feColorMatrix",
            "feComposite"
        ];
        let globals = hashset![
            "id",
            "class",
            "style",
            "fill",
            "fill-opacity",
            "fill-rule",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-dasharray",
            "stroke-opacity",
            "transform",
            "opacity",
            "clip-path",
            "mask",
            "filter"
        ];
        let specific = hashmap! {
            "svg" => hashset!["viewBox", "width", "height", "xmlns", "x", "y"],
            "path" => hashset!["d"],
            "circle" => hashset!["cx", "cy", "r"],
            "rect" => hashset!["x", "y", "width", "height", "rx", "ry"],
            "line" => hashset!["x1", "y1", "x2", "y2"],
            "ellipse" => hashset!["cx", "cy", "rx", "ry"],
            "polygon" => hashset!["points"],
            "polyline" => hashset!["points"],
            "linearGradient" => hashset!["gradientUnits", "gradientTransform", "spreadMethod", "x1", "y1", "x2", "y2", "cx", "cy", "r", "fx", "fy"],
            "radialGradient" => hashset!["gradientUnits", "gradientTransform", "spreadMethod", "x1", "y1", "x2", "y2", "cx", "cy", "r", "fx", "fy"],
            "stop" => hashset!["offset", "stop-color", "stop-opacity"],
            "filter" => hashset!["filterUnits", "x", "y", "width", "height"],
            "feGaussianBlur" => hashset!["in", "stdDeviation", "result"],
            "feOffset" => hashset!["in", "dx", "dy", "result"]
        };

        fn walk(
            el: &mut Element,
            tags: &HashSet<&str>,
            globals: &HashSet<&str>,
            specific: &HashMap<&str, HashSet<&str>>,
        ) {
            el.attributes.retain(|k, _| {
                globals.contains(k.as_str())
                    || specific
                        .get(el.name.as_str())
                        .is_some_and(|set| set.contains(k.as_str()))
            });

            el.children.retain_mut(|node| {
                if let XMLNode::Element(child) = node
                    && tags.contains(child.name.as_str())
                {
                    walk(child, tags, globals, specific);
                    return true;
                }
                false
            });
        }

        walk(root, &tags, &globals, &specific);
    }
}
