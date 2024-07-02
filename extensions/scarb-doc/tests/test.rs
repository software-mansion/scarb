use assert_fs::TempDir;
use indoc::indoc;
use std::iter::zip;

use scarb_metadata::MetadataCommand;
use scarb_test_support::project_builder::ProjectBuilder;

use scarb_doc::compilation::get_project_config;
use scarb_doc::generate_language_elements_tree_for_package;
use scarb_doc::types::ItemData;

#[test]
fn integration_test() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
        //! Fibonacci sequence calculator


        /// Main function that calculates the 16th Fibonacci number
        fn main() -> u32 {
            fib(16)
        }

        /// use into_trait
        use core::traits::Into as into_trait;
        use core::traits::TryInto;
        
        /// FOO constant with value 42
        const FOO: u32 = 42;

        /// Calculate the nth Fibonacci number
        ///
        /// # Arguments
        /// * `n` - The index of the Fibonacci number to calculate
        /// 
        fn fib(mut n: u32) -> u32 {
            let mut a: u32 = 0;
            let mut b: u32 = 1;
            while n != 0 {
                n = n - 1;
                let temp = b;
                b = a + b;
                a = temp;
            };
            a
        }

        /// Pair type alias for a tuple of two u32 values
        type Pair = (u32, u32);

        /// Color enum with Red, Green, and Blue variants
        enum Color {
            /// Red color
            Red: (),
            /// Green color
            Green: (),
            /// Blue color
            Blue: (),
        }

        /// Shape trait for objects that have an area
        trait Shape<T> {
            /// Constant for the shape type
            const SHAPE_CONST = "SHAPE";
        
            /// Type alias for a pair of shapes
            type ShapePair<T> = (Shape<T>, Shape<T>);
        
            /// Calculate the area of the shape
            fn area(self: T) -> u32;
        }

        /// Circle struct with radius field
        #[derive(Drop, Serde, PartialEq)]
        struct Circle {
            /// Radius of the circle
            radius: u32,
        }

        /// Implementation of the Shape trait for Circle
        impl CircleShape of Shape<Circle> {
            /// Type alias for a pair of circles
            type ShapePair<Circle> = (Circle, Circle);
        
            /// Shape constant
            const SHAPE_CONST = "xyz";

            /// Implementation of the area method for Circle
            fn area(self: Circle) -> u32 {
                3 * self.radius * self.radius
            }
        }

        /// Tests module
        mod tests {
            /// Imported fib function from the parent module
            use super::fib as fib_function;

            /// Really
            #[test]
            /// works.
            fn it_works() {
                assert(fib_function(16) == 987, 'it works!');
            }
        }
        "#})
        .build(&t);

    let metadata = MetadataCommand::new()
        .current_dir(t.path())
        .exec()
        .expect("Failed to obtain metadata");
    let package_metadata = metadata
        .packages
        .iter()
        .find(|pkg| pkg.id == metadata.workspace.members[0])
        .unwrap();

    let project_config = get_project_config(&metadata, package_metadata);

    let root_module =
        generate_language_elements_tree_for_package(package_metadata.name.clone(), project_config)
            .expect("Failed to generate language elements tree")
            .root_module;

    assert_eq!(
        root_module.item_data,
        ItemData {
            name: "hello_world".to_string(),
            doc: None, // FIXME: compiler doesn't support fetching root crate doc
            signature: None,
            full_path: "hello_world".to_string(),
        }
    );

    let tests_submodule = &root_module.submodules[0];
    assert_eq!(
        tests_submodule.item_data,
        ItemData {
            name: "tests".to_string(),
            doc: Some("Tests module".to_string()),
            signature: None,
            full_path: "hello_world::tests".to_string(),
        }
    );

    let free_func_in_submodule = &tests_submodule.free_functions[0];
    assert_eq!(
        free_func_in_submodule.item_data,
        ItemData {
            name: "it_works".to_string(),
            doc: Some("Really\nworks.".to_string()),
            signature: Some("fn it_works()".to_string()),
            full_path: "hello_world::tests::it_works".to_string(),
        }
    );

    let constant = &root_module.constants[0];
    assert_eq!(
        constant.item_data,
        ItemData {
            name: "FOO".to_string(),
            doc: Some("FOO constant with value 42".to_string()),
            signature: Some("const FOO: u32 = 42;".to_string()),
            full_path: "hello_world::FOO".to_string(),
        }
    );

    let main_function = &root_module.free_functions[0];
    assert_eq!(
        main_function.item_data,
        ItemData {
            name: "main".to_string(),
            doc: Some("Fibonacci sequence calculator\nMain function that calculates the 16th Fibonacci number".to_string()),
            signature: Some("fn main() -> u32".to_string()),
            full_path: "hello_world::main".to_string(),
        }
    );

    let fib_function = &root_module.free_functions[1];
    assert_eq!(
        fib_function.item_data,
        ItemData {
            name: "fib".to_string(),
            doc: Some("Calculate the nth Fibonacci number\n\n# Arguments\n* `n` - The index of the Fibonacci number to calculate\n".to_string()),
            signature: Some("fn fib(mut n: u32) -> u32".to_string()),
            full_path: "hello_world::fib".to_string(),
        }
    );

    let circle_struct = &root_module.structs[0];

    assert_eq!(
        circle_struct.item_data,
        ItemData {
            name: "Circle".to_string(),
            doc: Some("Circle struct with radius field".to_string()),
            signature: None,
            full_path: "hello_world::Circle".to_string(),
        }
    );

    let radius_field = &circle_struct.members[0];
    assert_eq!(
        radius_field.item_data,
        ItemData {
            name: "radius".to_string(),
            doc: Some("Radius of the circle".to_string()),
            signature: None,
            full_path: "hello_world::Circle::radius".to_string(),
        }
    );

    let color_enum = &root_module.enums[0];
    assert_eq!(
        color_enum.item_data,
        ItemData {
            name: "Color".to_string(),
            doc: Some("Color enum with Red, Green, and Blue variants".to_string()),
            signature: None,
            full_path: "hello_world::Color".to_string(),
        }
    );

    for (variant, color_name) in zip(&color_enum.variants, ["Red", "Green", "Blue"]) {
        assert_eq!(
            variant.item_data,
            ItemData {
                name: color_name.to_string(),
                doc: Some(format!("{color_name} color")),
                signature: None,
                full_path: format!("hello_world::Color::{color_name}"),
            }
        );
    }

    let pair_type_alias = &root_module.type_aliases[0];
    assert_eq!(
        pair_type_alias.item_data,
        ItemData {
            name: "Pair".to_string(),
            doc: Some("Pair type alias for a tuple of two u32 values".to_string()),
            signature: Some("type Pair = (u32, u32);".to_string()),
            full_path: "hello_world::Pair".to_string(),
        }
    );

    let shape_trait = &root_module.traits[0];
    assert_eq!(
        shape_trait.item_data,
        ItemData {
            name: "Shape".to_string(),
            doc: Some("Shape trait for objects that have an area".to_string()),
            signature: Some(" trait Shape<T>".to_string()), // FIXME: trim whitespaces in compiler
            full_path: "hello_world::Shape".to_string(),
        }
    );

    let trait_constant = &shape_trait.trait_constants[0];
    assert_eq!(
        trait_constant.item_data,
        ItemData {
            name: "SHAPE_CONST".to_string(),
            doc: Some("Constant for the shape type".to_string()),
            signature: None, // FIXME: compiler returns empty string here
            full_path: "Shape::SHAPE_CONST".to_string(), // FIXME: incorrect path
        }
    );

    let trait_type = &shape_trait.trait_types[0];
    assert_eq!(
        trait_type.item_data,
        ItemData {
            name: "ShapePair".to_string(),
            doc: Some("Type alias for a pair of shapes".to_string()),
            signature: None, // FIXME: compiler returns empty string here
            full_path: "Shape::ShapePair".to_string(), // FIXME: incorrect path
        }
    );

    let trait_function = &shape_trait.trait_functions[0];
    assert_eq!(
        trait_function.item_data,
        ItemData {
            name: "area".to_string(),
            doc: Some("Calculate the area of the shape".to_string()),
            signature: Some("fn area(self: T) -> u32;".to_string()),
            full_path: "Shape::area".to_string(), // FIXME: incorrect path
        }
    );

    let circle_shape_impl = &root_module.impls[0];
    assert_eq!(
        circle_shape_impl.item_data,
        ItemData {
            name: "CircleShape".to_string(),
            doc: Some("Implementation of the Shape trait for Circle".to_string()),
            signature: Some(" impl CircleShape  of Shape<Circle>".to_string()), // FIXME: trim whitespaces in compiler
            full_path: "hello_world::CircleShape".to_string(),
        }
    );

    let impl_func = &circle_shape_impl.impl_functions[0];
    assert_eq!(
        impl_func.item_data,
        ItemData {
            name: "area".to_string(),
            doc: Some("Implementation of the area method for Circle".to_string()),
            signature: Some("fn area(self: Circle) -> u32".to_string()),
            full_path: "hello_world::CircleShape::area".to_string(),
        }
    );

    let impl_const = &circle_shape_impl.impl_constants[0];
    assert_eq!(
        impl_const.item_data,
        ItemData {
            name: "SHAPE_CONST".to_string(),
            doc: Some("Shape constant".to_string()),
            signature: Some("const SHAPE_CONST = \"xyz\";".to_string()),
            full_path: "CircleShape::SHAPE_CONST".to_string(), // FIXME: incorrect path
        }
    );

    let impl_type = &circle_shape_impl.impl_types[0];
    assert_eq!(
        impl_type.item_data,
        ItemData {
            name: "ShapePair".to_string(),
            doc: Some("Type alias for a pair of circles".to_string()),
            signature: Some("type ShapePair<Circle> = (Circle, Circle);".to_string()),
            full_path: "CircleShape::ShapePair".to_string(), // FIXME: incorrect path
        }
    );

    assert_eq!(
        root_module.impls.len(),
        4,
        "Traits from derive are not present"
    );
}
