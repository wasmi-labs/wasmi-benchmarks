(module
    (func $fib (param $N i64) (param $a i64) (param $b i64) (result i64)
        (if (i64.eqz (local.get $N))
            (then
                (return (local.get $a))
            )
        )
        (if (i64.eq (local.get $N) (i64.const 1))
            (then
                (return (local.get $b))
            )
        )
        (return_call $fib
            (i64.sub (local.get $N) (i64.const 1))
            (local.get $b)
            (i64.add (local.get $a) (local.get $b))
        )
    )

    (func (export "run") (param $N i64) (result i64)
        (return_call $fib (local.get $N) (i64.const 0) (i64.const 1))
    )
)
