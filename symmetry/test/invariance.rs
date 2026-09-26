use super::support::{Shape, closure, shuffle, structure};
use random::Generator;

#[test]
fn relabeling() {
    let mut generator = Generator::new(3);
    for round in 0..120 {
        let shape = Shape {
            atom: 4 + generator.below(40),
            rule: 1 + generator.below(30),
            depth: generator.below(3),
        };
        let base = structure(&mut generator, &shape);
        let structure = if round % 2 == 0 {
            closure(&mut generator, &base)
        } else {
            base
        };
        let symmetry = structure
            .symmetry(1_000_000)
            .expect("the search fits its budget");
        let form = &symmetry.form;
        for _ in 0..4 {
            let (image, _) = shuffle(&mut generator, &structure);
            let other = image
                .symmetry(1_000_000)
                .expect("the search fits its budget");
            assert_eq!(other.key, symmetry.key);
            assert_eq!(other.size, symmetry.size);
            assert_eq!(&other.form, form);
            let map = symmetry
                .isomorphism(&other)
                .expect("a relabeling is an isomorphism");
            assert_eq!(super::support::rename(&structure, |atom| map[&atom]), image);
        }
        for permutation in super::support::generator(&symmetry) {
            assert_eq!(
                super::support::rename(&structure, |atom| permutation.image(atom)),
                structure
            );
        }
    }
}
