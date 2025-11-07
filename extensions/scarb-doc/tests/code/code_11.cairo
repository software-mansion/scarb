mod not_public {
    /// this might get documented as a result of changes in compiler at some point,
    /// if it does - the test will fail, this is intended
	pub macro not_a_public_macro {
		($name:ident) => {
			expose! {
				pub fn $name() -> felt252 { 10 }
			}
		};
	}

	/// this should never be documented
	fn not_a_public_fn() -> felt252 {10}
}

pub macro define_function {
	($name:ident) => {
		expose! {
			pub fn $name() -> felt252 { 10 }
		}
	};
}

define_function!(my_macro_defined_function);

pub macro define_trait {
	($name:ident) => {
		expose! {
			#[doc(group: "exposed can be a part of a group")]
			pub trait $name<T> {
				fn do_stuff(self: T) -> u32;
			}
		}
	};
}
define_trait!(ShapeShifters);

mod secret_mod {
    /// this might get documented as a result of changes in compiler at some point,
    /// if it does - the test will fail, this is intended
	pub fn secret_fn() -> felt252 {10}
}

pub macro nested_module_macro {
	(
		$name:ident
	) => {
		expose! {
			/// This is a doc comment we should see in generated docs
			pub mod $name {
			        // compiler accepts the syntax but does not resolve the pub use items in macro declaration
			        // properly yet, this will likely change in the future
			        // and may cause the test to fail - this is intended
					pub use hello::secret_mod::secret_fn;
					pub use hello::not_public::not_a_public_macro;

					#[doc(hidden)]
					pub struct InvisibleStruct {}

					/// Inner documentation is correctly retrieved
					pub struct VisibleStruct {
						/// for members as well
						pub works: felt252,
						properly: felt252,
					}

				 }
		}
	};
}

nested_module_macro!(regina);

fn main() -> felt252 {
	my_macro_defined_function()

}

/// the outer struct
pub struct OuterStruct {
	pub work: felt252,
	properly: felt252,
}
