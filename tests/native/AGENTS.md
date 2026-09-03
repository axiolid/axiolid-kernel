# Native consumer tests

`cmake-consumer/` must exercise only the installed/source C API through the stable `Axiolid::axiolid` target. `test_native_packaging.py` owns archive/path/binary-identity mutation tests. Keep fixtures platform-neutral; platform-specific setup belongs in `.github/workflows/native.yml`.
