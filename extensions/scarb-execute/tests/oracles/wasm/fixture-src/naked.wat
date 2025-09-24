(module
   (func $add (param $lhs i64) (param $rhs i64) (result i64)
       local.get $lhs
       local.get $rhs
       i64.add)
   (func $f1 (param $n i32) (result i32)
          local.get $n
          i32.const 1
          i32.add)
   (func $f1000 (param $n i32) (result i32)
       local.get $n
       i32.const 1000
       i32.add)
   (export "naked:adder/add@0.1.0#add" (func $add))
   (export "naked:adder/add@0.1.0#f" (func $f1))
   (export "naked:adder/ambiguous@0.1.0#f" (func $f1000))
)
