(module
   (func $add (param $lhs i64) (param $rhs i64) (result i64)
       local.get $lhs
       local.get $rhs
       i64.add)
   (export "naked:adder/add@0.1.0#add" (func $add))
)
