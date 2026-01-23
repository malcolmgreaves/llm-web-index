/// Applies a function to multiple values, tuple literal elements, or `tokio::join!` results.
/// Evaluates to a tuple of transformed values, output order corresponds 1:1 to input order.
#[macro_export]
macro_rules! map {
    // tokio::join! with 1 element
    ($f:expr, tokio::join!($x1:expr $(,)?)) => {{
        let (__r1,) = tokio::join!($x1);
        ($f(__r1),)
    }};

    // tokio::join! with 2 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr $(,)?)) => {{
        let (__r1, __r2) = tokio::join!($x1, $x2);
        ($f(__r1), $f(__r2))
    }};

    // tokio::join! with 3 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr $(,)?)) => {{
        let (__r1, __r2, __r3) = tokio::join!($x1, $x2, $x3);
        ($f(__r1), $f(__r2), $f(__r3))
    }};

    // tokio::join! with 4 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4) = tokio::join!($x1, $x2, $x3, $x4);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4))
    }};

    // tokio::join! with 5 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5) = tokio::join!($x1, $x2, $x3, $x4, $x5);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5))
    }};

    // tokio::join! with 6 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6))
    }};

    // tokio::join! with 7 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7))
    }};

    // tokio::join! with 8 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr, $x8:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7, __r8) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7, $x8);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7), $f(__r8))
    }};

    // tokio::join! with 9 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr, $x8:expr, $x9:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7, __r8, __r9) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7, $x8, $x9);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7), $f(__r8), $f(__r9))
    }};

    // tokio::join! with 10 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr, $x8:expr, $x9:expr, $x10:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7, __r8, __r9, __r10) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7, $x8, $x9, $x10);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7), $f(__r8), $f(__r9), $f(__r10))
    }};

    // tokio::join! with 11 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr, $x8:expr, $x9:expr, $x10:expr, $x11:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7, __r8, __r9, __r10, __r11) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7, $x8, $x9, $x10, $x11);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7), $f(__r8), $f(__r9), $f(__r10), $f(__r11))
    }};

    // tokio::join! with 12 elements
    ($f:expr, tokio::join!($x1:expr, $x2:expr, $x3:expr, $x4:expr, $x5:expr, $x6:expr, $x7:expr, $x8:expr, $x9:expr, $x10:expr, $x11:expr, $x12:expr $(,)?)) => {{
        let (__r1, __r2, __r3, __r4, __r5, __r6, __r7, __r8, __r9, __r10, __r11, __r12) = tokio::join!($x1, $x2, $x3, $x4, $x5, $x6, $x7, $x8, $x9, $x10, $x11, $x12);
        ($f(__r1), $f(__r2), $f(__r3), $f(__r4), $f(__r5), $f(__r6), $f(__r7), $f(__r8), $f(__r9), $f(__r10), $f(__r11), $f(__r12))
    }};

    // Tuple literal input: map!(f, (a, b, c)) -> (f(a), f(b), f(c))
    ($f:expr, ($($x:expr),+ $(,)?)) => {
        ($($f($x)),+)
    };

    // Individual arguments: map!(f, a, b, c) -> (f(a), f(b), f(c))
    ($f:expr, $($x:expr),+ $(,)?) => {
        ($($f($x)),+)
    };
}
