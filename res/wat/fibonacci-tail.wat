(module
    (func $fib_tail_recursive (param $n i64) (param $a i64) (param $b i64) (result i64)
        (if (i64.eqz (local.get $n))
            (then
                (return (local.get $a))
            )
        )
        (return_call $fib_tail_recursive
            (i64.sub (local.get $n) (i64.const 1))
            (local.get $b)
            (i64.add (local.get $a) (local.get $b))
        )
    )

    (func (export "run") (param $n i64) (result i64)
        (return_call $fib_tail_recursive (local.get $n) (i64.const 0) (i64.const 1))
    )
)
