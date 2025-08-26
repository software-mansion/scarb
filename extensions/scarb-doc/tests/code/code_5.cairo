/// Public function
pub fn public_function() {
    println!("public_function");
}

/// Public crate function
pub(crate) fn public_crate_function() {
    println!("public_function");
}

/// Private function
fn private_function() {
    println!("private_function");
}

/// Public struct
pub struct PublicStructure {
    /// Public struct field
    pub public_field: felt252,
    /// Public crate struct field
    pub(crate) public_crate_field: felt252,
    /// Private struct field
    private_field: felt252
}

/// Public crate struct
pub(crate) struct PublicCrateStructure {
    /// Public struct field
    pub public_field: felt252,
    /// Public crate struct field
    pub(crate) public_crate_field: felt252,
    /// Private struct field
    private_field: felt252
}

/// Private struct
struct PrivateStructure {
    /// Public struct field
    pub public_field: felt252,
    /// Public crate struct field
    pub(crate) public_crate_field: felt252,
    /// Private struct field
    private_field: felt252
}

/// Public enum
pub enum PublicEnum {
    /// Public enum variant
    PUBLIC_VARIANT
}

/// Public crate enum
pub(crate) enum PublicCrateEnum {
    /// Public enum variant
    PUBLIC_VARIANT
}

/// Private enum
enum PrivateEnum {
    /// Private enum variant
    PRIVATE_VARIANT,
}

/// Public macro declaration
pub macro public_macro_declaration {
	[$x:ident  ] => {
		println!("{}", $x)
	};
}

/// Private macro declaration
macro private_macro_declaration {
	[$x:ident  ] => {
		println!("{}", $x)
	};
}

/// Public parent module
pub mod PublicParent {
    /// Parent public function
    pub fn parent_public_function() {
        println!("parent_public_function");
    }

    /// Parent public crate function
    pub(crate) fn parent_public_crate_function() {
        println!("parent_public_crate_function");
    }

    /// Parent private function
    fn parent_private_function() {
        println!("parent_private_function");
    }

    /// Parent public struct
    pub struct PublicParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public crate struct
    pub(crate) struct PublicCrateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent private struct
    struct PrivateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public enum
    pub enum PublicParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent public crate enum
    pub(crate) enum PublicCrateParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent private enum
    enum PrivateParentEnum {
        /// Private enum variant
        PRIVATE_VARIANT,
    }

    /// Public child module
    pub mod PublicChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Public crate child module
    pub(crate) mod PublicCrateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Private child module
    mod PrivateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }
}

/// Public crate parent module
pub(crate) mod PublicCrateParent {
    /// Parent public function
    pub fn parent_public_function() {
        println!("parent_public_function");
    }

    /// Parent public crate function
    pub(crate) fn parent_public_crate_function() {
        println!("parent_public_crate_function");
    }

    /// Parent private function
    fn parent_private_function() {
        println!("parent_private_function");
    }

    /// Parent public struct
    pub struct PublicParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public struct
    pub(crate) struct PublicCrateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent private struct
    struct PrivateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public enum
    pub enum PublicParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent public crate enum
    pub(crate) enum PublicCrateParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent private enum
    enum PrivateParentEnum {
        /// Private enum variant
        PRIVATE_VARIANT,
    }

    /// Public child module
    pub mod PublicChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Public crate child module
    pub(crate) mod PublicCrateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Private child module
    mod PrivateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }
}

/// Private parent module
mod PrivateParent {
    /// Parent public function
    pub fn parent_public_function() {
        println!("parent_public_function");
    }

    /// Parent public crate function
    pub(crate) fn parent_public_crate_function() {
        println!("parent_public_crate_function");
    }

    /// Parent private function
    fn parent_private_function() {
        println!("parent_private_function");
    }

    /// Parent public struct
    pub struct PublicParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public struct
    pub(crate) struct PublicCrateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent private struct
    struct PrivateParentStructure {
        /// Public struct field
        pub public_field: felt252,
        /// Public crate struct field
        pub(crate) public_crate_field: felt252,
        /// Private struct field
        private_field: felt252
    }

    /// Parent public enum
    pub enum PublicParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent public crate enum
    pub(crate) enum PublicCrateParentEnum {
        /// Public enum variant
        PUBLIC_VARIANT
    }

    /// Parent private enum
    enum PrivateParentEnum {
        /// Private enum variant
        PRIVATE_VARIANT,
    }

    /// Public child module
    pub mod PublicChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Public crate child module
    pub(crate) mod PublicCrateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }

    /// Private child module
    mod PrivateChild {
        /// Child public function
        pub fn child_public_function() {
            println!("child_public_function");
        }

        /// Child crate public function
        pub(crate) fn child_public_crate_function() {
            println!("child_public_crate_function");
        }

        /// Child private function
        fn child_private_function() {
            println!("child_private_function");
        }

        /// Child public struct
        pub struct PublicChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public crate struct
        pub(crate) struct PublicCrateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child private struct
        struct PrivateChildStructure {
            /// Public struct field
            pub public_field: felt252,
            /// Public crate struct field
            pub(crate) public_crate_field: felt252,
            /// Private struct field
            private_field: felt252
        }

        /// Child public enum
        pub enum PublicChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child public crate enum
        pub(crate) enum PublicCrateChildEnum {
            /// Public enum variant
            PUBLIC_VARIANT
        }

        /// Child private enum
        enum PrivateChildEnum {
            /// Private enum variant
            PRIVATE_VARIANT,
        }
    }
}

fn main() {
    println!("hello_world");
}

