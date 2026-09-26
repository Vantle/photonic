pub struct Resource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub mime: &'static str,
    pub text: &'static str,
}

const PRIMER: &str = include_str!("primer.md");
const LIBRARY: &str = include_str!("../library/README.md");

pub fn catalog() -> [Resource; 2] {
    [
        Resource {
            uri: "photonic://primer",
            name: "primer",
            title: "Photonic for agents",
            description: "The grammar, what programs mean, and how to ask Spectrum about them.",
            mime: "text/markdown",
            text: PRIMER,
        },
        Resource {
            uri: "photonic://library",
            name: "library",
            title: "The standard library",
            description: "Every package, its requests and answers, and the conventions they share.",
            mime: "text/markdown",
            text: LIBRARY,
        },
    ]
}

pub fn find(uri: &str) -> Option<Resource> {
    catalog().into_iter().find(|resource| resource.uri == uri)
}
